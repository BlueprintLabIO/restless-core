<script lang="ts">
	/* Which model this agent thinks with, one click from where the owner talks
	 * to it. Choosing another model of the same connection applies to its next
	 * session, and the toast says so. Thinking effort is one company setting, chosen here too;
	 * everything else lives in Company › Intelligence. */
	import {
		EFFORTS,
		assignModel,
		intelligenceQuery,
		setEffort
	} from '$lib/model/intelligence.svelte';
	import { effortLabel, modelLabel } from '$lib/model/intelligence-labels';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import Check from '@lucide/svelte/icons/check';
	import Brain from '@lucide/svelte/icons/brain';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { FILTER_FROM, filterModels, splitModels } from '$lib/model/model-groups';

	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { hasOwnerSurfaceAccess } from '$lib/model/company-access';

	let { companyId, actorId, name }: { companyId: string; actorId: string; name: string } = $props();
	/* Choosing a model is the owner's alone, as the server decides; for anyone else the chip is not
	 * there, and reads nothing they would be refused. */
	const principal = $derived(companyPrincipalQuery(companyId));
	const allowed = $derived(hasOwnerSurfaceAccess(principal.view));
	const catalog = modelCatalog(() => allowed);
	const source = $derived(intelligenceQuery(companyId, () => allowed));
	const agent = $derived(source.view?.agents.find((row) => row.id === actorId) ?? null);
	const assignment = $derived(
		agent?.assignment ?? source.view?.default ?? source.view?.base_default ?? null
	);
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
	/* Newest of each family up front, older versions on request, a filter for long catalogues. */
	let query = $state('');
	let showOlder = $state(false);
	const filtered = $derived(filterModels(choices, query));
	const grouped = $derived(splitModels(filtered, current));
	const shown = $derived(query || showOlder ? filtered : grouped.latest);
	let busy = $state(false);
	let failure = $state('');

	async function choose(model: string) {
		if (!source.view || !assignment || busy || model === current) return;
		busy = true;
		failure = '';
		try {
			await assignModel(companyId, actorId, {
				connection: assignment.connection,
				model,
				reset: false,
				revision: source.view.revision
			});
			announceIntelligenceChange(companyId);
			const chosen = choices.find((row) => row.id === model)?.name ?? modelLabel(model);
			toast(`${name} will use ${chosen} from their next session.`);
		} catch (cause) {
			failure = failureSentence(cause, 'Could not change the model.');
		} finally {
			await source.refresh();
			busy = false;
		}
	}
	async function chooseEffort(effort: string) {
		if (!source.view || busy || effort === agent?.thinking_effort) return;
		busy = true;
		failure = '';
		try {
			await setEffort(companyId, { effort, revision: source.view.revision });
			announceIntelligenceChange(companyId);
			const label = EFFORTS.find((row) => row.value === effort)?.label ?? effort;
			toast(`Agents will think with ${label.toLowerCase()} effort from their next session.`);
		} catch (cause) {
			failure = failureSentence(cause, 'Could not change the effort.');
		} finally {
			await source.refresh();
			busy = false;
		}
	}
	/* The chip names the model, short: the family without its provider or date. */
	const shortModel = $derived.by(() => {
		const label = agent ? modelLabel(agent.effective_model) : '';
		return label.length > 18 ? `${label.slice(0, 17)}…` : label;
	});
	holdQueries(() => [source]);
</script>

{#if !allowed}{:else if agent && modelLabel(agent.effective_model) === 'Unavailable'}
	<a class="chip-link" href={`/${encodeURIComponent(companyId)}/company/provider`}>Choose a model</a
	>
{:else if agent}
	<div
		class="chip"
		title={`${name} thinks with ${modelLabel(agent.effective_model)}, ${effortLabel(agent.thinking_effort).toLowerCase()} effort`}
	>
		<ActionMenu label={`${name}’s model`}>
			{#snippet trigger()}<span class="face"
					><Brain size={14} strokeWidth={1.8} aria-hidden="true" /><span class="model-name"
						>{shortModel}</span
					></span
				>{/snippet}
			<p class="now">
				{modelLabel(agent.effective_model)}<i aria-hidden="true">·</i>{effortLabel(
					agent.thinking_effort
				)} effort
			</p>
			<div class="efforts" role="group" aria-label="Thinking effort for every agent">
				{#each EFFORTS as effort (effort.value)}
					<button
						type="button"
						class:on={agent.thinking_effort === effort.value}
						aria-pressed={agent.thinking_effort === effort.value}
						disabled={busy}
						title={`${effort.hint}. Applies to every agent from their next session.`}
						onclick={() => chooseEffort(effort.value)}>{effort.label}</button
					>
				{/each}
			</div>
			{#if choices.length >= FILTER_FROM || assignment?.connection.includes('openrouter')}<input
					class="filter"
					type="search"
					placeholder="Find a model"
					aria-label="Find a model"
					bind:value={query}
				/>{/if}
			<div class="models">
				{#each shown as model (model.id)}
					<button type="button" disabled={busy || !assignment} onclick={() => choose(model.id)}
						><span>{model.name}</span>{#if model.id === current}<Check
								size={14}
								aria-label="Current"
							/>{/if}</button
					>
				{:else}<p class="now">No listed model matches.</p>{/each}
			</div>
			{#if query.trim() && !choices.some((model) => model.id === query.trim())}<button
					type="button"
					disabled={busy || !assignment}
					title="Use a model ID this provider offers that the list does not show"
					onclick={() => choose(query.trim())}>Use model ID “{query.trim()}”</button
				>{/if}
			{#if !query && grouped.older.length}<button
					type="button"
					class="older"
					aria-expanded={showOlder}
					onclick={() => (showOlder = !showOlder)}
					>{showOlder ? 'Fewer models' : `Older models (${grouped.older.length})`}</button
				>{/if}
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
		display: flex;
		align-items: center;
		min-width: 0;
		height: 24px;
		padding: 0 6px;
		border-radius: 999px;
		color: var(--text-tertiary);
	}
	.face {
		display: flex;
		align-items: center;
		gap: 4px;
		min-width: 0;
	}
	.model-name {
		min-width: 0;
		overflow: hidden;
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.efforts {
		display: flex;
		flex-wrap: wrap;
		gap: 2px;
		margin: 0 8px 6px;
	}
	.chip :global(.action-menu-panel .efforts button) {
		flex: 1 0 auto;
		justify-content: center;
		padding: 2px 6px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		font-size: var(--t-label);
	}
	.chip :global(.action-menu-panel .efforts button.on) {
		border-color: var(--border-strong);
		background: var(--surface-hover);
		color: var(--ink);
	}
	.chip :global(summary:hover) {
		color: var(--ink);
	}
	.filter {
		margin: 4px 8px;
		padding: 4px 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
	}
	.models {
		display: grid;
		max-height: min(320px, 50vh);
		overflow-y: auto;
	}
	.older {
		color: var(--text-tertiary);
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
