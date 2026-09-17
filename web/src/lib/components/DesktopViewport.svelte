<script lang="ts">
	import { onDestroy } from 'svelte';
	import type { Snippet } from 'svelte';

	let {
		src = '',
		title = 'Company computer',
		offline = null,
		onload = null,
		onactivity = null
	}: {
		src?: string;
		title?: string;
		offline?: Snippet | null;
		onload?: (() => void) | null;
		/** A real pointer/key event observed inside the live desktop. */
		onactivity?: (() => void) | null;
	} = $props();

	let frame = $state<HTMLIFrameElement>();
	let detachInputObservers = () => {};

	function recordActivity() {
		onactivity?.();
	}

	function connected() {
		detachInputObservers();
		/* noVNC is served through the same local owner origin. Listen in capture
		 * phase so activity is observed even when the client consumes the event.
		 * If a future desktop transport becomes cross-origin, the outer pointer
		 * handler still provides a conservative claim signal. */
		const document = frame?.contentDocument;
		if (document) {
			// Keep the pointer legible on both light and dark remote content. The
			// center is the click hotspot; this changes appearance, never input ownership.
			const cursor = encodeURIComponent(
				'<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 32 32"><circle cx="16" cy="16" r="9" fill="none" stroke="black" stroke-width="4"/><circle cx="16" cy="16" r="9" fill="none" stroke="white" stroke-width="2"/><circle cx="16" cy="16" r="2.5" fill="black" stroke="white" stroke-width="1"/></svg>'
			);
			const style = document.createElement('style');
			style.textContent = `#noVNC_container canvas { cursor: url("data:image/svg+xml,${cursor}") 16 16, crosshair !important; }`;
			document.head.appendChild(style);
			const events: Array<keyof DocumentEventMap> = [
				'pointerdown',
				'pointermove',
				'wheel',
				'keydown',
				'keyup'
			];
			for (const event of events) document.addEventListener(event, recordActivity, true);
			detachInputObservers = () => {
				for (const event of events) document.removeEventListener(event, recordActivity, true);
			};
		}
		onload?.();
	}

	onDestroy(() => detachInputObservers());
</script>

<div class="desktop-viewport">
	{#if src}
		<iframe
			bind:this={frame}
			role="application"
			{title}
			{src}
			allow="clipboard-read; clipboard-write"
			referrerpolicy="same-origin"
			onpointerdown={recordActivity}
			onload={connected}
		></iframe>
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
		overflow: hidden;
		background: #101217;
	}

	.desktop-viewport iframe {
		display: block;
		width: 100%;
		height: 100%;
		min-width: 0;
		min-height: 0;
		flex: 1 1 auto;
		border: 0;
		background: #101217;
	}

	.desktop-viewport-empty {
		margin: auto;
		color: rgba(255, 255, 255, 0.72);
	}
</style>
