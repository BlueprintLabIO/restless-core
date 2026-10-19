<script lang="ts">
	import { connectionLabel, modelLabel } from '$lib/model/intelligence-labels';
	import { failureSentence } from '$lib/model/failure';
	import {
		assignModel,
		intelligenceQuery,
		type IntelligenceAgent
	} from '$lib/model/intelligence.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { Section, Item, Notice, Empty } from '$lib/ui/page';
	import Field from '$lib/ui/forms/Field.svelte';
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
	import Bot from '@lucide/svelte/icons/bot';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
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
	/* One saved key or sign-in can run under more than one harness (an Anthropic key under OMP or
	 * Claude Code). The connection list names each account once; the harness is its own choice. */
	const account = (id: string) =>
		id.startsWith('account-harness:')
			? id.split(':').slice(2).join(':')
			: id.replace(/^account:/, '');
	const accounts = $derived(
		(source.view?.connections ?? []).filter(
			(c, index, all) => all.findIndex((other) => account(other.id) === account(c.id)) === index
		)
	);
	const harnesses = $derived(
		(source.view?.connections ?? []).filter((c) => account(c.id) === account(connection))
	);
	const HARNESS_NAMES: Record<string, string> = {
		codex: 'Codex',
		'claude-agent': 'Claude Code'
	};
	const harnessName = (id: string) =>
		id.startsWith('account-harness:')
			? (HARNESS_NAMES[id.split(':')[1]] ?? id.split(':')[1])
			: 'Built in';
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
	/* The plan's included AI stays one click away once the company has moved to its own key. */
	const included = $derived(source.view?.connections.find((c) => c.included) ?? null);
	const defaultConnection = $derived(
		(source.view?.default ?? source.view?.base_default)?.connection ?? ''
	);
	async function useIncluded() {
		if (!included || !source.view) return;
		editing = 'default';
		connection = included.id;
		const base = source.view.base_default;
		model =
			base?.connection === included.id
				? base.model
				: (included.models?.find((m) => m.default)?.id ??
					included.models?.[0]?.id ??
					included.model ??
					presets[0]?.id ??
					'');
		if (!model) {
			choose();
			return;
		}
		await save();
	}
	async function save(reset = false) {
		if (!source.view || busy) return;
		if (!reset && (!connection || !model.trim())) {
			error = connection ? 'Enter the full model ID.' : 'Choose a connection.';
			return;
		}
		busy = true;
		error = '';
		try {
			await assignModel(companyId, editing, {
				connection,
				model,
				reset,
				revision: source.view.revision
			});
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
				{#if editing === agent.id}
					<form
						class="form plain"
						onsubmit={(event) => {
							event.preventDefault();
							void save();
						}}
					>
						<Field label="Connection" for={`connection-${agent.id}`}>
							<select
								id={`connection-${agent.id}`}
								value={accounts.find((c) => account(c.id) === account(connection))?.id ?? ''}
								onchange={(event) => {
									const next = event.currentTarget.value;
									const kept = harnessName(connection);
									connection =
										source.view?.connections.find(
											(c) => account(c.id) === account(next) && harnessName(c.id) === kept
										)?.id ?? next;
									choose();
								}}
								disabled={busy}
								>{#each accounts as c}<option
										value={c.id}
										title={c.loaded
											? undefined
											: 'Ready once the company restarts, which it does on its own.'}
										>{label(c.id.replace(/^account-harness:[^:]+:/, 'account:'))}</option
									>{/each}</select
							>
						</Field>
						<Field
							label="Model"
							hint={selected?.kind === 'harness'
								? selected.models?.length
									? 'Offered by this connection.'
									: 'Suggestions; the connection confirms them.'
								: ''}
							for={`model-${agent.id}`}
						>
							<select
								id={`model-${agent.id}`}
								value={custom ? '__custom' : model}
								disabled={busy}
								onchange={(event) => {
									custom = event.currentTarget.value === '__custom';
									if (!custom) model = event.currentTarget.value;
								}}
							>
								{#if model && !presets.some((m) => m.id === model)}<option value={model}
										>{modelLabel(model)} (current)</option
									>{/if}
								{#each presets as m}<option value={m.id}
										>{m.name}{'default' in m && m.default ? ' (recommended)' : ''}</option
									>{/each}<option value="__custom">Custom model ID…</option>
							</select>
						</Field>
						{#if custom}
							<Field label="Full model ID" for={`custom-${agent.id}`}>
								<input id={`custom-${agent.id}`} bind:value={model} required disabled={busy} />
							</Field>
						{/if}
						{#if harnesses.length > 1}<details
								class="form-more"
								open={harnessName(connection) !== 'Built in'}
							>
								<summary>More options</summary>
								<Field
									label="Runs with"
									info="Which agent program runs this model. Built in is the default; Claude Code and Codex are the model makers' own programs."
									for={`harness-${agent.id}`}
								>
									<select
										id={`harness-${agent.id}`}
										value={connection}
										disabled={busy}
										onchange={(event) => (connection = event.currentTarget.value)}
										>{#each harnesses as c}<option value={c.id}>{harnessName(c.id)}</option
											>{/each}</select
									>
								</Field>
							</details>{/if}
						<FormFoot submit="Save" {busy} busyLabel="Saving…" oncancel={() => (editing = '')}>
							{#if agent.assignment && agent.id !== 'default'}<button
									class="btn small"
									type="button"
									disabled={busy}
									onclick={() => save(true)}>Use company default</button
								>{/if}
						</FormFoot>
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
