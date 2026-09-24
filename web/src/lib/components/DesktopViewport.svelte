<script lang="ts">
	import { onDestroy, onMount, tick, untrack } from 'svelte';
	import type { Snippet } from 'svelte';
	import Maximize2 from '@lucide/svelte/icons/maximize-2';
	import Minimize2 from '@lucide/svelte/icons/minimize-2';
	import Clipboard from '@lucide/svelte/icons/clipboard';
	import Scaling from '@lucide/svelte/icons/scaling';
	import Scan from '@lucide/svelte/icons/scan';

	type Rfb = {
		scaleViewport: boolean;
		resizeSession: boolean;
		viewOnly: boolean;
		background: string;
		qualityLevel: number;
		compressionLevel: number;
		addEventListener: (type: string, listener: EventListener) => void;
		removeEventListener: (type: string, listener: EventListener) => void;
		disconnect: () => void;
		clipboardPasteFrom: (text: string) => void;
	};
	type RfbConstructor = new (target: HTMLElement, url: string) => Rfb;
	type Status = 'connecting' | 'connected' | 'reconnecting' | 'error';

	let {
		src = '',
		title = 'Company computer',
		offline = null,
		onload = null,
		onactivity = null,
		onstatus = null
	}: {
		src?: string;
		title?: string;
		offline?: Snippet | null;
		onload?: (() => void) | null;
		/** A real pointer/key event observed inside the live desktop. */
		onactivity?: (() => void) | null;
		onstatus?: ((status: Status, message?: string) => void) | null;
	} = $props();

	let target = $state<HTMLDivElement>();
	let screen = $state<HTMLDivElement>();
	let rfb = $state<Rfb | null>(null);
	let status = $state<Status>('connecting');
	let message = $state('Connecting to the company computer…');
	let remoteClipboard = $state('');
	let fit = $state(true);
	let fullscreen = $state(false);
	let utilityMessage = $state('');
	let clipboardPanelOpen = $state(false);
	let clipboardDraft = $state('');
	let clipboardFeedback = $state('');
	let clipboardInput = $state<HTMLTextAreaElement>();
	let generation = 0;
	let retryTimer: ReturnType<typeof setTimeout> | undefined;
	let active = true;
	let resolvedClientUrl = $state('');
	let retryCount = 0;

	function report(next: Status, detail = '') {
		status = next;
		message = detail || (next === 'connected' ? '' : 'Connecting to the company computer…');
		onstatus?.(next, detail);
	}

	async function connect(current: number, route: string, display: HTMLDivElement) {
		if (!route || !active || current !== generation) return;
		report(untrack(() => (status === 'error' ? 'reconnecting' : 'connecting')));
		try {
			// Resolve the one-use ticket once. Reconnects reuse the authorized
			// same-origin client route and its server-issued WebSocket lease path.
			if (!resolvedClientUrl) {
				const response = await fetch(route, { credentials: 'same-origin', redirect: 'follow' });
				if (!active || current !== generation) return;
				if (!response.ok)
					throw new Error(`Desktop connection returned ${response.status}. Reattach to continue.`);
				resolvedClientUrl = response.url;
			}
			const clientUrl = new URL(resolvedClientUrl);
			if (clientUrl.origin !== location.origin || !clientUrl.pathname.endsWith('/vnc.html')) {
				throw new Error('The desktop connection did not return a valid client route.');
			}
			const query = clientUrl.searchParams;
			const path = query.get('path');
			if (!path) throw new Error('The desktop connection route is missing its WebSocket path.');
			const socketUrl = new URL(`/${path}`, clientUrl.origin);
			socketUrl.protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
			const RFB = (
				await import(
					/* @vite-ignore */ `${clientUrl.origin}${clientUrl.pathname.replace(/\/vnc\.html$/, '/core/rfb.js')}`
				)
			).default as RfbConstructor;
			if (!active || current !== generation) return;
			const client = new RFB(display, socketUrl.href);
			rfb = client;
			const readOnly = query.get('view_only') === '1';
			fit = readOnly || query.get('resize') === 'scale';
			client.viewOnly = readOnly;
			client.resizeSession = !readOnly && query.get('resize') === 'remote';
			client.scaleViewport = fit;
			client.background = '#101217';
			client.qualityLevel = 9;
			client.compressionLevel = 2;
			client.addEventListener('connect', () => {
				if (current !== generation) return;
				retryCount = 0;
				report('connected');
				onload?.();
			});
			client.addEventListener('clipboard', ((event: CustomEvent<{ text?: string }>) => {
				remoteClipboard = event.detail?.text ?? '';
			}) as EventListener);
			client.addEventListener('disconnect', (() => {
				if (current !== generation || !active) return;
				untrack(() => {
					client.disconnect();
					if (rfb === client) rfb = null;
				});
				retryCount += 1;
				if (retryCount > 5) {
					report('error', 'Could not reconnect. Reattach to continue.');
					return;
				}
				report('reconnecting', 'Connection interrupted. Reconnecting…');
				retryTimer = setTimeout(
					() => void connect(current, route, display),
					Math.min(1000 * 2 ** (retryCount - 1), 8000)
				);
			}) as EventListener);
		} catch (cause) {
			if (current !== generation || !active) return;
			report('error', cause instanceof Error ? cause.message : 'The desktop could not be opened.');
		}
	}

	$effect(() => {
		const route = src;
		const display = screen;
		if (!route || !display) return;
		const current = ++generation;
		resolvedClientUrl = '';
		retryCount = 0;
		display.tabIndex = 0;
		const captureActivity = (event: Event) => {
			if (event instanceof KeyboardEvent && event.key === 'Escape' && event.shiftKey) {
				event.preventDefault();
				event.stopPropagation();
				(target?.querySelector('.desktop-tools button') as HTMLButtonElement | null)?.focus();
				return;
			}
			recordActivity();
		};
		const inputEvents = ['pointerdown', 'pointermove', 'wheel', 'keydown', 'keyup'] as const;
		for (const name of inputEvents) display.addEventListener(name, captureActivity, true);
		untrack(() => {
			clearTimeout(retryTimer);
			rfb?.disconnect();
			rfb = null;
			remoteClipboard = '';
		});
		untrack(() => void connect(current, route, display));
		return () => {
			if (current === generation) generation += 1;
			for (const name of inputEvents) display.removeEventListener(name, captureActivity, true);
			clearTimeout(retryTimer);
			untrack(() => {
				rfb?.disconnect();
				rfb = null;
			});
		};
	});

	function recordActivity() {
		onactivity?.();
	}

	function toggleFit() {
		fit = !fit;
		if (rfb) rfb.scaleViewport = fit;
	}

	async function toggleFullscreen() {
		if (!target) return;
		try {
			if (document.fullscreenElement === target) await document.exitFullscreen();
			else await target.requestFullscreen();
		} catch {
			utilityMessage = 'Fullscreen was blocked by the browser.';
		}
	}

	async function openClipboardPanel() {
		clipboardPanelOpen = true;
		clipboardFeedback = '';
		await tick();
		clipboardInput?.focus();
	}

	function closeClipboardPanel() {
		clipboardPanelOpen = false;
		clipboardFeedback = '';
	}

	function handleClipboardPanelKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			closeClipboardPanel();
		}
	}

	async function readLocalClipboard() {
		if (!navigator.clipboard?.readText) {
			clipboardFeedback =
				'This browser does not allow clipboard reading here. Paste into the text box instead.';
			return;
		}
		try {
			clipboardDraft = await navigator.clipboard.readText();
			clipboardFeedback = 'Local clipboard loaded into the text box.';
		} catch {
			clipboardFeedback = 'Browser clipboard access was denied. Paste into the text box instead.';
		}
	}

	async function copyRemoteClipboard() {
		if (!remoteClipboard) {
			clipboardFeedback = 'There is no remote clipboard text yet.';
			return;
		}
		if (!navigator.clipboard?.writeText) {
			clipboardFeedback = 'This browser does not allow clipboard writing here.';
			return;
		}
		try {
			await navigator.clipboard.writeText(remoteClipboard);
			clipboardFeedback = 'Remote clipboard copied to this device.';
		} catch {
			clipboardFeedback =
				'Browser clipboard access was denied. Select and copy the text below instead.';
		}
	}

	function sendClipboard() {
		if (!clipboardDraft.trim()) {
			clipboardFeedback = 'Enter or paste text before sending.';
			return;
		}
		if (!rfb || status !== 'connected' || rfb.viewOnly) {
			clipboardFeedback = 'Take control of the connected computer before sending clipboard text.';
			return;
		}
		rfb.clipboardPasteFrom(clipboardDraft);
		onactivity?.();
		clipboardFeedback = 'Clipboard sent. Press Ctrl+V in the company app.';
	}

	function retryConnection() {
		if (!screen || !src) return;
		retryCount = 0;
		const current = generation;
		void connect(current, src, screen);
	}

	onMount(() => {
		const syncFullscreen = () => (fullscreen = document.fullscreenElement === target);
		document.addEventListener('fullscreenchange', syncFullscreen);
		return () => document.removeEventListener('fullscreenchange', syncFullscreen);
	});

	onDestroy(() => {
		active = false;
		generation += 1;
		clearTimeout(retryTimer);
		rfb?.disconnect();
	});
</script>

<div class="desktop-viewport" bind:this={target} aria-label={title}>
	{#if src}
		<div bind:this={screen} class="desktop-screen" aria-label={title}></div>
		<div class="desktop-tools" aria-label="Desktop controls">
			<span class="desktop-live" role="status">
				{status === 'connected'
					? 'Connected'
					: status === 'reconnecting'
						? 'Reconnecting'
						: status === 'error'
							? 'Connection needs attention'
							: 'Connecting'}
			</span>
			<button
				type="button"
				title="Open clipboard panel"
				aria-label="Open clipboard panel"
				aria-expanded={clipboardPanelOpen}
				onclick={openClipboardPanel}
			>
				<Clipboard size={14} />
			</button>
			<button
				type="button"
				title={fit ? 'Actual size' : 'Fit to screen'}
				aria-label={fit ? 'Actual size' : 'Fit to screen'}
				onclick={toggleFit}
			>
				{#if fit}<Scan size={14} />{:else}<Scaling size={14} />{/if}
			</button>
			<button
				type="button"
				title="Toggle fullscreen"
				aria-label="Toggle fullscreen"
				onclick={toggleFullscreen}
			>
				{#if fullscreen}<Minimize2 size={14} />{:else}<Maximize2 size={14} />{/if}
			</button>
		</div>
		{#if clipboardPanelOpen}
			<div
				class="clipboard-panel"
				role="dialog"
				aria-modal="false"
				aria-labelledby="clipboard-panel-title"
				onkeydown={handleClipboardPanelKeydown}
			>
				<div class="clipboard-panel-heading">
					<h2 id="clipboard-panel-title">Clipboard</h2>
					<button
						type="button"
						aria-label="Close clipboard panel"
						title="Close"
						onclick={closeClipboardPanel}>×</button
					>
				</div>
				<label for="desktop-clipboard-draft">Text to send to the company computer</label>
				<textarea
					bind:this={clipboardInput}
					bind:value={clipboardDraft}
					id="desktop-clipboard-draft"
					rows="5"
					placeholder="Type or paste text here"></textarea>
				<div class="clipboard-panel-actions">
					<button type="button" onclick={readLocalClipboard}>Read local clipboard</button>
					<button type="button" onclick={copyRemoteClipboard} disabled={!remoteClipboard}
						>Copy remote clipboard</button
					>
					<button
						type="button"
						class="clipboard-send"
						onclick={sendClipboard}
						disabled={!rfb || status !== 'connected' || !!rfb?.viewOnly}>Send to computer</button
					>
				</div>
				{#if remoteClipboard}
					<details class="clipboard-remote-text">
						<summary>Remote clipboard text</summary>
						<pre>{remoteClipboard}</pre>
					</details>
				{/if}
				<p class="clipboard-feedback" aria-live="polite">
					{clipboardFeedback || 'After sending, press Ctrl+V in the company app.'}
				</p>
			</div>
		{/if}
		{#if status !== 'connected'}
			<div class="desktop-status" role={status === 'error' ? 'alert' : 'status'}>
				<span>{message || 'Connecting…'}</span>
				{#if status === 'error' && resolvedClientUrl}
					<button type="button" onclick={retryConnection}>Retry</button>
				{/if}
			</div>
		{/if}
		{#if utilityMessage}<div class="desktop-utility-message" role="status">
				{utilityMessage}
			</div>{/if}
	{:else if offline}
		{@render offline()}
	{:else}
		<div class="desktop-viewport-empty" role="status">No live desktop is attached.</div>
	{/if}
</div>

<style>
	.desktop-viewport {
		position: relative;
		min-width: 0;
		min-height: 0;
		width: 100%;
		height: 100%;
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		background: #101217;
	}
	.desktop-viewport:fullscreen {
		width: 100vw;
		height: 100vh;
	}
	.desktop-screen {
		width: 100%;
		min-width: 0;
		min-height: 0;
		flex: 1 1 auto;
		overflow: auto;
		outline: none;
	}
	.desktop-screen :global(canvas) {
		display: block;
	}
	.desktop-tools {
		position: relative;
		z-index: 2;
		display: flex;
		align-items: center;
		gap: 4px;
		min-height: 40px;
		padding: 4px 88px 4px 8px;
		border-top: 1px solid rgba(255, 255, 255, 0.12);
		background: rgba(16, 18, 23, 0.96);
	}
	.desktop-viewport:fullscreen > .desktop-tools {
		padding-right: 8px;
	}
	.desktop-live {
		margin-right: auto;
		padding-inline: 8px;
		color: rgba(255, 255, 255, 0.72);
		font: var(--t-label) var(--font-ui);
	}
	.clipboard-panel {
		position: absolute;
		z-index: 4;
		right: 88px;
		bottom: 48px;
		width: min(380px, calc(100% - 24px));
		max-height: min(70%, 520px);
		overflow: auto;
		padding: 16px;
		border: 1px solid rgba(255, 255, 255, 0.16);
		border-radius: 10px;
		color: var(--ink, rgba(255, 255, 255, 0.92));
		background: var(--surface-raised, #191c23);
		box-shadow: 0 12px 40px rgba(0, 0, 0, 0.42);
		font: var(--t-body) var(--font-ui);
	}
	.desktop-viewport:fullscreen > .clipboard-panel {
		right: 12px;
	}
	.clipboard-panel-heading {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 12px;
	}
	.clipboard-panel-heading h2 {
		margin: 0;
		font: 600 var(--t-head) var(--font-ui);
	}
	.clipboard-panel label {
		display: block;
		margin-bottom: 6px;
		font: var(--t-label) var(--font-ui);
	}
	.clipboard-panel textarea {
		display: block;
		width: 100%;
		min-height: 104px;
		resize: vertical;
		padding: 9px 10px;
		border: 1px solid rgba(255, 255, 255, 0.2);
		border-radius: 7px;
		color: inherit;
		background: rgba(0, 0, 0, 0.22);
		font: var(--t-body) var(--font-ui);
	}
	.clipboard-panel-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-top: 9px;
	}
	.clipboard-panel button {
		min-height: 32px;
		padding: 5px 9px;
		border: 1px solid rgba(255, 255, 255, 0.18);
		border-radius: 6px;
		color: inherit;
		background: rgba(255, 255, 255, 0.06);
		font: var(--t-label) var(--font-ui);
		cursor: pointer;
	}
	.clipboard-panel button:disabled {
		opacity: 0.48;
		cursor: not-allowed;
	}
	.clipboard-panel .clipboard-send {
		margin-left: auto;
		color: var(--button-primary-ink, white);
		background: var(--button-primary, #345e92);
	}
	.clipboard-remote-text {
		margin-top: 10px;
	}
	.clipboard-remote-text summary {
		cursor: pointer;
		font: var(--t-label) var(--font-ui);
	}
	.clipboard-remote-text pre {
		max-height: 120px;
		overflow: auto;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font: var(--t-label) var(--font-mono);
	}
	.clipboard-feedback {
		margin: 10px 0 0;
		color: var(--ink-muted, rgba(255, 255, 255, 0.72));
		font: var(--t-label) var(--font-ui);
	}
	.desktop-tools button {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		border: 0;
		border-radius: 6px;
		color: rgba(255, 255, 255, 0.86);
		background: transparent;
		cursor: pointer;
	}
	.desktop-tools button:hover,
	.desktop-tools button:focus-visible {
		color: white;
		background: rgba(255, 255, 255, 0.12);
		outline: none;
	}
	.desktop-status {
		position: absolute;
		inset: 12px 12px auto;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		width: fit-content;
		max-width: calc(100% - 24px);
		margin: 0 auto;
		padding: 8px 12px;
		border: 1px solid rgba(255, 255, 255, 0.14);
		border-radius: 8px;
		color: rgba(255, 255, 255, 0.88);
		background: rgba(16, 18, 23, 0.9);
		font: var(--t-label) var(--font-ui);
	}
	.desktop-status button {
		border: 0;
		color: inherit;
		background: transparent;
		text-decoration: underline;
		cursor: pointer;
		font: inherit;
	}
	.desktop-utility-message {
		position: absolute;
		z-index: 3;
		left: 50%;
		bottom: 48px;
		transform: translateX(-50%);
		padding: 7px 10px;
		border: 1px solid rgba(255, 255, 255, 0.14);
		border-radius: 8px;
		color: rgba(255, 255, 255, 0.88);
		background: rgba(16, 18, 23, 0.9);
		font: var(--t-label) var(--font-ui);
	}
	@media (max-width: 600px) {
		.clipboard-panel {
			right: 12px;
		}
	}
	.desktop-viewport-empty {
		margin: auto;
		color: rgba(255, 255, 255, 0.72);
	}
</style>
