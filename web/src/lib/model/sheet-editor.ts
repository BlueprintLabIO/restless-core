import { App, Component, xml } from '@odoo/owl';
import {
	CommandResult,
	CorePlugin,
	Model,
	Spreadsheet,
	UIPlugin,
	helpers,
	registries
} from '@odoo/o-spreadsheet/dist/o_spreadsheet.esm.js';
import templates from '@odoo/o-spreadsheet/dist/o_spreadsheet.xml?raw';
import spreadsheetCss from '@odoo/o-spreadsheet/dist/o_spreadsheet.css?inline';
import bootstrapCss from 'bootstrap/dist/css/bootstrap.min.css?inline';
import fontCss from 'font-awesome/css/font-awesome.min.css?inline';
import type { SheetRow } from './sheets';
import { sheetFaces, type PageFace } from './page-faces.ts';

class Root extends Component {
	static template = xml`<Spreadsheet model="props.model"/>`;
	static components = { Spreadsheet };
	static props = ['model'];
}

/** A single upstream model survives disconnects, including its pending OT edits.
 * Catch-up and lost acknowledgements arrive before retained submissions retry. */
export interface EditorStatus {
	state: string;
	access: string;
	revision?: string;
	worksheet?: string;
	message?: string;
	denied?: boolean;
	/** The sheet as the server holds it, from the session's bootstrap. */
	sheet?: SheetRow;
	/** A change was refused and the sheet reopened at its saved state; say so once. */
	notice?: { title: string; text: string; details?: string };
}
type UpstreamMessage = Record<string, any>;
/** The cells a person has selected: the tab by the grid's id and name, the cell as A1 or the
 * range as A1:C3. */
export interface SheetSelection {
	worksheet: string;
	name: string;
	cell: string;
	/** Every tab, by id, with its name now. */
	tabs: Record<string, string>;
}

/* "Comment" in the grid's own right-click menu, for anyone who can open the sheet, opens the
 * comments on the cell. The menu is the engine's, shared by every open sheet; it calls whichever
 * sheet was opened last. */
let commentRequest: (() => void) | undefined;
let commentMenuAdded = false;
function addCommentMenu() {
	if (commentMenuAdded) return;
	commentMenuAdded = true;
	registries.cellMenuRegistry.add('restless_comment', {
		name: 'Comment',
		description: navigator.platform.includes('Mac') ? '⌘⌥M' : 'Ctrl+Alt+M',
		sequence: 1,
		separator: true,
		isReadonlyAllowed: true,
		execute: () => commentRequest?.()
	});
}
/* The server keeps at most 30 tabs. A new or duplicated tab past that is refused here, before the
 * grid shows it, and the person is told why; refused by the server instead, it would cost the
 * edits queued behind it. Like the comment menu, the refusal speaks for the sheet opened last. */
const MAX_TABS = 30;
let tabLimitReached: (() => void) | undefined;
/* After a refused change, the edits queued behind it are applied again on the reopened sheet as
 * one change. Each command is checked on its own: one that needed the refused change is dropped. */
let replayed: { applied: number; dropped: number } = { applied: 0, dropped: 0 };
let pluginAdded = false;
function addPlugin() {
	if (pluginAdded) return;
	pluginAdded = true;
	// The engine checks a shared (core) command only with core plugins. This one keeps no state.
	class RestlessTabLimit extends CorePlugin {
		allowDispatch(cmd: any) {
			if (
				(cmd.type === 'CREATE_SHEET' || cmd.type === 'DUPLICATE_SHEET') &&
				this.getters.getSheetIds().length >= MAX_TABS
			) {
				tabLimitReached?.();
				return CommandResult.CancelledForUnknownReason;
			}
			return CommandResult.Success;
		}
	}
	registries.corePluginRegistry.add('restless_tab_limit', RestlessTabLimit);
	class RestlessSheetPlugin extends UIPlugin {
		handle(cmd: any) {
			if (cmd.type !== 'RESTLESS_REPLAY') return;
			for (const command of cmd.commands as any[]) {
				const result = this.dispatch(command.type, command);
				if (result.isSuccessful) replayed.applied++;
				else replayed.dropped++;
			}
		}
	}
	registries.featurePluginRegistry.add('restless_sheet', RestlessSheetPlugin);
}
/* Commands whose undoing moves nothing else: a later edit means the same cells without them.
 * A new tab qualifies too, since an edit inside it simply has nowhere to land. */
function independent(command: { type: string }) {
	return (
		command.type === 'CREATE_SHEET' ||
		command.type === 'DUPLICATE_SHEET' ||
		registries.inverseCommandRegistry.get(command.type) ===
			registries.inverseCommandRegistry.get('UPDATE_CELL')
	);
}
/** A cell to mark as carrying a comment thread. */
export interface SheetMark {
	worksheet: string;
	cell: string;
}

/* Commented cells carry a small corner mark, drawn over the grid where the engine draws other
 * people's cursors, and redrawn whenever the grid changes or scrolls. It never takes a click. */
function drawMarks(model: any, element: HTMLElement, marks: readonly SheetMark[]) {
	const canvas = element.querySelector<HTMLCanvasElement>('.o-grid canvas');
	const host = canvas?.parentElement;
	let layer = element.querySelector<HTMLDivElement>('.sheet-comment-marks');
	if (!host) return;
	if (!layer || layer.parentElement !== host) {
		layer?.remove();
		layer = document.createElement('div');
		layer.className = 'sheet-comment-marks';
		layer.setAttribute('aria-hidden', 'true');
		layer.style.cssText = 'position:absolute;inset:0;pointer-events:none;overflow:hidden;z-index:1';
		host.append(layer);
	}
	const sheetId = model.getters.getActiveSheetId();
	const seen = new Set<string>();
	const html: string[] = [];
	for (const mark of marks) {
		if (mark.worksheet !== sheetId || seen.has(mark.cell)) continue;
		seen.add(mark.cell);
		// A range is marked at its top right cell, where a single cell is marked.
		let position: { col: number; row: number };
		try {
			const zone = helpers.toZone(mark.cell);
			position = { col: zone.right, row: zone.top };
		} catch {
			continue;
		}
		if (!model.getters.isVisibleInViewport({ sheetId, ...position })) continue;
		const { x, y, width } = model.getters.getVisibleRect({
			left: position.col,
			top: position.row,
			right: position.col,
			bottom: position.row
		});
		html.push(
			`<span style="position:absolute;left:${x + width - 8}px;top:${y + 1}px;width:0;height:0;border-top:7px solid #e8a317;border-left:7px solid transparent"></span>`
		);
	}
	const next = html.join('');
	if (layer.innerHTML !== next) layer.innerHTML = next;
}

/* One change is one revision. A paste sends a command per cell, and the server takes up to
 * 100,000 commands in a message of at most 8 MiB. Past either, it is refused here, before it is
 * sent, rather than after the server has closed the session. */
const MAX_REVISION_COMMANDS = 100_000;
const MAX_MESSAGE_BYTES = 8 * 1024 * 1024 - 4096;
function messageBytes(text: string) {
	return text.length * 3 <= MAX_MESSAGE_BYTES ? text.length : new TextEncoder().encode(text).length;
}

/* Columns fit what they hold the first time an editor opens a sheet, as a real edit everyone
 * shares, then stay as people set them. A column of long links is capped so it cannot swallow
 * the view. */
const FIT_MAX_WIDTH = 320;
function fitColumnsOnce(model: any, sheet: string) {
	const key = `restless:sheet-fitted:${sheet}`;
	try {
		if (localStorage.getItem(key)) return;
		localStorage.setItem(key, '1');
	} catch {
		return;
	}
	for (const sheetId of model.getters.getSheetIds()) {
		// Only columns that hold something: an empty column keeps the default width.
		const used = [
			...new Set(
				Object.keys(model.getters.getCells(sheetId) ?? {}).map(
					(cellId) => model.getters.getCellPosition(cellId).col as number
				)
			)
		];
		if (!used.length) continue;
		model.dispatch('AUTORESIZE_COLUMNS', { sheetId, cols: used });
		const wide = used.filter((col) => model.getters.getColSize(sheetId, col) > FIT_MAX_WIDTH);
		if (wide.length)
			model.dispatch('RESIZE_COLUMNS_ROWS', {
				sheetId,
				dimension: 'COL',
				elements: wide,
				size: FIT_MAX_WIDTH
			});
	}
}
export function mountSheet(
	element: HTMLDivElement,
	company: string,
	sheet: string,
	status: (value: EditorStatus) => void,
	/** Who else has this sheet open, or just changed it; cleared while the session is down. */
	faces: (value: PageFace[]) => void = () => {},
	/** The cell the person has selected, each time it changes. */
	selected: (value: SheetSelection) => void = () => {},
	/** The person asked to comment on the selected cell. */
	comment: () => void = () => {}
) {
	addCommentMenu();
	addPlugin();
	commentRequest = comment;
	const refuseTab = () =>
		queueMicrotask(() =>
			tell({
				title: `A sheet can have up to ${MAX_TABS} tabs`,
				text: 'Delete a tab you no longer need, or start a new sheet for the rest.'
			})
		);
	tabLimitReached = refuseTab;
	const shortcut = (event: KeyboardEvent) => {
		if (
			(event.ctrlKey || event.metaKey) &&
			event.altKey &&
			event.code === 'KeyM' &&
			event.target instanceof Node &&
			element.contains(event.target)
		) {
			event.preventDefault();
			event.stopPropagation();
			comment();
		}
	};
	element.addEventListener('keydown', shortcut, true);
	/* Tab moves between cells, as in any spreadsheet, so the grid would keep the keyboard for good.
	 * Escape, then Tab (or Shift+Tab), leaves it for the controls before or after the sheet. */
	let leaving = false;
	const leave = (event: KeyboardEvent) => {
		if (!(event.target instanceof Node) || !element.contains(event.target)) return;
		if (event.key === 'Escape') {
			leaving = true;
			return;
		}
		if (event.key !== 'Tab' || !leaving) {
			if (!['Shift', 'Control', 'Alt', 'Meta'].includes(event.key)) leaving = false;
			return;
		}
		leaving = false;
		const outside = [
			...document.querySelectorAll<HTMLElement>(
				'a[href],button:not(:disabled),input:not(:disabled),textarea:not(:disabled),select:not(:disabled),summary,[contenteditable="true"],[tabindex]:not([tabindex="-1"])'
			)
		].filter(
			(candidate) =>
				!element.contains(candidate) &&
				candidate.getClientRects().length > 0 &&
				!candidate.closest('[inert],[aria-hidden="true"],[hidden]')
		);
		const after = (candidate: HTMLElement) =>
			Boolean(element.compareDocumentPosition(candidate) & Node.DOCUMENT_POSITION_FOLLOWING);
		const next = event.shiftKey
			? outside.filter((candidate) => !after(candidate)).at(-1)
			: outside.find(after);
		if (!next) return;
		event.preventDefault();
		event.stopPropagation();
		next.focus();
	};
	const pointer = () => (leaving = false);
	element.addEventListener('keydown', leave, true);
	element.addEventListener('pointerdown', pointer, true);
	let model: any,
		app: App<typeof Root> | undefined,
		socket: WebSocket | undefined,
		receive: ((message: UpstreamMessage) => void) | undefined;
	let cursor = 0,
		clientId = '',
		reconnectToken = '',
		disposed = false,
		blocked = false,
		timer: ReturnType<typeof setTimeout>;
	let access = 'read';
	let row: SheetRow | undefined;
	let present: unknown = [];
	let marks: readonly SheetMark[] = [];
	let lastSelection = '';
	const listener = {};
	function gridChanged() {
		if (!model) return;
		const { col, row } = model.getters.getActivePosition();
		const zone = model.getters.getSelectedZone();
		const worksheet = model.getters.getActiveSheetId();
		const tabs: Record<string, string> = {};
		for (const id of model.getters.getSheetIds()) tabs[id] = model.getters.getSheetName(id);
		const single = !zone || (zone.left === zone.right && zone.top === zone.bottom);
		const value = {
			worksheet,
			name: tabs[worksheet] ?? '',
			cell: single
				? helpers.toXC(col, row)
				: `${helpers.toXC(zone.left, zone.top)}:${helpers.toXC(zone.right, zone.bottom)}`,
			tabs
		};
		const key = JSON.stringify(value);
		if (key !== lastSelection) {
			lastSelection = key;
			selected(value);
		}
		drawMarks(model, element, marks);
	}
	/* Each face takes the selection colour the grid gives that person's session. */
	function showFaces(value: unknown = present) {
		present = value;
		const clients: { id: string; color?: string }[] = model?.getters.getConnectedClients() ?? [];
		faces(sheetFaces(present, (id) => clients.find((client) => client.id === id)?.color));
	}
	const pending = new Map();
	const styles = document.createElement('style');
	// Bootstrap is required by Owl, and scoped to this editor so it cannot reset
	// the cockpit typography or controls. Upstream's own CSS already uses .o-spreadsheet.
	styles.textContent = `${spreadsheetCss}\n@scope (.native-sheet-surface) { ${bootstrapCss.replaceAll(':root', ':scope')} ${fontCss} }`;
	element.append(styles);
	const target = document.createElement('div');
	target.className = 'sheet-owl-root';
	target.style.cssText = 'height:100%;width:100%;min-height:0';
	element.append(target);
	const transport = {
		onNewMessage(_id: string, callback: (message: UpstreamMessage) => void) {
			receive = callback;
		},
		async sendMessage(message: UpstreamMessage) {
			if (message.type === 'SNAPSHOT') return; // Only server-engine snapshots are authoritative.
			const text = JSON.stringify(message);
			if (message.type === 'REMOTE_REVISION') {
				const cells = message.commands?.length ?? 0;
				if (cells > MAX_REVISION_COMMANDS || messageBytes(text) > MAX_MESSAGE_BYTES) {
					// Never sent: reopening drops it locally; edits queued behind it are applied again.
					queueMicrotask(() =>
						reload({
							title: 'That change is too large to save at once',
							text: 'Paste it in smaller parts, or import the file as a new sheet.',
							details: `It changes about ${cells.toLocaleString()} cells. One change can hold up to ${MAX_REVISION_COMMANDS.toLocaleString()} cells and 8 MB.`
						})
					);
					return;
				}
			}
			if (message.nextRevisionId) pending.set(message.nextRevisionId, message);
			if (socket?.readyState === WebSocket.OPEN && !blocked) socket.send(text);
			queueMicrotask(report);
		},
		leave() {}
	};
	function withLost(details: string | undefined, lost: number) {
		const note = `${lost.toLocaleString()} later ${lost === 1 ? 'edit' : 'edits'} made after it ${lost === 1 ? 'was' : 'were'} lost too.`;
		return details ? `${details}\n${note}` : note;
	}
	/* The kept edits go in as one new change, so the owner can undo them together. */
	function replay(kept: NonNullable<typeof carried>) {
		replayed = { applied: 0, dropped: 0 };
		if (access === 'edit') model.dispatch('RESTLESS_REPLAY', { commands: kept.commands });
		const { applied, dropped } = replayed;
		const edits = `${kept.edits.toLocaleString()} ${kept.edits === 1 ? 'edit' : 'edits'} you made after it`;
		const one = kept.edits === 1;
		tell({
			...kept.notice,
			text: `${kept.notice.text} ${applied ? `The ${edits} ${one ? 'was' : 'were'} kept and ${one ? 'is' : 'are'} saving.` : `The ${edits} could not be kept.`}`,
			details:
				[
					kept.notice.details,
					dropped && applied
						? `${dropped.toLocaleString()} cell ${dropped === 1 ? 'change' : 'changes'} needed it and ${dropped === 1 ? 'was' : 'were'} dropped.`
						: '',
					kept.lost ? withLost(undefined, kept.lost) : ''
				]
					.filter(Boolean)
					.join('\n') || undefined
		});
	}
	function accepted(sequence: number, message: UpstreamMessage) {
		if (sequence <= cursor) return;
		receive?.(message);
		pending.delete(message.nextRevisionId);
		cursor = sequence;
	}
	function report() {
		if (disposed || blocked || !model) return;
		if (socket?.readyState !== WebSocket.OPEN) {
			status({
				state: 'offline',
				access,
				message: pending.size
					? 'Keep editing; your changes are kept in this tab and save when it reconnects.'
					: 'Keep editing; your changes save when it reconnects.'
			});
			return;
		}
		status({
			state: pending.size ? 'saving' : 'saved',
			access,
			revision: model?.exportData().revisionId,
			worksheet: model?.getters.getActiveSheetId(),
			sheet: row
		});
	}
	function fail(message: string, denied = false) {
		if (disposed) return;
		blocked = true;
		socket?.close();
		app?.destroy();
		app = undefined;
		model = undefined;
		target.replaceChildren();
		showFaces([]);
		status({ state: 'error', message, access: 'read', denied });
	}
	/** Say something once, over the sheet as it is now. */
	function tell(notice: NonNullable<EditorStatus['notice']>) {
		if (disposed || blocked || !model) return;
		status({
			state: pending.size ? 'saving' : 'saved',
			access,
			revision: model.exportData().revisionId,
			worksheet: model.getters.getActiveSheetId(),
			sheet: row,
			notice
		});
	}
	/* The engine sends one change at a time, so the refused one heads its queue and every edit
	 * after it waits behind. Those later edits are kept to apply again on the reopened sheet,
	 * unless the refused change moved cells or tabs they were made against. */
	let carried:
		| { commands: any[]; edits: number; lost: number; notice: NonNullable<EditorStatus['notice']> }
		| undefined;
	function carryQueue(notice: NonNullable<EditorStatus['notice']>) {
		const session = model?.session;
		const queue: UpstreamMessage[] = session?.pendingMessages ?? [];
		const commandsOf = (message: UpstreamMessage): any[] =>
			session?.revisions?.get?.(message.nextRevisionId)?.commands ?? message.commands ?? [];
		const [refused, ...later] = queue;
		const edits = later.filter((message) => message.type === 'REMOTE_REVISION');
		const safe = !refused || commandsOf(refused).every(independent);
		if (!edits.length || !safe) return { commands: [], edits: 0, lost: later.length, notice };
		return {
			notice,
			commands: edits.flatMap(commandsOf),
			edits: edits.length,
			// An undo or redo of something else cannot be carried across sessions.
			lost: later.length - edits.length
		};
	}
	/* A change the server refused, or one too large to send, must not take the editor down with
	 * it: the sheet reopens as a fresh session at its saved state, the edits queued behind it are
	 * applied again where they still fit, and the owner is told once what was kept and lost. */
	function reload(notice: NonNullable<EditorStatus['notice']>) {
		if (disposed || blocked) return;
		carried = access === 'edit' ? carryQueue(notice) : undefined;
		if (carried && !carried.edits && carried.lost)
			notice = { ...notice, details: withLost(notice.details, carried.lost) };
		const previous = socket;
		socket = undefined;
		previous?.close();
		clearTimeout(timer);
		app?.destroy();
		app = undefined;
		model = undefined;
		receive = undefined;
		pending.clear();
		cursor = 0;
		target.replaceChildren();
		showFaces([]);
		status({ state: 'connecting', access, sheet: row, notice });
		connect();
	}
	async function mount(mounted: App<typeof Root>) {
		const outsideControl = (candidate: EventTarget | null) =>
			candidate instanceof HTMLElement &&
			!element.contains(candidate) &&
			candidate.matches(
				'a[href],button,input,textarea,select,[contenteditable="true"],[tabindex]'
			) &&
			candidate.isConnected &&
			candidate.getClientRects().length > 0 &&
			!candidate.closest('[inert],[aria-hidden="true"]')
				? candidate
				: null;
		let retained = outsideControl(document.activeElement);
		const focus = (event: FocusEvent) => {
			const control = outsideControl(event.target);
			if (control) retained = control;
		};
		const pointer = (event: PointerEvent) => {
			retained = outsideControl(
				event.target instanceof Element
					? event.target.closest(
							'a[href],button,input,textarea,select,[contenteditable="true"],[tabindex]'
						)
					: null
			);
		};
		const key = (event: KeyboardEvent) => {
			if (event.target instanceof Node && element.contains(event.target)) retained = null;
		};
		document.addEventListener('focusin', focus, true);
		document.addEventListener('pointerdown', pointer, true);
		document.addEventListener('keydown', key, true);
		try {
			await mounted.mount(target);
		} finally {
			document.removeEventListener('focusin', focus, true);
			document.removeEventListener('pointerdown', pointer, true);
			document.removeEventListener('keydown', key, true);
			// Owl focuses its grid on mount. Keep deliberate sidebar/back-control
			// focus, unless the user entered the workbook while it was loading.
			if (
				!disposed &&
				!blocked &&
				element.contains(document.activeElement) &&
				outsideControl(retained)
			)
				retained!.focus({ preventScroll: true });
		}
	}
	function connect() {
		if (disposed || blocked) return;
		status({ state: 'connecting', access });
		const url = new URL(
			`/api/companies/${encodeURIComponent(company)}/sheets/${encodeURIComponent(sheet)}/collaboration`,
			location.href
		);
		url.protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
		if (model) {
			url.searchParams.set('after', String(cursor));
			url.searchParams.set('client_id', clientId);
			url.searchParams.set('reconnect_token', reconnectToken);
		}
		const current = new WebSocket(url);
		socket = current;
		// A session replaced by a reload is ignored, including its late close.
		const stale = () => disposed || blocked || socket !== current;
		current.onmessage = async (event) => {
			if (stale()) return;
			try {
				const envelope = JSON.parse(event.data);
				if (envelope.type === 'ERROR') {
					const message = String(envelope.message);
					if (message.includes('unavailable')) fail(message, true);
					// Before the sheet has opened there is no change to drop; reopening would only repeat it.
					else if (!model) fail(message);
					else
						reload({
							title: 'Your last change was not saved',
							text: 'The sheet reopened at its last saved state.',
							details: message
						});
					return;
				}
				// Other people's and agents' cursors: shown, never sequenced or stored.
				if (envelope.type === 'PRESENCE') {
					if (model) receive?.(envelope.message);
					showFaces();
					return;
				}
				if (envelope.type === 'FACES') {
					showFaces(envelope.faces);
					return;
				}
				if (envelope.type === 'ACCESS') {
					access = envelope.access;
					model?.updateMode(access === 'edit' ? 'normal' : 'readonly');
					report();
					return;
				}
				if (envelope.type === 'BOOTSTRAP') {
					const state = envelope.state;
					access = state.access;
					row = state.sheet;
					if (state.sheet.engine_version !== '19.0.51')
						throw new Error('This sheet requires a different editor version.');
					if (!model) {
						clientId = envelope.client_id;
						reconnectToken = envelope.reconnect_token;
						model = new Model(
							state.snapshot,
							{
								transportService: transport,
								client: { id: clientId, name: 'You' },
								mode: access === 'edit' ? 'normal' : 'readonly'
							},
							state.messages.map((m: { message: UpstreamMessage }) => m.message)
						);
						cursor = state.sheet.sequence;
						lastSelection = '';
						model.on('update', listener, gridChanged);
						const mounted = new App(Root, { props: { model }, templates });
						app = mounted;
						await mount(mounted);
						if (stale()) {
							mounted.destroy();
							return;
						}
						if (access === 'edit') fitColumnsOnce(model, sheet);
						if (carried?.edits) replay(carried);
						carried = undefined;
						gridChanged();
					} else for (const m of state.messages) accepted(m.sequence, m.message);
					model.updateMode(access === 'edit' ? 'normal' : 'readonly');
					for (const message of pending.values()) socket?.send(JSON.stringify(message));
					report();
				} else if (envelope.type === 'MESSAGE') {
					accepted(envelope.sequence, envelope.message);
					report();
				}
			} catch (error) {
				fail(error instanceof Error ? error.message : 'The sheet could not synchronize.');
			}
		};
		current.onclose = async () => {
			if (stale()) return;
			/* The grid stays writable while it reconnects: edits queue in this tab and are sent,
			 * in order, as soon as the session resumes. */
			showFaces([]);
			if (model) report();
			else status({ state: 'offline', access, message: 'Reconnecting.' });
			try {
				const response = await fetch(
					`/api/companies/${encodeURIComponent(company)}/sheets/${encodeURIComponent(sheet)}`,
					{ cache: 'no-store' }
				);
				if (stale()) return;
				if ([401, 403, 404].includes(response.status)) {
					fail('Access to this sheet has ended.', true);
					return;
				}
			} catch {
				/* A transient outage retains pending work for this identity. */
			}
			if (stale()) return;
			timer = setTimeout(connect, 1500);
		};
	}
	connect();
	return {
		getModel: () => model,
		/** Show a cell or range: its tab, selected, scrolled into view. False if the tab is gone. */
		select(worksheet: string, cell: string): boolean {
			if (!model || !model.getters.getSheetIds().includes(worksheet)) return false;
			const active = model.getters.getActiveSheetId();
			if (active !== worksheet)
				model.dispatch('ACTIVATE_SHEET', { sheetIdFrom: active, sheetIdTo: worksheet });
			const zone = helpers.toZone(cell);
			const at = { col: zone.left, row: zone.top };
			if (zone.left === zone.right && zone.top === zone.bottom)
				model.selection.selectCell(at.col, at.row);
			else model.selection.selectZone({ cell: at, zone });
			model.dispatch('SCROLL_TO_CELL', at);
			return true;
		},
		/** The cells to mark as carrying a comment thread. */
		mark(value: readonly SheetMark[]) {
			marks = value;
			if (model) drawMarks(model, element, marks);
		},
		destroy() {
			disposed = true;
			element.removeEventListener('keydown', shortcut, true);
			element.removeEventListener('keydown', leave, true);
			element.removeEventListener('pointerdown', pointer, true);
			if (commentRequest === comment) commentRequest = undefined;
			if (tabLimitReached === refuseTab) tabLimitReached = undefined;
			clearTimeout(timer);
			socket?.close();
			app?.destroy();
			target.remove();
			styles.remove();
		}
	};
}
