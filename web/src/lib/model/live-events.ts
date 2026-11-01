/** The same SSE frames over a socket on localhost HTTP/1. HTTPS keeps native EventSource.
 * A socket does not occupy the browser's six HTTP connections, even with many open tabs. */
export interface EventFrame {
	type: string;
	data: string;
}
export class EventFrames {
	private buffer = '';
	feed(text: string): EventFrame[] {
		this.buffer += text;
		const frames: EventFrame[] = [];
		let boundary: RegExpExecArray | null;
		while ((boundary = /\r?\n\r?\n/.exec(this.buffer))) {
			const block = this.buffer.slice(0, boundary.index);
			this.buffer = this.buffer.slice(boundary.index + boundary[0].length);
			let type = 'message';
			const data: string[] = [];
			for (const line of block.split(/\r?\n/)) {
				if (line.startsWith(':')) continue;
				const colon = line.indexOf(':');
				const field = colon < 0 ? line : line.slice(0, colon);
				const value = colon < 0 ? '' : line.slice(colon + 1).replace(/^ /, '');
				if (field === 'event') type = value;
				if (field === 'data') data.push(value);
			}
			if (data.length) frames.push({ type, data: data.join('\n') });
		}
		return frames;
	}
}
interface LiveEvents {
	onopen: ((event: Event) => void) | null;
	onerror: ((event: Event) => void) | null;
	addEventListener(type: string, listener: EventListener): void;
	close(): void;
}
class SocketEvents extends EventTarget implements LiveEvents {
	onopen: ((event: Event) => void) | null = null;
	onerror: ((event: Event) => void) | null = null;
	private socket: WebSocket | null = null;
	private timer: ReturnType<typeof setTimeout> | null = null;
	private stopped = false;
	private delay = 1000;
	private url: string;
	constructor(url: string) {
		super();
		this.url = url;
		this.connect();
	}
	private connect() {
		if (this.stopped) return;
		const frames = new EventFrames(),
			decoder = new TextDecoder();
		const socket = (this.socket = new WebSocket(this.url));
		socket.binaryType = 'arraybuffer';
		socket.onopen = () => {
			this.delay = 1000;
			this.onopen?.(new Event('open'));
		};
		socket.onmessage = (event) => {
			const text =
				typeof event.data === 'string'
					? event.data
					: decoder.decode(event.data as ArrayBuffer, { stream: true });
			for (const frame of frames.feed(text))
				this.dispatchEvent(new MessageEvent(frame.type, { data: frame.data }));
		};
		socket.onclose = () => {
			if (this.stopped) return;
			this.onerror?.(new Event('error'));
			// A terminal message callback can close the stream during onerror.
			if (this.stopped) return;
			this.timer = setTimeout(() => {
				this.timer = null;
				this.connect();
			}, this.delay);
			this.delay = Math.min(this.delay * 2, 15000);
		};
		socket.onerror = () => {}; // close owns one failure notification and one retry.
	}
	close() {
		this.stopped = true;
		if (this.timer) clearTimeout(this.timer);
		this.timer = null;
		this.socket?.close();
	}
}
/** Loopback names, `*.localhost` included (RFC 6761): the multiplayer smoke serves the cockpit on
 * `plane.localhost`, where native EventSource held one of six connections per stream and a page
 * with enough of them left a Room's own read queued behind them, so a message never showed. */
export function isLoopbackHost(hostname: string): boolean {
	return (
		['127.0.0.1', 'localhost', '[::1]'].includes(hostname) || hostname.endsWith('.localhost')
	);
}
export function openLiveEvents(path: string): LiveEvents {
	if (
		typeof location !== 'undefined' &&
		location.protocol === 'http:' &&
		isLoopbackHost(location.hostname) &&
		typeof WebSocket !== 'undefined'
	) {
		const url = new URL(path, location.origin);
		url.protocol = 'ws:';
		return new SocketEvents(url.href);
	}
	return new EventSource(path);
}
