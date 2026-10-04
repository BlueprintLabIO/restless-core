import { App, Component, xml } from '@odoo/owl';
import { Model, Spreadsheet } from '@odoo/o-spreadsheet/dist/o_spreadsheet.esm.js';
import templates from '@odoo/o-spreadsheet/dist/o_spreadsheet.xml?raw';
import spreadsheetCss from '@odoo/o-spreadsheet/dist/o_spreadsheet.css?inline';
import bootstrapCss from 'bootstrap/dist/css/bootstrap.min.css?inline';
import fontCss from 'font-awesome/css/font-awesome.min.css?inline';

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
}
type UpstreamMessage = Record<string, any>;
export function mountSheet(
	element: HTMLDivElement,
	company: string,
	sheet: string,
	status: (value: EditorStatus) => void
) {
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
			if (message.nextRevisionId) pending.set(message.nextRevisionId, message);
			if (socket?.readyState === WebSocket.OPEN && !blocked) socket.send(JSON.stringify(message));
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
		if (disposed || blocked) return;
		status({
			state: pending.size ? 'saving' : 'saved',
			access,
			revision: model?.exportData().revisionId,
			worksheet: model?.getters.getActiveSheetId()
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
		status({ state: 'error', message, access: 'read', denied });
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
		socket = new WebSocket(url);
		socket.onmessage = async (event) => {
			if (disposed || blocked) return;
			try {
				const envelope = JSON.parse(event.data);
				if (envelope.type === 'ERROR') {
					fail(envelope.message, String(envelope.message).includes('unavailable'));
					return;
				}
				// Other people's and agents' cursors: shown, never sequenced or stored.
				if (envelope.type === 'PRESENCE') {
					if (model) receive?.(envelope.message);
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
						const mounted = new App(Root, { props: { model }, templates });
						app = mounted;
						await mount(mounted);
						if (disposed || blocked) {
							mounted.destroy();
							return;
						}
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
		socket.onclose = async () => {
			if (disposed || blocked) return;
			model?.updateMode('readonly');
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
				if (disposed || blocked) return;
				if ([401, 403, 404].includes(response.status)) {
					fail('Access to this sheet has ended.', true);
					return;
				}
			} catch {
				/* A transient outage retains pending work for this identity. */
			}
			if (disposed || blocked) return;
			timer = setTimeout(connect, 1500);
		};
	}
	connect();
	return {
		getModel: () => model,
		destroy() {
			disposed = true;
			clearTimeout(timer);
			socket?.close();
			app?.destroy();
			target.remove();
			styles.remove();
		}
	};
}
