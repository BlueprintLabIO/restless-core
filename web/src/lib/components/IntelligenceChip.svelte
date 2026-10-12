<script lang="ts">
	/* Which model this agent thinks with, one click from where the owner talks
	 * to it. Choosing another model of the same connection applies to its next
	 * session; everything else lives in Company › Intelligence. */
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { effortLabel, modelLabel } from '$lib/model/intelligence-labels';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	import { failureSentence } from '$lib/model/failure';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import Check from '@lucide/svelte/icons/check';
	import Brain from '@lucide/svelte/icons/brain';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	let { companyId, actorId, name }: { companyId: string; actorId: string; name: string } = $props();
	const catalog = modelCatalog();
	const source = $derived(intelligenceQuery(companyId));
	const agent = $derived(source.view?.agents.find((row) => row.id === actorId) ?? null);
	const assignment = $derived(agent?.assignment ?? source.view?.default ?? source.view?.base_default ?? null);
	const connection = $derived(
		source.view?.connections.find((row) => row.id === assignment?.connection) ?? null
	);
	const choices = $derived(
		connection?.models ??
			catalog.models(
				connection?.account_provider ?? connection?.provider ?? '',
				connection?.account_kind ?? 'api_key'
			)
	);
	const current = $derived(assignment?.model ?? '');
	let busy = $state(false);
	let failure = $state('');

	async function choose(model: string) {
		if (!source.view || !assignment || busy || model === current) return;
		busy = true;
		failure = '';
		try {
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/intelligence/${encodeURIComponent(actorId)}`,
				{
					method: 'PUT',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({
						connection: assignment.connection,
						model,
						reset: false,
						revision: source.view.revision
					})
				}
			);
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not change the model.');
			announceIntelligenceChange(companyId);
		} catch (cause) {
			failure = failureSentence(cause, 'Could not change the model.');
		} finally {
			await source.refresh();
			busy = false;
		}
	}
	holdQueries(() => [source]);
</script>

{#if agent && modelLabel(agent.effective_model) === 'Unavailable'}
	<a class="chip-link" href={`/${encodeURIComponent(companyId)}/company/provider`}>Choose a model</a
	>
{:else if agent}
	<div
		class="chip"
		title={`${name} thinks with ${modelLabel(agent.effective_model)}, ${effortLabel(agent.thinking_effort).toLowerCase()} effort`}
	>
		<ActionMenu label={`${name}’s model`}>
			{#snippet trigger()}<span class="face" aria-hidden="true"
					><Brain size={14} strokeWidth={1.8} /></span
				>{/snippet}
			<p class="now">
				{modelLabel(agent.effective_model)}<i aria-hidden="true">·</i>{effortLabel(
					agent.thinking_effort
				)} effort
			</p>
			{#each choices as model (model.id)}
				<button type="button" disabled={busy || !assignment} onclick={() => choose(model.id)}
					><span>{model.name}</span>{#if model.id === current}<Check
							size={14}
							aria-label="Current"
						/>{/if}</button
				>
			{/each}
			{#if failure}<p class="failure" role="alert">{failure}</p>{/if}
			<a href={`/${encodeURIComponent(companyId)}/company/provider`}>All intelligence settings</a>
		</ActionMenu>
	</div>
{/if}

<style>
	.chip,
	.chip :global(.action-menu) {
		flex: 0 1 auto;
		min-width: 0;
	}
	.chip :global(summary) {
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		padding: 0;
		border-radius: 999px;
		color: var(--text-tertiary);
	}
	.chip :global(summary:hover) {
		color: var(--ink);
	}
	.chip-link {
		padding: 0 8px;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.chip-link:hover {
		color: var(--ink);
	}
	.now {
		margin: 4px 10px 6px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.now i {
		margin: 0 4px;
		color: var(--text-tertiary);
		font-style: normal;
	}
	.chip :global(.action-menu-panel button) {
		display: flex;
		justify-content: space-between;
		gap: 12px;
	}
	.failure {
		margin: 4px 8px;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
