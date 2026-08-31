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
