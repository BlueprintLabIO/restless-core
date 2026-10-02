import { createInterface } from 'node:readline';
import { randomUUID } from 'node:crypto';
import { Model, upstream } from './upstream.mjs';

export const ENGINE_VERSION = '19.0.51';
const MAX_CELLS = 10000;
function requireValue(condition, message) { if (!condition) throw new Error(message); }
function position(address) {
  const match = /^([A-Z]{1,3})([1-9][0-9]{0,5})$/i.exec(address);
  requireValue(match, 'Expected an A1 cell address');
  let col = 0;
  for (const c of match[1].toUpperCase()) col = col * 26 + c.charCodeAt(0) - 64;
  return { col: col - 1, row: Number(match[2]) - 1 };
}
function zone(range) {
  const [start, end = start] = String(range).split(':');
  const a = position(start), b = position(end);
  requireValue(a.col <= b.col && a.row <= b.row, 'Range must run from top left to bottom right');
  requireValue((b.col - a.col + 1) * (b.row - a.row + 1) <= MAX_CELLS, 'Range exceeds 10,000 cells');
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
function readRange(model, sheetId, range) {
  const z = zone(range);
  const size = model.getters.getSheetSize(sheetId);
  requireValue(z.right < size.numberOfCols && z.bottom < size.numberOfRows, 'Range is outside the worksheet');
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
function parseCsv(csv) {
  requireValue(typeof csv === 'string' && csv.length <= 2_000_000, 'CSV exceeds limit');
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
function usedRange(model, sheetId) {
  const cells = model.exportData().sheets.find(s => s.id === sheetId).cells;
  let right = 0, bottom = 0;
  for (const address of Object.keys(cells)) { const p = position(address); right = Math.max(right, p.col); bottom = Math.max(bottom, p.row); }
  let name = '', n = right + 1;
  while (n) { name = String.fromCharCode(65 + (n - 1) % 26) + name; n = Math.floor((n - 1) / 26); }
  return `A1:${name}${bottom + 1}`;
}
function boundWorkbook(workbook) {
  requireValue(workbook.sheets.length <= 30, 'At most 30 worksheets are supported');
  for (const sheet of workbook.sheets) requireValue(sheet.rowNumber <= 100000 && sheet.colNumber <= 1000 && sheet.rowNumber * sheet.colNumber <= 5_000_000, 'Worksheet dimensions exceed the supported limit');
  requireValue(JSON.stringify(workbook).length <= 8 * 1024 * 1024, 'Workbook exceeds 8 MiB');
}

// Each request rebuilds from the immutable replay base and accepted log. This
// stateless worker never owns a database credential or accepts client snapshots.
export async function execute(input) {
  requireValue(input.engine_version === ENGINE_VERSION, 'Unsupported spreadsheet engine version');
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
      requireValue(Array.isArray(message.commands) && message.commands.length <= MAX_CELLS, 'Invalid command list');
      // Validate the actual upstream core commands on a temporary model. It
      // checks dependent commands in sequence without changing accepted history.
      const check = new Model(model.exportData());
        for (const command of message.commands) {
        requireValue(command && upstream.coreTypes.has(command.type), 'Only upstream core commands may be synchronized');
          dispatch(check, command.type, command);
          boundWorkbook(check.exportData());
      }
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
      case 'set_cells': {
        const start = position(op.start ?? 'A1');
        requireValue(Array.isArray(op.values) && op.values.reduce((n, r) => n + (Array.isArray(r) ? r.length : MAX_CELLS + 1), 0) <= MAX_CELLS, 'Expected at most 10,000 cells in rows');
        const size = model.getters.getSheetSize(sheetId), endRow = start.row + op.values.length,
          endCol = start.col + Math.max(0, ...op.values.map(r => r.length));
        requireValue(endRow <= 100000 && endCol <= 1000 && Math.max(endRow, size.numberOfRows) * Math.max(endCol, size.numberOfCols) <= 5_000_000, 'Worksheet dimensions exceed the supported limit');
        for (const [dimension, limit, end] of [['ROW', size.numberOfRows, endRow], ['COL', size.numberOfCols, endCol]]) {
          if (end > limit) { dispatch(model, 'ADD_COLUMNS_ROWS', { sheetId, dimension, position: 'after', base: limit - 1, quantity: end - limit }); await Promise.resolve(); }
        }
        for (let r = 0; r < op.values.length; r++) for (let c = 0; c < op.values[r].length; c++) {
          const value = op.values[r][c];
          const cell = value && typeof value === 'object' ? value : { content: value };
          dispatch(model, 'UPDATE_CELL', { sheetId, col: start.col + c, row: start.row + r, content: content(cell.content), ...(cell.format ? { format: cell.format } : {}) });
          await Promise.resolve();
        }
        result = { changed_cells: op.values.reduce((n, r) => n + r.length, 0) }; break;
      }
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
      case 'export_csv': result = { csv: readRange(model, sheetId, op.range ?? usedRange(model, sheetId)).map(row => row.map(c => quoteCsv(op.formulas ? c.content : c.value)).join(',')).join('\r\n') + '\r\n' }; break;
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
  return { workbook, messages: outgoing, result, engine_version: ENGINE_VERSION };
}

if (process.argv[1]?.endsWith('/worker.mjs')) {
  const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
  for await (const line of lines) {
    try { process.stdout.write(JSON.stringify({ ok: true, ...await execute(JSON.parse(line)) }) + '\n'); }
    catch (error) { process.stdout.write(JSON.stringify({ ok: false, error: error.message }) + '\n'); }
  }
  process.exit(0);
}
