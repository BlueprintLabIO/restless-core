<script lang="ts">
	/* The one place a confirmation appears. Escape or Cancel says no; the act's own button says
	 * yes. Cancel holds the focus first, so Enter never goes ahead by accident. */
	import { tick } from 'svelte';
	import { confirmation } from './confirm.svelte';

	let dialog: HTMLDialogElement | undefined = $state();
	let cancel: HTMLButtonElement | undefined = $state();
	const pending = $derived(confirmation.pending);
	let typed = $state('');
	$effect(() => {
		pending;
		typed = '';
	});
	const ready = $derived(!pending?.typeToConfirm || typed.trim() === pending.typeToConfirm);

	$effect(() => {
		if (!dialog) return;
		if (pending && !dialog.open) {
			dialog.showModal();
			void tick().then(() => cancel?.focus());
		} else if (!pending && dialog.open) dialog.close();
	});
</script>

<dialog
	bind:this={dialog}
	class="confirm bridge-tokens"
	aria-labelledby="confirm-title"
	aria-describedby={pending?.body ? 'confirm-body' : undefined}
	onclose={() => confirmation.pending?.answer(false)}
>
	{#if pending}
		<h2 id="confirm-title">{pending.title}</h2>
		{#if pending.body}<p id="confirm-body">{pending.body}</p>{/if}
		{#if pending.offer}<p>
				<a
					class="btn small"
					href={pending.offer.href}
					download={pending.offer.download ? '' : undefined}>{pending.offer.label}</a
				>
			</p>{/if}
		{#if pending.typeToConfirm}<label class="typed"
				>Type <strong>{pending.typeToConfirm}</strong> to confirm
				<input bind:value={typed} autocomplete="off" spellcheck="false" />
			</label>{/if}
		<div class="actions">
			<button
				bind:this={cancel}
				type="button"
				class="btn small"
				onclick={() => pending.answer(false)}>Cancel</button
			>
			<button
				type="button"
				class="btn small"
				class:danger={pending.danger}
				class:primary={!pending.danger}
				disabled={!ready}
				onclick={() => pending.answer(true)}>{pending.action}</button
			>
		</div>
	{/if}
</dialog>

<style>
	.confirm {
		width: min(420px, calc(100vw - 32px));
		padding: 18px 20px 16px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-float, var(--shadow-soft));
		color: var(--ink);
	}
	.confirm::backdrop {
		background: rgba(34, 39, 51, 0.18);
	}
	h2 {
		margin: 0 0 8px;
		font-size: var(--t-head);
		font-weight: 600;
		overflow-wrap: anywhere;
	}
	p {
		margin: 0 0 14px;
		color: var(--text-secondary);
	}
	.typed {
		display: grid;
		gap: 6px;
		margin: 0 0 14px;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.typed strong {
		color: var(--ink);
		overflow-wrap: anywhere;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
</style>
