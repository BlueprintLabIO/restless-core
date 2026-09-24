<script lang="ts">
	import { onDestroy, onMount, tick, untrack } from 'svelte';
	import type { Snippet } from 'svelte';
	import Maximize2 from '@lucide/svelte/icons/maximize-2';
	import Minimize2 from '@lucide/svelte/icons/minimize-2';
	import Clipboard from '@lucide/svelte/icons/clipboard';

	type Rfb = {
		scaleViewport: boolean;
		resizeSession: boolean;
		viewOnly: boolean;
		_supportsSetDesktopSize?: boolean;
		_screenSize?: () => { w: number; h: number };
		_requestRemoteResize?: () => void;
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
	type BufferedInput = { event: Event; target: EventTarget | null };
	type ClipboardClip = { id: number; source: 'device' | 'manual' | 'computer'; text: string };
	const MAX_CLIPS = 8;
	const MAX_CLIP_LENGTH = 64000;
	const MAX_DESKTOP_FRAMEBUFFER = { width: 3840, height: 2160 };

	let {
		src = '',
		title = 'Company computer',
		offline = null,
		onload = null,
		onactivity = null,
		onclaim = null,
		interactive = false,
		canResize = false,
		ondisplayresize = null,
		onstatus = null
	}: {
		src?: string;
		title?: string;
		offline?: Snippet | null;
		onload?: (() => void) | null;
		/** A real pointer/key event observed inside the live desktop. */
		onactivity?: (() => void) | null;
		/** Called after the viewer's first deliberate input. Resolves true when the lease is granted. */
		onclaim?: (() => Promise<boolean>) | null;
		interactive?: boolean;
		canResize?: boolean;
		ondisplayresize?: ((width: number, height: number) => void) | null;
		onstatus?: ((status: Status, message?: string) => void) | null;
	} = $props();

	let target = $state<HTMLDivElement>();
	let screen = $state<HTMLDivElement>();
	let rfb = $state<Rfb | null>(null);
	let status = $state<Status>('connecting');
	let message = $state('Connecting to the company computer…');
	let clipboardClips = $state<ClipboardClip[]>([]);
	let selectedClipId = $state<number | null>(null);
	let nextClipId = 0;
	let fullscreen = $state(false);
	let utilityMessage = $state('');
	let clipboardPanelOpen = $state(false);
	let manualClipboardOpen = $state(false);
	let clipboardDraft = $state('');
	let clipboardFeedback = $state('');
	let clipboardPanel = $state<HTMLDivElement>();
	let clipboardInput = $state<HTMLTextAreaElement>();
	let clipboardAction = $state<HTMLButtonElement>();
	let clipboardOpener = $state<HTMLButtonElement>();
	let generation = 0;
	let retryTimer: ReturnType<typeof setTimeout> | undefined;
	let active = true;
	let resolvedClientUrl = $state('');
	let retryCount = 0;
	let resizeObserver: ResizeObserver | undefined;
	let displayLeaseTimer: ReturnType<typeof setInterval> | undefined;
	let pixelRatioQuery: MediaQueryList | undefined;
	let pendingInput: BufferedInput[] = [];
	let claiming = false;
	let lastReportedSize = '';
	let resizeFrame = 0;
	let physicalResize = $state(false);
	let reportDisplaySize: (() => void) | undefined;

	function report(next: Status, detail = '') {
		status = next;
		message = detail || (next === 'connected' ? '' : 'Connecting to the company computer…');
		onstatus?.(next, detail);
	}

	function rememberClip(
		source: ClipboardClip['source'],
		text: string,
		select = true
	): ClipboardClip | null {
		if (text.length > MAX_CLIP_LENGTH) {
			clipboardFeedback = `That item is over the ${MAX_CLIP_LENGTH.toLocaleString()} character limit, so it was skipped.`;
			return null;
		}
		const value = text;
		if (!value.trim()) return null;
		const existing = clipboardClips.find((item) => item.text === value);
		if (existing) {
			if (select) selectedClipId = existing.id;
			return existing;
		}
		const clip = { id: ++nextClipId, source, text: value };
		clipboardClips = [clip, ...clipboardClips.filter((item) => item.text !== value)].slice(
			0,
			MAX_CLIPS
		);
		if (select) selectedClipId = clip.id;
		return clip;
	}

	function clearClipboardSession() {
		clipboardClips = [];
		selectedClipId = null;
		clipboardDraft = '';
	}

	function clearRecentClips() {
		clipboardClips = [];
		selectedClipId = null;
		clipboardFeedback = 'Recent items cleared.';
	}

	function physicalDisplaySize(element: HTMLElement): { width: number; height: number } {
		const rect = element.getBoundingClientRect();
		const scale = Math.max(0.25, window.devicePixelRatio || 1);
		const width = Math.max(1, Math.round(rect.width * scale));
		const height = Math.max(1, Math.round(rect.height * scale));
		const limit = Math.min(
			1,
			MAX_DESKTOP_FRAMEBUFFER.width / width,
			MAX_DESKTOP_FRAMEBUFFER.height / height
		);
		return {
			width: Math.max(1, Math.floor(width * limit)),
			height: Math.max(1, Math.floor(height * limit))
		};
	}

	function usePhysicalPixelsForResize(client: Rfb, element: HTMLElement): boolean {
		const requestResize = client._requestRemoteResize;
		const screenSize = client._screenSize;
		if (
			typeof requestResize !== 'function' ||
			typeof screenSize !== 'function' ||
			typeof client._supportsSetDesktopSize !== 'boolean'
		)
			return false;

		// noVNC's built-in SetDesktopSize path otherwise requests CSS pixels.
		// Override geometry only while that request is being assembled, so its
		// ordinary canvas scaling and pointer-coordinate mapping stay in CSS pixels.
		client._requestRemoteResize = () => {
			if (!client.resizeSession || client.viewOnly || !client._supportsSetDesktopSize) {
				requestResize.call(client);
				return;
			}
			const currentScreenSize = client._screenSize;
			const physical = physicalDisplaySize(element);
			client._screenSize = () => ({ w: physical.width, h: physical.height });
			try {
				requestResize.call(client);
			} finally {
				if (currentScreenSize) client._screenSize = currentScreenSize;
				else delete client._screenSize;
			}
		};
		return true;
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
			physicalResize = usePhysicalPixelsForResize(client, display);
			if (physicalResize) {
				lastReportedSize = '';
				reportDisplaySize?.();
			}
			// The observer connection is stable. Control changes only flip this
			// client-side gate after the server grants the tab's live lease.
			client.viewOnly = !(interactive || canResize);
			client.resizeSession = interactive || canResize;
			client.scaleViewport = true;
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
				const text = event.detail?.text ?? '';
				if (current === generation && text && !clipboardClips.some((clip) => clip.text === text)) {
					rememberClip('computer', text, !clipboardPanelOpen || selectedClipId === null);
				}
			}) as EventListener);
			client.addEventListener('disconnect', (() => {
				if (current !== generation || !active) return;
				clearClipboardSession();
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
			if (!rfb || status !== 'connected' || interactive || !onclaim) return;
			if (!claiming && !isDesktopInput(event)) return;
			event.preventDefault();
			event.stopImmediatePropagation();
			if (event instanceof PointerEvent && event.type === 'pointerdown') {
				try {
					(event.target as Element).setPointerCapture(event.pointerId);
				} catch {
					// Pointer capture is only an assist; the lease request still proceeds.
				}
			}
			if (event.type !== 'pointermove')
				pendingInput.push({ event: copyInputEvent(event), target: event.target });
			if (pendingInput.length > 40) pendingInput.splice(0, pendingInput.length - 40);
			if (!claiming) void claimAndReplay(display);
		};
		const inputEvents = [
			'pointerdown',
			'pointerup',
			'pointermove',
			'wheel',
			'keydown',
			'keyup'
		] as const;
		for (const name of inputEvents) display.addEventListener(name, captureActivity, true);
		untrack(() => {
			clearTimeout(retryTimer);
			rfb?.disconnect();
			rfb = null;
			clearClipboardSession();
		});
		untrack(() => void connect(current, route, display));
		return () => {
			if (current === generation) generation += 1;
			for (const name of inputEvents) display.removeEventListener(name, captureActivity, true);
			clearTimeout(retryTimer);
			untrack(() => {
				rfb?.disconnect();
				rfb = null;
				clearClipboardSession();
			});
		};
	});

	$effect(() => {
		const client = rfb;
		const mayInteract = interactive;
		const mayResize = canResize;
		if (client) {
			client.viewOnly = !(mayInteract || mayResize);
			client.resizeSession = mayInteract || mayResize;
		}
	});

	$effect(() => {
		const element = screen;
		if (!element || !ondisplayresize) return;
		const reportSize = () => {
			cancelAnimationFrame(resizeFrame);
			resizeFrame = requestAnimationFrame(() => {
				const rect = element.getBoundingClientRect();
				const physical = physicalResize ? physicalDisplaySize(element) : undefined;
				const width = physical?.width ?? Math.max(1, Math.round(rect.width));
				const height = physical?.height ?? Math.max(1, Math.round(rect.height));
				const signature = `${width}x${height}`;
				if (signature === lastReportedSize || document.visibilityState !== 'visible') return;
				lastReportedSize = signature;
				if (rfb) rfb.scaleViewport = true;
				ondisplayresize?.(width, height);
			});
		};
		reportDisplaySize = reportSize;
		resizeObserver = new ResizeObserver(reportSize);
		resizeObserver.observe(element);
		const onPixelRatioChange = () => {
			lastReportedSize = '';
			reportSize();
			// A monitor move or browser zoom can change DPR without changing the
			// element's CSS size, so ask noVNC for a new framebuffer explicitly.
			if (rfb?._supportsSetDesktopSize) rfb._requestRemoteResize?.();
			watchPixelRatio();
		};
		const watchPixelRatio = () => {
			pixelRatioQuery?.removeEventListener('change', onPixelRatioChange);
			pixelRatioQuery = window.matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`);
			pixelRatioQuery.addEventListener('change', onPixelRatioChange, { once: true });
		};
		watchPixelRatio();
		displayLeaseTimer = setInterval(() => {
			if (document.visibilityState === 'visible') {
				lastReportedSize = '';
				reportSize();
			}
		}, 15_000);
		const onVisibility = () => {
			if (document.visibilityState === 'visible') {
				lastReportedSize = '';
				reportSize();
			}
		};
		document.addEventListener('visibilitychange', onVisibility);
		return () => {
			resizeObserver?.disconnect();
			resizeObserver = undefined;
			if (displayLeaseTimer) clearInterval(displayLeaseTimer);
			displayLeaseTimer = undefined;
			document.removeEventListener('visibilitychange', onVisibility);
			pixelRatioQuery?.removeEventListener('change', onPixelRatioChange);
			pixelRatioQuery = undefined;
			if (reportDisplaySize === reportSize) reportDisplaySize = undefined;
			cancelAnimationFrame(resizeFrame);
		};
	});

	function isDesktopInput(event: Event): boolean {
		return (
			(event instanceof PointerEvent && event.type === 'pointerdown') ||
			event instanceof WheelEvent ||
			(event instanceof KeyboardEvent && event.type === 'keydown' && event.key !== 'Escape')
		);
	}

	function copyInputEvent(event: Event): Event {
		if (event instanceof PointerEvent) {
			return new PointerEvent(event.type, {
				bubbles: true,
				cancelable: true,
				composed: true,
				pointerId: event.pointerId,
				pointerType: event.pointerType,
				isPrimary: event.isPrimary,
				button: event.button,
				buttons: event.buttons,
				clientX: event.clientX,
				clientY: event.clientY,
				ctrlKey: event.ctrlKey,
				shiftKey: event.shiftKey,
				altKey: event.altKey,
				metaKey: event.metaKey
			});
		}
		if (event instanceof WheelEvent) {
			return new WheelEvent(event.type, {
				bubbles: true,
				cancelable: true,
				clientX: event.clientX,
				clientY: event.clientY,
				deltaX: event.deltaX,
				deltaY: event.deltaY,
				deltaZ: event.deltaZ,
				deltaMode: event.deltaMode
			});
		}
		if (event instanceof KeyboardEvent) {
			return new KeyboardEvent(event.type, {
				bubbles: true,
				cancelable: true,
				composed: true,
				key: event.key,
				code: event.code,
				location: event.location,
				repeat: event.repeat,
				ctrlKey: event.ctrlKey,
				shiftKey: event.shiftKey,
				altKey: event.altKey,
				metaKey: event.metaKey
			});
		}
		return event;
	}

	async function claimAndReplay(display: HTMLDivElement) {
		if (claiming || !onclaim) return;
		claiming = true;
		try {
			const granted = await onclaim();
			await tick();
			if (!granted || !active || !rfb || !interactive) {
				pendingInput = [];
				return;
			}
			// Replay against the same live canvas after authorization. Coordinates
			// remain viewport-relative and a detached/replaced canvas is discarded.
			for (const input of pendingInput.splice(0)) {
				const destination = input.target as HTMLElement | null;
				if (!destination?.isConnected) continue;
				if (input.event instanceof PointerEvent) {
					// noVNC listens for mouse events. A cancelled pointerdown does not
					// produce compatibility mouse events, and redispatching a pointer
					// event does not recreate them. Complete the first click as a pair
					// so a slow lease response cannot leave the remote button held.
					if (input.event.type !== 'pointerdown') continue;
					const options = {
						bubbles: true,
						cancelable: true,
						clientX: input.event.clientX,
						clientY: input.event.clientY,
						button: input.event.button,
						ctrlKey: input.event.ctrlKey,
						shiftKey: input.event.shiftKey,
						altKey: input.event.altKey,
						metaKey: input.event.metaKey
					};
					destination.dispatchEvent(new MouseEvent('mousedown', { ...options, buttons: 1 }));
					destination.dispatchEvent(new MouseEvent('mouseup', { ...options, buttons: 0 }));
				} else {
					destination.dispatchEvent(input.event);
				}
			}
		} finally {
			claiming = false;
			if (pendingInput.length && !interactive) pendingInput = [];
		}
	}

	function recordActivity() {
		onactivity?.();
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
		clipboardOpener =
			document.activeElement instanceof HTMLButtonElement ? document.activeElement : undefined;
		clipboardPanelOpen = true;
		clipboardFeedback = '';
		await tick();
		if (clipboardAction?.disabled) clipboardPanel?.focus();
		else clipboardAction?.focus();
	}

	function closeClipboardPanel() {
		clipboardPanelOpen = false;
		manualClipboardOpen = false;
		clipboardDraft = '';
		clipboardFeedback = '';
		clipboardOpener?.focus();
	}

	function handleClipboardPanelKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			closeClipboardPanel();
		}
	}

	async function openManualClipboard() {
		manualClipboardOpen = true;
		await tick();
		clipboardInput?.focus();
	}

	function toggleManualClipboard() {
		manualClipboardOpen = !manualClipboardOpen;
		if (!manualClipboardOpen) clipboardDraft = '';
		else void openManualClipboard();
	}

	async function sendLocalClipboard() {
		if (status !== 'connected') {
			clipboardFeedback = 'Connect to the computer before reading the device clipboard.';
			return;
		}
		if (!navigator.clipboard?.readText) {
			clipboardFeedback =
				'This browser cannot read the device clipboard. Paste text manually below.';
			await openManualClipboard();
			return;
		}
		let text: string;
		try {
			text = await navigator.clipboard.readText();
		} catch {
			clipboardFeedback = 'Clipboard access was denied. Paste text manually below.';
			await openManualClipboard();
			return;
		}
		if (!text.trim()) {
			clipboardFeedback = 'The device clipboard has no text to send.';
			return;
		}
		if (rememberClip('device', text)) clipboardFeedback = 'Added from the device clipboard.';
	}

	async function transferSelectedClip() {
		if (clipboardDraft.trim()) {
			if (!rfb || status !== 'connected' || !interactive) {
				clipboardFeedback = 'Take control of the connected computer before sending text.';
				return;
			}
			const text = clipboardDraft;
			if (text.length > MAX_CLIP_LENGTH) {
				clipboardFeedback = `Keep typed text under ${MAX_CLIP_LENGTH.toLocaleString()} characters.`;
				return;
			}
			try {
				rfb.clipboardPasteFrom(text);
				onactivity?.();
				rememberClip('manual', text);
				clipboardDraft = '';
				clipboardFeedback = 'Sent to the computer. Press Ctrl+V in the company app.';
			} catch {
				clipboardFeedback = 'Could not send the text. Try again.';
			}
			return;
		}
		const clip = clipboardClips.find((item) => item.id === selectedClipId);
		if (!clip) return;
		if (clip.source === 'computer') {
			if (!navigator.clipboard?.writeText) {
				clipboardFeedback = 'Select the text preview below and copy it manually.';
				return;
			}
			try {
				await navigator.clipboard.writeText(clip.text);
				clipboardFeedback = 'Copied to the device clipboard.';
			} catch {
				clipboardFeedback = 'Select the text preview below and copy it manually.';
			}
			return;
		}
		if (!rfb || status !== 'connected' || !interactive) {
			clipboardFeedback = 'Take control of the connected computer before sending text.';
			return;
		}
		rfb.clipboardPasteFrom(clip.text);
		onactivity?.();
		clipboardFeedback = 'Sent to the computer. Press Ctrl+V in the company app.';
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
		clearClipboardSession();
	});
</script>

<div class="desktop-viewport" bind:this={target} aria-label={title}>
	{#if src}
		<div bind:this={screen} class="desktop-screen" aria-label={title}></div>
		<div class="desktop-tools" aria-label="Desktop controls">
			<span class="desktop-live" role="status">
				{status === 'connected'
					? interactive
						? 'You’re using the computer'
						: 'Viewing · auto-fit'
					: status === 'reconnecting'
						? 'Reconnecting'
						: status === 'error'
							? 'Connection needs attention'
							: 'Connecting'}
			</span>
			<div class="desktop-tool-actions">
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
					title="Toggle fullscreen"
					aria-label="Toggle fullscreen"
					onclick={toggleFullscreen}
				>
					{#if fullscreen}<Minimize2 size={14} />{:else}<Maximize2 size={14} />{/if}
				</button>
			</div>
		</div>
		{#if clipboardPanelOpen}
			<div
				bind:this={clipboardPanel}
				class="clipboard-panel"
				role="dialog"
				tabindex="-1"
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
				<div class="clipboard-panel-actions" role="group" aria-label="Clipboard actions">
					<button
						type="button"
						bind:this={clipboardAction}
						onclick={sendLocalClipboard}
						disabled={status !== 'connected'}>Read device</button
					>
					<button type="button" aria-expanded={manualClipboardOpen} onclick={toggleManualClipboard}
						>Type text</button
					>
					<button
						type="button"
						class="clipboard-send"
						aria-label={clipboardDraft.trim()
							? 'Send typed text to the company computer'
							: clipboardClips.find((clip) => clip.id === selectedClipId)?.source === 'computer'
								? 'Copy selected computer text to the device clipboard'
								: 'Send selected text to the company computer'}
						onclick={transferSelectedClip}
						disabled={clipboardDraft.trim()
							? !rfb || status !== 'connected' || !interactive
							: !clipboardClips.some((clip) => clip.id === selectedClipId) ||
								(clipboardClips.find((clip) => clip.id === selectedClipId)?.source !== 'computer' &&
									(!rfb || status !== 'connected' || !interactive))}
						>{clipboardDraft.trim()
							? 'Send text'
							: clipboardClips.find((clip) => clip.id === selectedClipId)?.source === 'computer'
								? 'Copy'
								: 'Send'}</button
					>
				</div>
				{#if manualClipboardOpen}
					<label for="desktop-clipboard-draft">Text to send to the company computer</label>
					<textarea
						bind:this={clipboardInput}
						bind:value={clipboardDraft}
						id="desktop-clipboard-draft"
						rows="5"
						placeholder="Paste text here"></textarea>
				{/if}
				{#if clipboardClips.length}
					<section class="clipboard-recents" aria-labelledby="clipboard-recents-title">
						<div class="clipboard-recents-heading">
							<h3 id="clipboard-recents-title">Recent items <span>this session</span></h3>
							<button type="button" class="clipboard-clear" onclick={clearRecentClips}>Clear</button
							>
						</div>
						<p class="clipboard-list-hint">Select an item to transfer it.</p>
						<div class="clipboard-clip-list" role="group" aria-label="Recent clipboard items">
							{#each clipboardClips as clip (clip.id)}
								<button
									type="button"
									class="clipboard-clip"
									aria-pressed={selectedClipId === clip.id}
									onclick={() => {
										selectedClipId = clip.id;
										clipboardFeedback = '';
									}}
								>
									<span class="clipboard-clip-source"
										>{clip.source === 'computer'
											? 'Computer'
											: clip.source === 'device'
												? 'Device'
												: 'Typed text'}</span
									>
									<span class="clipboard-clip-text">{clip.text}</span>
								</button>
							{/each}
						</div>
						{#if clipboardClips.find((clip) => clip.id === selectedClipId)?.source === 'computer'}
							<textarea
								class="clipboard-preview"
								aria-label="Selected computer clipboard text"
								readonly
								rows="4"
								value={clipboardClips.find((clip) => clip.id === selectedClipId)?.text ?? ''}
							></textarea>
						{/if}
					</section>
				{:else if !manualClipboardOpen}
					<p class="clipboard-empty">
						Read device clipboard or copy text inside the computer to see it here.
					</p>
				{/if}
				<p class="clipboard-feedback" aria-live="polite">
					{clipboardFeedback || 'Clipboard items stay here only until disconnect.'}
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
		box-sizing: border-box;
		width: 100%;
		min-height: 40px;
		padding: 4px max(12px, env(safe-area-inset-right)) 4px max(12px, env(safe-area-inset-left));
		border-top: 1px solid var(--border-strong);
		background: var(--surface-pane);
	}
	.desktop-viewport:fullscreen > .desktop-tools {
		padding-right: max(12px, env(safe-area-inset-right));
	}
	.desktop-live {
		min-width: 0;
		flex: 1 1 auto;
		padding-inline: 8px;
		color: var(--text-secondary);
		font: var(--t-label) var(--font-ui);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.desktop-tool-actions {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
		gap: 4px;
		margin-left: auto;
	}
	.desktop-tools button {
		flex: 0 0 30px;
	}
	.clipboard-panel,
	.clipboard-panel :is(button, textarea) {
		color-scheme: light;
	}
	.clipboard-panel {
		position: absolute;
		z-index: 4;
		right: 12px;
		bottom: 48px;
		width: min(380px, calc(100% - 24px));
		max-height: min(70%, 520px);
		overflow: auto;
		padding: 16px;
		box-sizing: border-box;
		border: 1px solid var(--border-strong);
		border-radius: 10px;
		color: var(--ink);
		background: var(--surface-pane);
		box-shadow: var(--shadow-lift);
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
		box-sizing: border-box;
		border: 1px solid var(--control-edge);
		border-radius: 7px;
		color: var(--ink);
		background: var(--surface);
		font: var(--t-body) var(--font-ui);
	}
	.clipboard-panel-actions {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 5px;
	}
	.clipboard-panel-actions button {
		min-width: 0;
		white-space: nowrap;
		padding-inline: 5px;
		font: var(--t-label) var(--font-ui);
	}
	.clipboard-panel button {
		min-height: 32px;
		padding: 5px 9px;
		border: 1px solid var(--control-edge);
		border-radius: 6px;
		color: var(--ink);
		background: var(--surface-alt);
		font: var(--t-label) var(--font-ui);
		cursor: pointer;
	}
	.clipboard-panel button:hover:not(:disabled) {
		background: var(--surface-alt);
		border-color: var(--border-strong);
	}
	.clipboard-panel :is(button, textarea):focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.clipboard-panel button:disabled {
		color: var(--text-tertiary);
		cursor: not-allowed;
	}
	.clipboard-panel .clipboard-send:disabled {
		color: var(--text-tertiary);
		background: var(--surface-alt);
		border-color: var(--control-edge);
	}
	.clipboard-panel .clipboard-send {
		color: var(--text-inverse);
		background: var(--intent-conversation);
		border-color: transparent;
	}
	.clipboard-panel .clipboard-send:hover:not(:disabled) {
		background: color-mix(in srgb, var(--intent-conversation) 88%, var(--ink));
	}
	.clipboard-recents {
		margin-top: 13px;
	}
	.clipboard-recents-heading {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 8px;
		margin-bottom: 7px;
	}
	.clipboard-recents h3 {
		display: flex;
		align-items: baseline;
		gap: 6px;
		margin: 0;
		font: 600 var(--t-label) var(--font-ui);
	}
	.clipboard-panel .clipboard-clear {
		min-height: 0;
		padding: 0;
		border: 0;
		background: transparent;
		color: var(--text-secondary);
		text-decoration: underline;
	}
	.clipboard-recents h3 span {
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-ui);
	}
	.clipboard-list-hint {
		margin: -3px 0 7px;
		color: var(--text-secondary);
		font: var(--t-label) var(--font-ui);
	}
	.clipboard-clip-list {
		display: grid;
		gap: 5px;
		max-height: 194px;
		overflow: auto;
	}
	.clipboard-panel .clipboard-clip {
		display: grid;
		gap: 3px;
		min-height: 0;
		padding: 7px 9px;
		text-align: left;
		background: var(--surface);
	}
	.clipboard-panel .clipboard-clip[aria-pressed='true'] {
		border-color: var(--intent-conversation);
		background: color-mix(in srgb, var(--intent-conversation) 8%, var(--surface));
	}
	.clipboard-clip-source {
		color: var(--text-secondary);
		font: 600 10px var(--font-ui);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.clipboard-clip-text {
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font: var(--t-label) var(--font-mono);
	}
	.clipboard-panel .clipboard-preview {
		max-height: 96px;
		overflow: auto;
		resize: vertical;
		margin: 7px 0 0;
		padding: 8px;
		border-radius: 6px;
		background: var(--surface-alt);
		font: var(--t-label) var(--font-mono);
	}
	.clipboard-empty {
		margin: 13px 0 0;
		color: var(--text-secondary);
		font: var(--t-label) var(--font-ui);
	}
	.clipboard-feedback {
		margin: 10px 0 0;
		color: var(--text-secondary);
		font: var(--t-label) var(--font-ui);
	}
	.desktop-tools button {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		border: 0;
		border-radius: 6px;
		color: var(--ink);
		background: transparent;
		cursor: pointer;
	}
	.desktop-tools button:hover {
		background: var(--surface-alt);
	}
	.desktop-tools button:focus-visible {
		background: var(--surface-alt);
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
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
		border: 1px solid var(--border-strong);
		border-radius: 8px;
		color: var(--ink);
		background: var(--surface-pane);
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
		border: 1px solid var(--border-strong);
		border-radius: 8px;
		color: var(--ink);
		background: var(--surface-pane);
		font: var(--t-label) var(--font-ui);
	}
	@media (max-width: 600px) {
		.clipboard-panel {
			right: 8px;
			bottom: calc(48px + env(safe-area-inset-bottom));
			width: calc(100% - 16px);
			max-height: min(70%, 520px);
			padding: var(--space-3);
		}
		.clipboard-panel-actions {
			gap: 4px;
		}
		.desktop-tools {
			padding-right: max(8px, env(safe-area-inset-right));
		}
		.desktop-live {
			padding-inline: 0 4px;
		}
	}
	@media (max-width: 360px) {
		.clipboard-panel-actions button {
			font-size: 11px;
		}
	}
	.desktop-viewport-empty {
		margin: auto;
		color: var(--text-secondary);
	}
</style>
