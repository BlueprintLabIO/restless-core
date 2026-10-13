import { createInterface } from 'node:readline';
import { randomUUID } from 'node:crypto';
import { Model, upstream } from './upstream.mjs';

export const ENGINE_VERSION = '19.0.51';
// Agent range reads, filters, sorts and record edits.
const MAX_RANGE_CELLS = 10000;
// One write or CSV import into an existing sheet, about 8 MiB of revisions.
// The workbook limits in boundWorkbook still apply.
const MAX_EDIT_CELLS = 100000;
// A new sheet made from CSV. It becomes the replay base, not revisions.
const MAX_IMPORT_CELLS = 500000;
// One browser revision. A paste sends one command per pasted cell, so this is
// the largest paste; the 8 MiB message limit usually binds first.
const MAX_REVISION_COMMANDS = 100000;
// An agent write is stored as revisions of at most this many commands rather
// than one revision per cell, so a large import is a handful of log entries.
const BATCH_COMMANDS = 10000;
const MAX_WORKBOOK_BYTES = 8 * 1024 * 1024;
const count = (n) => n.toLocaleString('en-US');
function requireValue(condition, message) { if (!condition) throw new Error(message); }
function position(address) {
  const match = /^([A-Z]{1,3})([1-9][0-9]{0,5})$/i.exec(address);
  requireValue(match, 'Expected an A1 cell address');
  let col = 0;
  for (const c of match[1].toUpperCase()) col = col * 26 + c.charCodeAt(0) - 64;
  return { col: col - 1, row: Number(match[2]) - 1 };
}
function zone(range, limit = MAX_RANGE_CELLS) {
  const [start, end = start] = String(range).split(':');
  const a = position(start), b = position(end);
  requireValue(a.col <= b.col && a.row <= b.row, 'Range must run from top left to bottom right');
  requireValue((b.col - a.col + 1) * (b.row - a.row + 1) <= limit, `Range exceeds ${count(limit)} cells`);
  return { left: a.col, top: a.row, right: b.col, bottom: b.row };
}
function content(value) {
  requireValue(value === null || ['string', 'number', 'boolean'].includes(typeof value), 'Cell value must be text, number, boolean or null');
  requireValue(typeof value !== 'number' || Number.isFinite(value), 'Cell number must be finite');
  return value === null ? '' : typeof value === 'boolean' ? (value ? 'TRUE' : 'FALSE') : String(value);
}
function dispatch(model, type, payload) {
  if (type === 'ADD_COLUMNS_ROWS') {
    const size = model.getters.getSheetSize(payload.sheetId);
    requireValue(Number.isInteger(payload.quantity) && payload.quantity > 0 && payload.quantity <= 100000, 'Invalid structural edit size');
    const rows = size.numberOfRows + (payload.dimension === 'ROW' ? payload.quantity : 0);
    const cols = size.numberOfCols + (payload.dimension === 'COL' ? payload.quantity : 0);
    requireValue(rows <= 100000 && cols <= 1000 && rows * cols <= 5_000_000, 'Worksheet dimensions exceed the supported limit');
  }
  if (type === 'CREATE_SHEET' || type === 'DUPLICATE_SHEET') requireValue(model.getters.getSheetIds().length < 30, 'At most 30 worksheets are supported');
  const result = model.dispatch(type, payload);
  requireValue(result.isSuccessful, `Spreadsheet command ${type} rejected: ${JSON.stringify(result.reasons ?? result)}`);
}
// A local command that dispatches a list of core commands inside one root
// dispatch, as upstream's paste does. Upstream records them as one revision of
// those core commands only; this command itself is never stored or sent.
class ApplyCoreCommands extends upstream.UIPlugin {
  handle(cmd) {
    if (cmd.type !== 'APPLY_CORE_COMMANDS') return;
    for (const command of cmd.commands) {
      const result = this.dispatch(command.type, command);
      requireValue(result.isSuccessful, `Spreadsheet command ${command.type} rejected: ${JSON.stringify(result.reasons ?? result)}`);
    }
  }
}
upstream.registries.featurePluginRegistry.add('apply_core_commands', ApplyCoreCommands);
const STRUCTURAL = new Set(['ADD_COLUMNS_ROWS', 'CREATE_SHEET', 'DUPLICATE_SHEET']);
// Applies core commands in revisions of at most BATCH_COMMANDS. A root dispatch
// per command cost about 0.7 ms each, so 25,000 cells took 17 seconds. Commands
// that change the workbook's shape go one at a time through dispatch(), which
// bounds them; duplicating a worksheet is the one that multiplies size.
async function apply(model, commands) {
  let batch = [];
  const flush = async () => {
    if (batch.length) { dispatch(model, 'APPLY_CORE_COMMANDS', { commands: batch }); await Promise.resolve(); }
    batch = [];
  };
  for (const command of commands) {
    if (!STRUCTURAL.has(command.type)) {
      batch.push(command);
      if (batch.length === BATCH_COMMANDS) await flush();
      continue;
    }
    await flush();
    // Upstream allocates every inserted row or column at once, so a browser
    // revision is bounded before each insertion, not only after the whole list.
    if (command.type === 'ADD_COLUMNS_ROWS' && model.getters.getSheetIds().includes(command.sheetId)) {
      const size = model.getters.getSheetSize(command.sheetId), quantity = Number(command.quantity);
      const rows = size.numberOfRows + (command.dimension === 'ROW' ? quantity : 0);
      const cols = size.numberOfCols + (command.dimension === 'COL' ? quantity : 0);
      requireValue(Number.isInteger(quantity) && rows <= 100000 && cols <= 1000 && rows * cols <= 5_000_000, 'Worksheet dimensions exceed the supported limit');
    }
    dispatch(model, command.type, command);
    await Promise.resolve();
    if (command.type === 'DUPLICATE_SHEET') boundWorkbook(model.exportData());
    else requireValue(model.getters.getSheetIds().length <= 30, 'At most 30 worksheets are supported');
  }
  await flush();
}
function inWorksheet(model, sheetId, z) {
  const size = model.getters.getSheetSize(sheetId);
  requireValue(z.right < size.numberOfCols && z.bottom < size.numberOfRows, 'Range is outside the worksheet');
}
function readRange(model, sheetId, range) {
  const z = zone(range);
  inWorksheet(model, sheetId, z);
  const rows = [];
  for (let row = z.top; row <= z.bottom; row++) {
    const cells = [];
    for (let col = z.left; col <= z.right; col++) {
      const pos = { sheetId, row, col };
      const evaluated = model.getters.getEvaluatedCell(pos);
      cells.push({ content: model.getters.getCell(pos)?.content ?? '', value: evaluated.value ?? null,
        formatted: evaluated.formattedValue ?? '', type: evaluated.type, error: evaluated.error ?? null });
    }
    rows.push(cells);
  }
  return rows;
}
// Writes rows of values from a top-left cell, growing the worksheet to fit.
async function fill(model, sheetId, start, values, limit = MAX_EDIT_CELLS) {
  requireValue(Array.isArray(values) && values.every(Array.isArray), 'Expected rows of cell values');
  let cells = 0, width = 0;
  for (const row of values) { cells += row.length; width = Math.max(width, row.length); }
  requireValue(cells <= limit, `Expected at most ${count(limit)} cells in rows`);
  const size = model.getters.getSheetSize(sheetId), endRow = start.row + values.length, endCol = start.col + width;
  requireValue(endRow <= 100000 && endCol <= 1000 && Math.max(endRow, size.numberOfRows) * Math.max(endCol, size.numberOfCols) <= 5_000_000, 'Worksheet dimensions exceed the supported limit');
  const commands = [];
  for (const [dimension, limit, end] of [['ROW', size.numberOfRows, endRow], ['COL', size.numberOfCols, endCol]]) {
    if (end > limit) commands.push({ type: 'ADD_COLUMNS_ROWS', sheetId, dimension, position: 'after', base: limit - 1, quantity: end - limit });
  }
  for (let r = 0; r < values.length; r++) for (let c = 0; c < values[r].length; c++) {
    const value = values[r][c];
    const cell = value && typeof value === 'object' ? value : { content: value };
    const text = content(cell.content), pos = { sheetId, col: start.col + c, row: start.row + r };
    // A cell that already holds this content changes nothing, and the engine refuses a change
    // that changes nothing: skip it. This covers clearing an empty cell, and re-importing a file.
    if (!cell.format && (model.getters.getCell(pos)?.content ?? '') === text) continue;
    commands.push({ type: 'UPDATE_CELL', ...pos, content: text, ...(cell.format ? { format: cell.format } : {}) });
  }
  await apply(model, commands);
  return cells;
}
function parseCsv(csv) {
  requireValue(typeof csv === 'string' && csv.length <= MAX_WORKBOOK_BYTES, 'CSV exceeds 8 MiB');
  const rows = []; let row = [], field = '', quoted = false;
  for (let i = 0; i < csv.length; i++) {
    const c = csv[i];
    if (c === '"') {
      if (quoted && csv[i + 1] === '"') { field += '"'; i++; }
      else { requireValue(quoted || !field, 'Malformed CSV quotation'); quoted = !quoted; }
    } else if (c === ',' && !quoted) { row.push(field); field = ''; }
    else if ((c === '\r' || c === '\n') && !quoted) {
      if (c === '\r' && csv[i + 1] === '\n') i++;
      row.push(field); rows.push(row); row = []; field = '';
    } else field += c;
  }
  requireValue(!quoted, 'Unterminated CSV quotation');
  if (field || row.length) { row.push(field); rows.push(row); }
  return rows;
}
function quoteCsv(value) { const s = String(value ?? ''); return /[",\r\n]/.test(s) ? `"${s.replaceAll('"', '""')}"` : s; }
function usedZone(model, sheetId) {
  const cells = model.exportData().sheets.find(s => s.id === sheetId).cells;
  let right = 0, bottom = 0;
  for (const address of Object.keys(cells)) { const p = position(address); right = Math.max(right, p.col); bottom = Math.max(bottom, p.row); }
  return { left: 0, top: 0, right, bottom };
}
// Reads row by row, without the range-read cap, so a worksheet of any
// supported size exports whole.
function exportCsv(model, sheetId, range, formulas) {
  const z = range ? zone(range, 5_000_000) : usedZone(model, sheetId);
  inWorksheet(model, sheetId, z);
  const lines = [];
  for (let row = z.top; row <= z.bottom; row++) {
    const fields = [];
    for (let col = z.left; col <= z.right; col++) {
      const pos = { sheetId, row, col };
      fields.push(quoteCsv(formulas ? model.getters.getCell(pos)?.content : model.getters.getEvaluatedCell(pos).value));
    }
    lines.push(fields.join(','));
  }
  return lines.join('\r\n') + '\r\n';
}
function boundWorkbook(workbook) {
  requireValue(workbook.sheets.length <= 30, 'At most 30 worksheets are supported');
  for (const sheet of workbook.sheets) requireValue(sheet.rowNumber <= 100000 && sheet.colNumber <= 1000 && sheet.rowNumber * sheet.colNumber <= 5_000_000, 'Worksheet dimensions exceed the supported limit');
  requireValue(JSON.stringify(workbook).length <= MAX_WORKBOOK_BYTES, 'Workbook exceeds 8 MiB');
}
// Consecutive revisions one operation produced become one revision of up to
// BATCH_COMMANDS commands. Replay applies the same commands in the same order,
// and the chain still runs from the observed head to the operation's result.
function batched(messages) {
  const out = [];
  for (const message of messages) {
    const last = out.at(-1);
    if (last?.type === 'REMOTE_REVISION' && message.type === 'REMOTE_REVISION' && last.clientId === message.clientId
      && last.nextRevisionId === message.serverRevisionId && last.commands.length + message.commands.length <= BATCH_COMMANDS) {
      last.nextRevisionId = message.nextRevisionId;
      for (const command of message.commands) last.commands.push(command);
    } else out.push(message.type === 'REMOTE_REVISION' ? { ...message, commands: [...message.commands] } : message);
  }
  return out;
}
// A new sheet may start from a spreadsheet: CSV parsed here, or workbook data
// the browser converted from XLSX with this same engine. Either way the stored
// replay base is this engine's own export, never the submitted data.
async function importWorkbook(source) {
  requireValue(source && typeof source === 'object' && !Array.isArray(source), 'Expected a spreadsheet to import');
  if (typeof source.csv === 'string') {
    const model = new Model({});
    await fill(model, model.getters.getSheetIds()[0], { col: 0, row: 0 }, parseCsv(source.csv), MAX_IMPORT_CELLS);
    return model.exportData();
  }
  const workbook = source.workbook;
  requireValue(workbook && typeof workbook === 'object' && !Array.isArray(workbook) && Array.isArray(workbook.sheets), 'Expected CSV text or workbook data');
  boundWorkbook(workbook);
  const { revisionId: _submitted, ...data } = workbook;
  return new Model(data).exportData();
}

// Each request rebuilds from the immutable replay base and accepted log. This
// stateless worker never owns a database credential or accepts client snapshots.
export async function execute(input) {
  requireValue(input.engine_version === ENGINE_VERSION, 'Unsupported spreadsheet engine version');
  if (input.import) {
    requireValue(!input.snapshot && !input.messages && !input.revision && !input.operation, 'An import starts a new workbook');
    const workbook = await importWorkbook(input.import);
    boundWorkbook(workbook);
    return { workbook, messages: [], result: { worksheets: workbook.sheets.length }, engine_version: ENGINE_VERSION };
  }
  let receive;
  const outgoing = [];
  const clientId = input.client_id ?? randomUUID();
  const transport = {
    onNewMessage(_id, callback) { receive = callback; },
    async sendMessage(message) {
      if (['REMOTE_REVISION', 'REVISION_UNDONE', 'REVISION_REDONE'].includes(message.type)) {
        outgoing.push(message);
        queueMicrotask(() => receive({ ...message, timestamp: Date.now() }));
      }
    },
    leave() {},
  };
  const model = new Model(input.snapshot ?? {}, { transportService: transport,
    client: { id: clientId, name: 'Restless' } }, input.messages ?? []);
  if (input.revision) {
    const message = input.revision;
    requireValue(message.serverRevisionId === model.exportData().revisionId, 'Stale spreadsheet revision');
    if (message.type === 'REMOTE_REVISION') {
      requireValue(Array.isArray(message.commands), 'Invalid command list');
      requireValue(message.commands.length <= MAX_REVISION_COMMANDS, `One change can hold at most ${count(MAX_REVISION_COMMANDS)} cells`);
      // Validate the actual upstream core commands on a temporary model. It
      // checks dependent commands in sequence without changing accepted history.
      // The workbook is bounded once for the whole revision: bounding it after
      // every command made a large paste quadratic.
      for (const command of message.commands) requireValue(command && upstream.coreTypes.has(command.type), 'Only upstream core commands may be synchronized');
      const check = new Model(model.exportData());
      await apply(check, message.commands);
      boundWorkbook(check.exportData());
    }
    receive(message);
  }
  const op = input.operation;
  let result = null;
  if (op) {
    const sheetId = op.worksheet ?? model.getters.getSheetIds()[0];
    if (op.action !== 'create_worksheet') requireValue(model.getters.getSheetIds().includes(sheetId), 'Worksheet is unavailable');
    switch (op.action) {
      case 'get_range': result = { rows: readRange(model, sheetId, op.range) }; break;
      case 'set_cells': result = { changed_cells: await fill(model, sheetId, position(op.start ?? 'A1'), op.values) }; break;
      case 'insert_rows':
        requireValue(Number.isInteger(op.before) && op.before >= 1 && Number.isInteger(op.count) && op.count > 0 && op.count <= 1000, 'Invalid row insertion');
        dispatch(model, 'ADD_COLUMNS_ROWS', { sheetId, dimension: 'ROW', position: 'before', base: op.before - 1, quantity: op.count }); break;
      case 'sort': {
        const z = zone(op.range), anchor = position(op.by);
        dispatch(model, 'SORT_CELLS', { sheetId, zone: z, col: anchor.col, row: anchor.row,
          sortDirection: op.direction === 'desc' ? 'desc' : 'asc', sortOptions: { hasHeader: Boolean(op.header) } }); break;
      }
      case 'filter': {
        const rows = readRange(model, sheetId, op.range), index = Number(op.column ?? 0);
        requireValue(Number.isInteger(index) && index >= 0 && index < rows[0]?.length, 'Invalid filter column');
        result = { rows: rows.filter((row, i) => (op.header && i === 0) || row[index].value === op.equals) }; break;
      }
      case 'create_worksheet':
        dispatch(model, 'CREATE_SHEET', { sheetId: op.id ?? randomUUID(), name: op.name }); break;
      case 'export_csv': result = { csv: exportCsv(model, sheetId, op.range, op.formulas) }; break;
      case 'import_csv': {
        const values = parseCsv(op.csv);
        return execute({ ...input, operation: { action: 'set_cells', worksheet: sheetId, start: op.start ?? 'A1', values } });
      }
      case 'update_record': {
        const z = zone(op.range), rows = readRange(model, sheetId, op.range);
        const names = rows[0].map(c => String(c.value));
        const idColumn = names.indexOf(op.id_column ?? 'record_id');
        requireValue(idColumn >= 0 && new Set(names).size === names.length, 'Record range needs unique named columns and an ID column');
        const matches = rows.slice(1).map((r, i) => ({ r, i })).filter(x => String(x.r[idColumn].value) === String(op.record_id));
        requireValue(matches.length === 1, 'Record ID must identify exactly one row');
        const row = z.top + matches[0].i + 1;
        requireValue(op.values && typeof op.values === 'object' && !Array.isArray(op.values), 'Expected named record values');
        for (const [name, value] of Object.entries(op.values)) {
          const index = names.indexOf(name); requireValue(index >= 0 && index !== idColumn, 'Unknown column or immutable record ID');
          dispatch(model, 'UPDATE_CELL', { sheetId, row, col: z.left + index, content: content(value) }); await Promise.resolve();
        }
        result = { record_id: op.record_id }; break;
      }
      case 'snapshot': result = { workbook: model.exportData() }; break;
      default: throw new Error('Unsupported sheet action');
    }
  }
  await Promise.resolve();
  const workbook = model.exportData();
  boundWorkbook(workbook);
  return { workbook, messages: batched(outgoing), result, engine_version: ENGINE_VERSION };
}

if (process.argv[1]?.endsWith('/worker.mjs')) {
  const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
  for await (const line of lines) {
    try { process.stdout.write(JSON.stringify({ ok: true, ...await execute(JSON.parse(line)) }) + '\n'); }
    catch (error) { process.stdout.write(JSON.stringify({ ok: false, error: error.message }) + '\n'); }
  }
  process.exit(0);
}
