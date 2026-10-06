<script lang="ts">
	/* Copies a value; the icon turns into a check for a moment, so the click is answered where it
	 * happened. A failure says so rather than pretending. */
	import Copy from '@lucide/svelte/icons/copy';
	import Check from '@lucide/svelte/icons/check';

	let { value, label = 'Copy' }: { value: string; label?: string } = $props();
	let state = $state<'idle' | 'copied' | 'failed'>('idle');
	let timer: ReturnType<typeof setTimeout> | undefined;

	async function copy() {
		try {
			await navigator.clipboard.writeText(value);
			state = 'copied';
		} catch {
			state = 'failed';
		}
		clearTimeout(timer);
		timer = setTimeout(() => (state = 'idle'), 1600);
	}
</script>

<button
	type="button"
	class="copy-button"
	class:copied={state === 'copied'}
	aria-label={state === 'copied' ? 'Copied' : state === 'failed' ? 'Could not copy' : label}
	title={state === 'copied' ? 'Copied' : state === 'failed' ? 'Could not copy' : label}
	onclick={copy}
>
	<span class="icon copy-icon" aria-hidden="true"><Copy size={14} strokeWidth={1.9} /></span>
	<span class="icon check-icon" aria-hidden="true"><Check size={14} strokeWidth={2.2} /></span>
</button>
<span class="sr-only" aria-live="polite"
	>{state === 'copied' ? 'Copied' : state === 'failed' ? 'Copy failed' : ''}</span
>

<style>
	.copy-button {
		position: relative;
		display: grid;
		place-items: center;
		flex: none;
		width: 28px;
		height: 28px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard),
			transform var(--motion-press) var(--ease-standard);
	}
	.copy-button:hover {
		background: var(--wash-hover, color-mix(in srgb, var(--ink) 6%, transparent));
		color: var(--ink);
	}
	.copy-button:active {
		transform: scale(0.92);
	}
	.icon {
		grid-area: 1 / 1;
		display: grid;
		transition:
			opacity var(--motion-state) var(--ease-standard),
			transform 0.35s var(--ease-spring);
	}
	.check-icon {
		color: var(--intent-feedback);
		opacity: 0;
		transform: scale(0.4);
	}
	.copied .copy-icon {
		opacity: 0;
		transform: scale(0.4);
	}
	.copied .check-icon {
		opacity: 1;
		transform: scale(1);
	}
	@media (prefers-reduced-motion: reduce) {
		.icon {
			transition: none;
		}
	}
</style>
