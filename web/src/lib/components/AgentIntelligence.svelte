<script lang="ts">
	import { intelligenceQuery, type IntelligenceAgent } from '$lib/model/intelligence.svelte';
	import { MODEL_PRESETS } from '$lib/model/model-presets';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
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
						: (selected?.provider ?? ''))
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
						effective_model: source.view.default?.model ?? 'Choose a connection',
						harness: ''
					},
					...source.view.agents
				]
			: []
	);

	function label(id: string) {
		const c = source.view?.connections.find((c) => c.id === id);
		if (!c)
			return id.startsWith('account:') || id.startsWith('account-harness:')
				? 'Account connection unavailable'
				: id.replace('direct:', '').replace('harness:', '');
		if (c.id.startsWith('account-harness:'))
			return `${c.label ?? c.account_provider} · ${c.provider === 'codex' ? 'Codex' : 'Claude Agent'}`;
		if (c.id.startsWith('account:')) return `${c.label ?? c.provider} · Restless agent`;
		if (c.id.startsWith('harness:custom:')) return c.provider;
		return c.kind === 'harness'
			? c.provider === 'codex'
				? 'ChatGPT / Codex'
				: 'Claude Code'
			: (MODEL_PRESETS.find((p) => p.id === c.provider)?.name ?? c.provider);
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
			editing = '';
			notice = 'Intelligence saved. Applies to the next session.';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not save assignment.';
			await source.refresh();
		} finally {
			busy = false;
		}
	}
</script>

<section class="assignments" aria-labelledby="agent-assignments-title">
	<header>
		<h2 id="agent-assignments-title">Agent intelligence</h2>
		<span
			title="Choose a company-local sign-in or a granted account connection for each agent. Account sign-ins can power Codex or Claude Agent without copying their credentials into the company. Changes apply to the next session; removing access stops the next model request."
			>ⓘ</span
		>
	</header>
	{#if source.error}<p role="alert">
			Could not load agents. <button class="btn small" onclick={() => source.refresh()}
				>Retry</button
			>
		</p>
	{:else if !source.view}<p role="status">Loading agents…</p>
	{:else}
		{#if !source.view.connections.length}<p class="hint">
				Add a connection above to choose what powers each agent.
			</p>{/if}
		{#each rows as agent (agent.id)}
			<div class="agent-row">
				<div class="identity">
					<strong>{agent.id === 'exec' ? 'Exec' : agent.name}</strong><small>{agent.role}</small>
				</div>
				<div class="route">
					<span
						>{agent.assignment
							? label(agent.assignment.connection)
							: agent.id === 'default'
								? 'No default selected'
								: 'Use company default'}</span
					><small
						>{agent.assignment?.model ??
							agent.effective_model.slice(agent.effective_model.indexOf('/') + 1)}</small
					>
				</div>
				<button
					class="btn small"
					disabled={busy || !source.view.connections.length}
					onclick={() => edit(agent)}
					aria-label={`Change intelligence for ${agent.id === 'exec' ? 'Exec' : agent.name}`}
					>Change</button
				>
			</div>
			{#if editing === agent.id}
				<form
					class="agent-editor"
					onsubmit={(e) => {
						e.preventDefault();
						void save();
					}}
				>
					<label
						>Connection<select
							aria-label="Connection"
							bind:value={connection}
							onchange={choose}
							disabled={busy}
							>{#each source.view.connections as c}<option value={c.id}
									>{label(c.id)}{!c.loaded ? ' · restart pending' : ''}</option
								>{/each}</select
						></label
					>
					<label
						>Model<select
							aria-label="Model"
							value={custom ? '__custom' : model}
							disabled={busy}
							onchange={(e) => {
								custom = e.currentTarget.value === '__custom';
								if (!custom) model = e.currentTarget.value;
							}}
						>
							{#if model && !presets.some((m) => m.id === model)}<option value={model}
									>{model} (saved/custom)</option
								>{/if}
							{#each presets as m}<option value={m.id}>{m.name}</option>{/each}<option
								value="__custom">Custom model…</option
							>
						</select></label
					>
					{#if custom}<label
							>Custom model ID<input
								aria-label="Custom model ID"
								bind:value={model}
								required
								disabled={busy}
							/></label
						>{/if}
					{#if selected?.kind === 'harness'}<small
							>{selected.models?.length
								? 'Models offered by this connection.'
								: 'Catalog suggestions; availability is confirmed by the connection.'}</small
						>{/if}

					<div class="buttons">
						<button class="btn primary small" disabled={busy || !connection || !model.trim()}
							>{busy ? 'Saving…' : 'Save assignment'}</button
						><button class="btn small" type="button" disabled={busy} onclick={() => (editing = '')}
							>Cancel</button
						>{#if agent.assignment && agent.id !== 'default'}<button
								class="btn small"
								type="button"
								disabled={busy}
								onclick={() => save(true)}>Use company default</button
							>{/if}
					</div>
				</form>
			{/if}
		{/each}
	{/if}
	{#if error}<p role="alert">{error}</p>{/if}{#if notice}<p role="status">{notice}</p>{/if}
</section>

<style>
	.assignments {
		margin-top: var(--space-6);
		border-top: 1px solid var(--control-edge);
		padding-top: var(--space-5);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	h2 {
		font-size: var(--t-head);
		margin: 0;
	}
	.hint,
	small {
		color: var(--text-tertiary);
	}
	.agent-row {
		display: grid;
		grid-template-columns: minmax(90px, 0.8fr) minmax(100px, 1.4fr) auto;
		gap: var(--space-3);
		align-items: center;
		padding-block: var(--space-4);
		border-bottom: 1px solid var(--control-edge);
	}
	.identity,
	.route {
		display: grid;
		gap: var(--space-1);
		min-width: 0;
	}
	small {
		font-size: var(--t-label);
		overflow-wrap: anywhere;
	}
	.agent-editor {
		padding: var(--space-4);
		background: var(--surface-alt);
		display: grid;
		gap: var(--space-3);
	}
	label {
		display: grid;
		gap: var(--space-2);
	}
	input,
	select,
	button {
		font: inherit;
		min-height: 38px;
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--ink);
	}
	input,
	select {
		width: 100%;
		min-width: 0;
		box-sizing: border-box;
	}
	button {
		cursor: pointer;
	}
	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.buttons {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	[role='alert'] {
		color: var(--state-danger);
	}
	@container company-canvas (max-width: 480px) {
		.agent-row {
			grid-template-columns: 1fr auto;
		}
		.route {
			grid-column: 1;
			grid-row: 2;
		}
		.agent-row > button {
			grid-column: 2;
			grid-row: 1 / 3;
		}
	}
</style>
