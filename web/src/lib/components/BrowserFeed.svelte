<script lang="ts">
	/* The company browser as Chrome's own screen feed (S104 F4): the tab the company is on, sharp
	 * at any size. Input goes to the page only while this tab holds control; the first press asks
	 * for control, as the desktop does. The whole desktop stays one switch away. */
	import { onDestroy } from 'svelte';

	let {
		companyId,
		clientId,
		interactive = false,
		onclaim = null,
		onactivity = null
	}: {
		companyId: string;
		clientId: string;
		interactive?: boolean;
		onclaim?: (() => Promise<boolean>) | null;
		onactivity?: (() => void) | null;
	} = $props();

	let frame = $state('');
	let size = $state({ width: 0, height: 0 });
	let pageUrl = $state('');
	let status = $state<'connecting' | 'live' | 'lost'>('connecting');
	let image = $state<HTMLImageElement>();
	let surface = $state<HTMLDivElement>();
	let socket: WebSocket | null = null;
	let retry: ReturnType<typeof setTimeout> | undefined;
	let closed = false;
	let claiming = false;
	let lastMove = 0;

	function connect() {
		if (closed || !clientId) return;
		const scheme = location.protocol === 'https:' ? 'wss' : 'ws';
		const url = `${scheme}://${location.host}/desktop/${encodeURIComponent(companyId)}/screencast?client_id=${encodeURIComponent(clientId)}`;
		const next = new WebSocket(url);
		next.binaryType = 'blob';
		socket = next;
		status = 'connecting';
		next.onmessage = (event) => {
			if (typeof event.data === 'string') {
				const note = JSON.parse(event.data) as {
					type: string;
					width?: number;
					height?: number;
					url?: string;
				};
				if (note.type === 'size') size = { width: note.width ?? 0, height: note.height ?? 0 };
				if (note.type === 'page') pageUrl = note.url ?? '';
				return;
			}
			const previous = frame;
			frame = URL.createObjectURL(event.data as Blob);
			if (previous) URL.revokeObjectURL(previous);
			status = 'live';
		};
		next.onclose = () => {
			if (socket !== next) return;
			socket = null;
			status = 'lost';
			if (!closed) retry = setTimeout(connect, 1500);
		};
	}

	$effect(() => {
		if (!clientId) return;
		closed = false;
		connect();
		return () => {
			closed = true;
			clearTimeout(retry);
			socket?.close();
			socket = null;
		};
	});

	onDestroy(() => {
		if (frame) URL.revokeObjectURL(frame);
	});

	/* Where a viewer point lands on the page, in the page's CSS pixels. The image keeps its
	 * aspect ratio inside its box, so the letterbox is taken out first. */
	function pagePoint(event: MouseEvent): { x: number; y: number } | null {
		if (!image || !size.width || !size.height) return null;
		const box = image.getBoundingClientRect();
		const scale = Math.min(box.width / size.width, box.height / size.height);
		const left = box.left + (box.width - size.width * scale) / 2;
		const top = box.top + (box.height - size.height * scale) / 2;
		const x = (event.clientX - left) / scale;
		const y = (event.clientY - top) / scale;
		if (x < 0 || y < 0 || x > size.width || y > size.height) return null;
		return { x, y };
	}

	const modifiers = (event: MouseEvent | KeyboardEvent) =>
		(event.altKey ? 1 : 0) |
		(event.ctrlKey ? 2 : 0) |
		(event.metaKey ? 4 : 0) |
		(event.shiftKey ? 8 : 0);
	const buttonName = (button: number) =>
		button === 1 ? 'middle' : button === 2 ? 'right' : 'left';

	function send(input: Record<string, unknown>) {
		if (socket?.readyState !== WebSocket.OPEN) return;
		socket.send(JSON.stringify(input));
		onactivity?.();
	}

	async function mayAct(): Promise<boolean> {
		if (interactive) return true;
		if (!onclaim || claiming) return false;
		claiming = true;
		try {
			return await onclaim();
		} finally {
			claiming = false;
		}
	}

	async function pointerDown(event: PointerEvent) {
		surface?.focus();
		const point = pagePoint(event);
		if (!point) return;
		event.preventDefault();
		const held = interactive;
		if (!(await mayAct())) return;
		const base = {
			type: 'mouse',
			...point,
			button: buttonName(event.button),
			clicks: Math.max(1, event.detail),
			modifiers: modifiers(event)
		};
		send({ ...base, event: 'down' });
		// The press that took control ends as a whole click: its release came while asking.
		if (!held) send({ ...base, event: 'up' });
	}

	function pointerUp(event: PointerEvent) {
		const point = pagePoint(event);
		if (!point || !interactive) return;
		send({
			type: 'mouse',
			event: 'up',
			...point,
			button: buttonName(event.button),
			clicks: Math.max(1, event.detail),
			modifiers: modifiers(event)
		});
	}

	function pointerMove(event: PointerEvent) {
		if (!interactive || Date.now() - lastMove < 40) return;
		const point = pagePoint(event);
		if (!point) return;
		lastMove = Date.now();
		send({
			type: 'mouse',
			event: 'move',
			...point,
			button: event.buttons ? 'left' : 'none',
			modifiers: modifiers(event)
		});
	}

	function wheel(event: WheelEvent) {
		if (!interactive) return;
		const point = pagePoint(event);
		if (!point) return;
		event.preventDefault();
		send({ type: 'wheel', ...point, dx: event.deltaX, dy: event.deltaY });
	}

	async function key(event: KeyboardEvent, phase: 'down' | 'up') {
		if (phase === 'down' && !(await mayAct())) return;
		if (!interactive) return;
		event.preventDefault();
		send({
			type: 'key',
			event: phase,
			key: event.key,
			code: event.code,
			key_code: event.keyCode,
			text: event.key.length === 1 && !event.ctrlKey && !event.metaKey ? event.key : null,
			modifiers: modifiers(event)
		});
	}
</script>

<!-- A remote browser surface: it takes the pointer and keys and forwards them while this tab controls. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
	class="browser-feed"
	class:interactive
	bind:this={surface}
	tabindex="0"
	role="application"
	aria-label="Company browser"
	onpointerdown={pointerDown}
	onpointerup={pointerUp}
	onpointermove={pointerMove}
	onwheel={wheel}
	onkeydown={(event) => void key(event, 'down')}
	onkeyup={(event) => void key(event, 'up')}
	oncontextmenu={(event) => event.preventDefault()}
>
	{#if frame}
		<img
			bind:this={image}
			src={frame}
			alt={pageUrl ? `The company browser on ${pageUrl}` : 'The company browser'}
			draggable="false"
		/>
	{/if}
	{#if status !== 'live'}
		<p class="browser-feed-state" role="status">
			{status === 'lost' ? 'Reconnecting to the company browser…' : 'Opening the company browser…'}
		</p>
	{:else if !interactive}
		<p class="browser-feed-hint">Click to take control</p>
	{/if}
</div>

<style>
	.browser-feed {
		position: relative;
		display: grid;
		place-items: center;
		width: 100%;
		height: 100%;
		min-height: 0;
		overflow: hidden;
		background: var(--surface-sunken, var(--surface));
		outline: none;
		cursor: default;
		touch-action: none;
	}
	.browser-feed:focus-visible {
		box-shadow: inset 0 0 0 2px var(--focus-ring, var(--primary));
	}
	/* The frame fills the space it is given, scaled up as well as down (owner, 23 Oct: a 1280-wide
	 * tab sat small in the middle of a large window, since max-width only ever shrinks). Its
	 * proportions are kept; pagePoint maps pointer positions through the same letterbox. */
	.browser-feed img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		user-select: none;
		-webkit-user-drag: none;
	}
	.browser-feed-state,
	.browser-feed-hint {
		position: absolute;
		margin: 0;
		padding: 6px 12px;
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--surface-raised) 92%, transparent);
		box-shadow: var(--shadow-soft);
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.browser-feed-hint {
		bottom: 14px;
		pointer-events: none;
	}
</style>
