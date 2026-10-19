<script lang="ts">
	/* Company › Intelligence › Models: what powers each agent. Changing one opens the shared model
	 * editor (S101), the same one an Inbox card shows when an agent's model is refused. */
	import { connectionLabel, modelLabel } from '$lib/model/intelligence-labels';
	import { failureSentence } from '$lib/model/failure';
	import {
		assignModel,
		intelligenceQuery,
		type IntelligenceAgent
	} from '$lib/model/intelligence.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	import { Section, Item, Notice, Empty } from '$lib/ui/page';
	import Bot from '@lucide/svelte/icons/bot';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import EditorForm from './editors/EditorForm.svelte';
	import ModelAssignmentEditor from './editors/ModelAssignmentEditor.svelte';

	let { companyId }: { companyId: string } = $props();
	const source = $derived(intelligenceQuery(companyId));
	let editing = $state(''),
		busy = $state(false),
		error = $state(''),
		notice = $state('');
	const rows = $derived(
		source.view
			? [
					{
						id: 'default',
						name: 'Company default',
						role: 'Used by agents without an override',
						assignment: source.view.default ?? source.view.base_default ?? null,
						effective_model:
							(source.view.default ?? source.view.base_default)?.model ??
							(source.view.connections.length ? 'Choose a connection' : 'Connect a provider below'),
						harness: ''
					},
					...source.view.agents
				]
			: []
	);

	/* Agents on the company default all say the same thing; list the default,
	 * the Exec and anyone with their own choice, and fold the rest. */
	let showAll = $state(false);
	const shownRows = $derived(
		showAll
			? rows
			: rows.filter(
					(row) =>
						row.id === 'default' || row.id === 'exec' || !!row.assignment || row.id === editing
				)
	);
	const foldedCount = $derived(rows.length - shownRows.length);

	function label(id: string) {
		return connectionLabel(id, source.view?.connections ?? []);
	}
	function edit(agent: IntelligenceAgent) {
		editing = agent.id;
		error = '';
		notice = '';
	}
	/* The plan's included AI stays one click away once the company has moved to its own key. */
	const included = $derived(source.view?.connections.find((c) => c.included) ?? null);
	const defaultConnection = $derived(
		(source.view?.default ?? source.view?.base_default)?.connection ?? ''
	);
	async function useIncluded() {
		if (!included || !source.view || busy) return;
		const base = source.view.base_default;
		const model =
			base?.connection === included.id
				? base.model
				: (included.models?.find((m) => m.default)?.id ??
					included.models?.[0]?.id ??
					included.model ??
					'');
		if (!model) {
			editing = 'default';
			return;
		}
		busy = true;
		error = '';
		try {
			await assignModel(companyId, 'default', {
				connection: included.id,
				model,
				reset: false,
				revision: source.view.revision
			});
			await source.refresh();
			announceIntelligenceChange(companyId);
			notice = 'Intelligence saved. Applies to the next session.';
		} catch (cause) {
			error = failureSentence(cause, 'Could not save assignment.');
			await source.refresh();
		} finally {
			busy = false;
		}
	}
	holdQueries(() => [source]);
</script>

<Section
	id="models"
	title="Models"
	info="What powers each agent. Agents without their own choice use the company default. Changes apply to the next session."
>
	{#if source.error}
		<Notice tone="danger" title="Could not load agents">
			{#snippet actions()}<button class="btn small" onclick={() => source.refresh()}>Retry</button
				>{/snippet}
		</Notice>
	{:else if !source.view}
		<Empty compact title="Loading agents…" />
	{:else}
		{#each shownRows as agent (agent.id)}
			{@const name = agent.id === 'exec' ? 'Exec' : agent.name}
			<Item
				title={name}
				meta={`${
					agent.assignment
						? label(agent.assignment.connection)
						: agent.id === 'default'
							? 'No default selected'
							: 'Company default'
				} · ${
					!agent.assignment && agent.id !== 'default' && !defaultConnection
						? 'Not chosen yet'
						: modelLabel(agent.assignment?.model ?? agent.effective_model)
				}`}
				selected={editing === agent.id}
			>
				{#snippet leading()}<Bot size={15} strokeWidth={1.8} />{/snippet}
				{#snippet trailing()}
					{#if agent.id === 'default'}<span title={agent.role}>Default</span>{/if}
					{#if agent.id === 'default' && included && defaultConnection !== included.id && editing !== 'default'}<button
							class="btn small"
							disabled={busy}
							title="Go back to the AI included in your plan"
							onclick={() => void useIncluded()}>Use Included AI</button
						>{/if}
					{#if source.view?.connections.length && editing !== agent.id}<button
							class="btn small"
							disabled={busy}
							onclick={() => edit(agent)}
							aria-label={`Change the model for ${name}`}>Change</button
						>{/if}
				{/snippet}
				{#if editing === agent.id && source.view}
					<EditorForm oncancel={() => (editing = '')}>
						<ModelAssignmentEditor
							{companyId}
							view={source.view}
							agentId={agent.id}
							onsaved={async (receipt) => {
								await source.refresh();
								editing = '';
								notice = receipt;
							}}
						/>
					</EditorForm>
				{/if}
			</Item>
		{/each}
		{#if foldedCount > 0 || showAll}
			<button
				class="more"
				type="button"
				aria-expanded={showAll}
				onclick={() => (showAll = !showAll)}
			>
				{showAll
					? 'Show fewer'
					: `${foldedCount} more ${foldedCount === 1 ? 'agent uses' : 'agents use'} the company default`}
			</button>
		{/if}
	{/if}
	{#if error}<Notice tone="danger" title="Could not save the model">{error}</Notice>{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}
</Section>

<style>
	.more {
		width: 100%;
		min-height: 40px;
		padding: 0 16px;
		border: 0;
		background: transparent;
		color: var(--text-tertiary);
		font: inherit;
		font-size: var(--t-body);
		text-align: left;
		cursor: pointer;
	}
	.more:hover {
		background: var(--surface-hover);
		color: var(--ink);
	}
</style>
