<script lang="ts">
	import { connectionLabel, modelLabel } from '$lib/model/intelligence-labels';
	import { failureSentence } from '$lib/model/failure';
	import { intelligenceQuery, type IntelligenceAgent } from '$lib/model/intelligence.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { Section, Item, Notice, Empty } from '$lib/ui/page';
	import Bot from '@lucide/svelte/icons/bot';
	const catalog = modelCatalog();
	let { companyId }: { companyId: string } = $props();
	const source = $derived(intelligenceQuery(companyId));
	let editing = $state(''),
		connection = $state(''),
		model = $state(''),
		busy = $state(false),
		error = $state(''),
		notice = $state('');
	const selected = $derived(source.view?.connections.find((c) => c.id === connection));
	let custom = $state(false);
	const presets = $derived(
		selected?.models ??
			catalog.models(
				selected?.account_provider ??
					(selected?.kind === 'harness'
						? selected.provider === 'codex'
							? 'openai'
							: 'anthropic'
						: (selected?.provider ?? '')),
				selected?.account_kind ?? 'api_key'
			)
	);
	const rows = $derived(
		source.view
			? [
					{
						id: 'default',
						name: 'Company default',
						role: 'Used by agents without an override',
						assignment: source.view.default,
						effective_model:
							source.view.default?.model ??
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
	function choose() {
		model =
			presets.find((m) => 'default' in m && m.default)?.id ??
			presets[0]?.id ??
			selected?.model ??
			'';
		custom = false;
	}
	function edit(agent: IntelligenceAgent) {
		editing = agent.id;
		custom = false;
		error = '';
		notice = '';
		connection =
			agent.assignment?.connection ??
			source.view?.connections.find(
				(c) =>
					c.id === `harness:${agent.harness}` ||
					c.id === `direct:${agent.effective_model.split('/')[0]}`
			)?.id ??
			source.view?.connections[0]?.id ??
			'';
		model =
			agent.assignment?.model ??
			(agent.effective_model.startsWith((selected?.provider ?? '') + '/')
				? agent.effective_model.slice(agent.effective_model.indexOf('/') + 1)
				: (selected?.models?.find((m) => m.default)?.id ??
					selected?.models?.[0]?.id ??
					selected?.model ??
					presets[0]?.id ??
					''));
	}
	async function save(reset = false) {
		if (!source.view || busy) return;
		busy = true;
		error = '';
		try {
			const response = await fetch(
				`/api/companies/${companyId}/intelligence/${encodeURIComponent(editing)}`,
				{
					method: 'PUT',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ connection, model, reset, revision: source.view.revision })
				}
			);
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not save assignment.');
			await source.refresh();
			announceIntelligenceChange(companyId);
			editing = '';
			notice = 'Intelligence saved. Applies to the next session.';
		} catch (cause) {
			error = failureSentence(cause, 'Could not save assignment.');
			await source.refresh();
		} finally {
			busy = false;
		}
	}
</script>

<Section
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
				} · ${modelLabel(agent.assignment?.model ?? agent.effective_model)}`}
				selected={editing === agent.id}
			>
				{#snippet leading()}<Bot size={15} strokeWidth={1.8} />{/snippet}
				{#snippet trailing()}
					{#if agent.id === 'default'}<span title={agent.role}>Default</span>{/if}
					{#if source.view?.connections.length && editing !== agent.id}<button
							class="btn small"
							disabled={busy}
							onclick={() => edit(agent)}
							aria-label={`Change the model for ${name}`}>Change</button
						>{/if}
				{/snippet}
				{#if editing === agent.id}
					<form
						class="editor"
						onsubmit={(event) => {
							event.preventDefault();
							void save();
						}}
					>
						<label
							><span>Connection</span><select
								bind:value={connection}
								onchange={choose}
								disabled={busy}
								>{#each source.view.connections as c}<option value={c.id}
										>{label(c.id)}{!c.loaded ? ' · restart pending' : ''}</option
									>{/each}</select
							></label
						>
						<label
							title={selected?.kind === 'harness'
								? selected.models?.length
									? 'Models offered by this connection.'
									: 'Catalog suggestions; the connection confirms availability.'
								: undefined}
							><span>Model</span><select
								value={custom ? '__custom' : model}
								disabled={busy}
								onchange={(event) => {
									custom = event.currentTarget.value === '__custom';
									if (!custom) model = event.currentTarget.value;
								}}
							>
								{#if model && !presets.some((m) => m.id === model)}<option value={model}
										>{model} (saved)</option
									>{/if}
								{#each presets as m}<option value={m.id}>{m.name}</option>{/each}<option
									value="__custom">Custom model…</option
								>
							</select></label
						>
						{#if custom}<label
								><span>Model ID</span><input bind:value={model} required disabled={busy} /></label
							>{/if}
						<div class="bar">
							<button class="btn primary small" disabled={busy || !connection || !model.trim()}
								>{busy ? 'Saving…' : 'Save'}</button
							>
							{#if agent.assignment && agent.id !== 'default'}<button
									class="btn small"
									type="button"
									disabled={busy}
									onclick={() => save(true)}>Use company default</button
								>{/if}
							<button
								class="btn small ghost"
								type="button"
								disabled={busy}
								onclick={() => (editing = '')}>Cancel</button
							>
						</div>
					</form>
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
	.editor {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: 12px;
		padding-top: 4px;
	}
	.editor label {
		display: grid;
		gap: 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.editor select,
	.editor input {
		min-width: 200px;
	}
	.bar {
		display: flex;
		gap: var(--space-2);
	}
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
