<script lang="ts">
	import type { Snippet } from 'svelte';
	import X from '@lucide/svelte/icons/x';

	let {
		title,
		busy = false,
		onclose,
		children
	}: {
		title: string;
		busy?: boolean;
		onclose: () => void;
		children: Snippet;
	} = $props();
	const id = $props.id();
	let dialog = $state<HTMLDialogElement>();
	$effect(() => {
		if (dialog && !dialog.open) {
			dialog.showModal();
			dialog
				.querySelector<HTMLElement>(
					'input:not([type=hidden]):not(:disabled), textarea:not(:disabled), select:not(:disabled)'
				)
				?.focus();
		}
	});
</script>

<dialog
	bind:this={dialog}
	aria-labelledby={id}
	oncancel={(event) => {
		event.preventDefault();
		if (!busy) onclose();
	}}
>
	<header>
		<h2 {id}>{title}</h2>
		<button
			class="btn small ghost"
			type="button"
			aria-label="Close"
			disabled={busy}
			onclick={onclose}><X size={16} aria-hidden="true" /></button
		>
	</header>
	{@render children()}
</dialog>

<style>
	dialog {
		width: min(520px, calc(100vw - 32px));
		max-height: calc(100dvh - 32px);
		overflow: auto;
		padding: 24px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-pane);
		color: var(--ink);
		box-shadow: var(--shadow-lift);
	}
	dialog::backdrop {
		background: color-mix(in srgb, var(--ink) 28%, transparent);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 20px;
	}
	h2 {
		margin: 0;
		font-size: var(--t-head);
		line-height: 1.35;
		font-weight: 600;
	}
</style>
