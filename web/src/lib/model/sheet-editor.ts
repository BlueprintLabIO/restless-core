import { App, Component, xml } from '@odoo/owl';
import {
	Model,
	Spreadsheet,
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
/** The cell a person has selected: the tab by the grid's id and name, the cell as A1. */
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
		let position: { col: number; row: number };
		try {
			position = helpers.toCartesian(mark.cell);
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
	commentRequest = comment;
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
		const worksheet = model.getters.getActiveSheetId();
		const tabs: Record<string, string> = {};
		for (const id of model.getters.getSheetIds()) tabs[id] = model.getters.getSheetName(id);
		const value = {
			worksheet,
			name: tabs[worksheet] ?? '',
			cell: helpers.toXC(col, row),
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
					// Never sent: reopening drops it locally, with the pending edits queued behind it.
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
	function accepted(sequence: number, message: UpstreamMessage) {
		if (sequence <= cursor) return;
		receive?.(message);
		pending.delete(message.nextRevisionId);
		cursor = sequence;
	}
	function report() {
		if (disposed || blocked || !model) return;
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
	/* A change the server refused, or one too large to send, must not take the editor down with
	 * it: the sheet reopens as a fresh session at its saved state and the owner is told once. */
	function reload(notice: NonNullable<EditorStatus['notice']>) {
		if (disposed || blocked) return;
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
			model?.updateMode('readonly');
			showFaces([]);
			status({
				state: 'offline',
				access,
				message: 'Reconnecting. Your pending edits are retained.'
			});
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
		/** Show a cell: its tab, selected, scrolled into view. False if the tab is gone. */
		select(worksheet: string, cell: string): boolean {
			if (!model || !model.getters.getSheetIds().includes(worksheet)) return false;
			const active = model.getters.getActiveSheetId();
			if (active !== worksheet)
				model.dispatch('ACTIVATE_SHEET', { sheetIdFrom: active, sheetIdTo: worksheet });
			const { col, row } = helpers.toCartesian(cell);
			model.selection.selectCell(col, row);
			model.dispatch('SCROLL_TO_CELL', { col, row });
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
			if (commentRequest === comment) commentRequest = undefined;
			clearTimeout(timer);
			socket?.close();
			app?.destroy();
			target.remove();
			styles.remove();
		}
	};
}
