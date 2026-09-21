<script lang="ts">
	import { goto } from '$app/navigation';
	import { createCompany } from '$lib/model/cockpit';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	const catalog = modelCatalog();
	let dialog: HTMLDialogElement;
	let model = $state('');
	let displayName = $state('');
	const validModel = $derived(/^[a-zA-Z0-9_.:-]+\/[a-zA-Z0-9_./:-]+$/.test(model.trim()));
	let busy = $state(false);
	let error = $state('');
	function open() {
		error = '';
		dialog.showModal();
	}
	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !validModel) return;
		busy = true;
		error = '';
		try {
			const company = await createCompany({
				name: `company_${crypto.randomUUID().replaceAll('-', '').slice(0, 16)}`,
				display_name: displayName.trim() || 'Untitled company',
				mission: '',
				model: model.trim()
			});
			dialog.close();
			await goto(`/${company.id}/company/provider`);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not open a new company.';
		} finally {
			busy = false;
		}
	}
</script>

<button
	class="add-company"
	type="button"
	title="Create company"
	aria-label="Create company"
	disabled={busy}
	aria-busy={busy}
	onclick={open}
>
	{#if busy}<span aria-hidden="true">…</span>{:else}<svg
			width="18"
			height="18"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="1.8"
			aria-hidden="true"><path d="M12 5v14M5 12h14" /></svg
		>{/if}
</button>
<dialog
	bind:this={dialog}
	aria-labelledby="create-company-title"
	oncancel={(event) => {
		if (busy) event.preventDefault();
	}}
>
	<form onsubmit={submit}>
		<h2 id="create-company-title">Create company</h2>
		<label for="new-company-name">Company name</label>
		<input
			id="new-company-name"
			bind:value={displayName}
			maxlength="200"
			placeholder="Your company"
			disabled={busy}
		/>
		<label for="new-company-model">Starting model</label>
		<input
			id="new-company-model"
			bind:value={model}
			list="new-company-models"
			placeholder="Choose or enter provider/model"
			required
			pattern="\S+/\S+"
			maxlength="200"
			disabled={busy}
			aria-describedby="new-company-connection"
		/>
		<datalist id="new-company-models">
			{#each catalog.providers as provider}
				{#each provider.models as choice}<option value={`${provider.id}/${choice.id}`}
						>{provider.name} · {choice.name}</option
					>{/each}
			{/each}
		</datalist>
		<p id="new-company-connection">
			Next, connect Codex, Claude or an API provider. You can change models and assign intelligence
			per agent in company settings.
		</p>
		{#if error}<p class="error" role="alert">{error}</p>{/if}
		<footer>
			<button type="button" disabled={busy} onclick={() => dialog.close()}>Cancel</button>
			<button type="submit" disabled={busy || !validModel} aria-busy={busy}
				>{busy ? 'Creating…' : 'Create company'}</button
			>
		</footer>
	</form>
</dialog>

<style>
	dialog {
		width: min(440px, calc(100vw - 32px));
		max-height: calc(100dvh - 32px);
		padding: 24px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--ink);
	}
	dialog::backdrop {
		background: rgb(12 25 40 / 35%);
	}
	form {
		display: grid;
		gap: 12px;
	}
	h2 {
		margin: 0 0 8px;
		font-size: var(--t-head);
	}
	input {
		width: 100%;
		min-width: 0;
		min-height: 44px;
		font: inherit;
		padding: 8px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--ink);
	}
	label {
		font-size: var(--t-label);
	}
	p {
		margin: 0;
		font-size: var(--t-body);
	}
	footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 12px;
	}
	footer button {
		min-height: 44px;
	}

	.add-company {
		display: grid;
		place-items: center;
		width: 36px;
		height: 36px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--intent-conversation);
		cursor: pointer;
	}
	.add-company:hover {
		background: var(--intent-conversation-soft);
	}
	.add-company:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 3px;
	}
	.add-company:disabled {
		opacity: 0.6;
		cursor: wait;
	}
	.error {
		color: var(--state-danger);
		font-size: var(--t-label);
		max-width: 260px;
	}
	@media (max-width: 520px) {
		.add-company {
			width: 44px;
			height: 44px;
		}
	}
</style>
