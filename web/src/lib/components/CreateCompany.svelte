<script lang="ts">
	import { goto } from '$app/navigation';
	import { createCompany } from '$lib/model/cockpit';
	import { DEFAULT_COMPANY_MODEL } from '$lib/model/model-presets';
	let busy = $state(false);
	let error = $state('');
	async function open() {
		if (busy) return;
		busy = true;
		error = '';
		try {
			const company = await createCompany({
				name: `company_${crypto.randomUUID().replaceAll('-', '').slice(0, 16)}`,
				display_name: 'Untitled company',
				mission: '',
				model: DEFAULT_COMPANY_MODEL
			});
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
{#if error}<span class="error" role="alert">{error}</span>{/if}

<style>
	.add-company {
		display: grid;
		place-items: center;
		width: 36px;
		height: 36px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
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
