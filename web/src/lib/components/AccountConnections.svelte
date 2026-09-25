<script lang="ts">
	import { onMount } from 'svelte';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import { getCompanies, type CompanyCatalogEntry } from '$lib/model/cockpit';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	const catalog = modelCatalog();
	type CompanyUse = { id: string; name: string; in_use?: boolean };
	type AccountConnection = {
		id: string;
		label: string;
		provider: string;
		kind?: 'api_key' | 'oauth';
		status?: 'present' | 'absent' | 'invalid';
		detail?: string | null;
		companies: CompanyUse[];
	};
	const providerOptions = [
		['anthropic', 'Anthropic'],
		['openai', 'OpenAI'],
		['google', 'Google Gemini'],
		['groq', 'Groq'],
		['mistral', 'Mistral'],
		['deepseek', 'DeepSeek'],
		['openrouter', 'OpenRouter'],
		['xai', 'xAI'],
		['zai', 'Z.ai'],
		['moonshot', 'Moonshot'],
		['litellm', 'OpenAI-compatible gateway']
	];
	let connections = $state<AccountConnection[]>([]);
	let loading = $state(true);
	let firstLoad = true;
	let error = $state('');
	let addOpen = $state(false);
	let busy = $state(false);
	let label = $state('');
	let provider = $state('anthropic');
	let secret = $state('');
	let kind = $state<'api_key' | 'oauth'>('api_key');
	let companies = $state<CompanyCatalogEntry[]>([]);
	let companyError = $state('');
	let managingId = $state('');
	let companyRevisions = $state<Record<string, string>>({});
	let selectedCompany = $state('');
	let selectedModel = $state('');
	let replaceRequired = $state(false);
	let busyCompany = $state(false);

	async function refresh() {
		loading = true;
		error = '';
		try {
			const response = await fetch('/api/connections', { cache: 'no-store' });
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not load account connections.');
			connections = body.connections ?? [];
			if (firstLoad && connections.length === 0) addOpen = true;
			firstLoad = false;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not load account connections.';
		} finally {
			loading = false;
		}
	}
	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		busy = true;
		error = '';
		try {
			const response = await fetch('/api/connections', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify(
					kind === 'oauth'
						? { label: label.trim(), provider, kind: 'oauth' }
						: { label: label.trim(), provider, secret }
				)
			});
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not save this connection.');
			label = '';
			secret = '';
			addOpen = false;
			await refresh();
		} catch (cause) {
			secret = '';
			error = cause instanceof Error ? cause.message : 'Could not save this connection.';
		} finally {
			busy = false;
		}
	}
	function statusText(item: AccountConnection) {
		const credential = item.kind === 'oauth' ? 'Sign-in' : 'Key';
		if (item.status === 'present') return item.kind === 'oauth' ? 'Signed in' : 'Key stored';
		if (item.status === 'invalid') return `${credential} unavailable`;
		return `${credential} missing`;
	}
	function providerName(id: string) {
		return providerOptions.find(([key]) => key === id)?.[1] ?? id;
	}
	function modelChoices(providerId: string) {
		return catalog.models(providerId);
	}
	function routeModel(providerId: string, model: string) {
		return model.startsWith(`${providerId}/`) ? model : `${providerId}/${model}`;
	}
	function defaultModel(providerId: string) {
		const models = modelChoices(providerId);
		const model = models.find((item) => 'default' in item && item.default)?.id ?? models[0]?.id;
		return model ? routeModel(providerId, model) : '';
	}
	function toggleManage(item: AccountConnection) {
		managingId = managingId === item.id ? '' : item.id;
		selectedCompany = '';
		selectedModel = defaultModel(item.provider);
		replaceRequired = false;
	}
	async function grant(item: AccountConnection, companyId: string, replace = false) {
		if (busyCompany) return;
		busyCompany = true;
		error = '';
		try {
			let revision = companyRevisions[companyId];
			if (!revision) {
				const stateResponse = await fetch(
					`/api/companies/${encodeURIComponent(companyId)}/provider`,
					{ cache: 'no-store' }
				);
				const state = await stateResponse.json();
				if (!stateResponse.ok)
					throw new Error(state.message ?? 'Could not load this project’s connection settings.');
				revision = state.revision;
				companyRevisions = { ...companyRevisions, [companyId]: revision };
			}
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/connections/${encodeURIComponent(item.id)}`,
				{
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({
						model: selectedModel || defaultModel(item.provider),
						make_default: false,
						revision,
						...(replace ? { replace_existing: true } : {})
					})
				}
			);
			const body = await response.json();
			if (
				response.status === 409 &&
				(body.code ?? body.error) === 'provider_conflict' &&
				!replace
			) {
				replaceRequired = true;
				selectedCompany = companyId;
				return;
			}
			if (!response.ok) throw new Error(body.message ?? 'Could not grant this project access.');
			companyRevisions = { ...companyRevisions, [companyId]: body.provider?.revision ?? revision };
			await refresh();
			selectedCompany = '';
			replaceRequired = false;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not grant this project access.';
		} finally {
			busyCompany = false;
		}
	}
	async function revoke(item: AccountConnection, company: CompanyUse) {
		if (busyCompany || company.in_use) return;
		busyCompany = true;
		error = '';
		try {
			let revision = companyRevisions[company.id];
			if (!revision) {
				const stateResponse = await fetch(
					`/api/companies/${encodeURIComponent(company.id)}/provider`,
					{ cache: 'no-store' }
				);
				const state = await stateResponse.json();
				if (!stateResponse.ok)
					throw new Error(state.message ?? 'Could not load this project’s connection settings.');
				revision = state.revision;
			}
			const response = await fetch(
				`/api/companies/${encodeURIComponent(company.id)}/connections/${encodeURIComponent(item.id)}`,
				{
					method: 'DELETE',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ revision })
				}
			);
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not remove this project’s access.');
			companyRevisions = { ...companyRevisions, [company.id]: body.provider?.revision ?? revision };
			await refresh();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not remove this project’s access.';
		} finally {
			busyCompany = false;
		}
	}

	onMount(() => {
		void refresh();
		void getCompanies()
			.then((rows) => {
				companies = rows.filter((company) => company.lifecycle_status === 'active');
			})
			.catch(() => {
				companyError = 'Projects could not be loaded. Reload to manage access.';
			});
	});
</script>

<svelte:head><title>Account connections — {PRODUCT_NAME}</title></svelte:head>
<main class="account-connections">
	<header class="page-head">
		<div class="page-intro">
			<h1
				title="Save API keys or register an existing host-broker sign-in. Grant each active project access here, and choose its model."
			>
				Connections
			</h1>
		</div>
		<button class="btn primary" onclick={() => (addOpen = !addOpen)}
			>{addOpen ? 'Close' : 'Add connection'}</button
		>
	</header>
	{#if error}<div class="error" role="alert">
			{error}<button class="btn small" onclick={() => void refresh()}>Try again</button>
		</div>{/if}
	{#if addOpen}
		<form class="add-form" onsubmit={create}>
			<h2>New provider connection</h2>
			<p>Register an API key or a provider sign-in already available in the host broker.</p>
			<div class="form-grid">
				<label
					>Connection type<select bind:value={kind}
						><option value="api_key">API key</option><option value="oauth"
							>Existing host broker sign-in</option
						></select
					></label
				><label
					>Provider<select bind:value={provider}
						>{#each providerOptions as [id, name]}<option value={id}>{name}</option>{/each}</select
					></label
				><label class="full"
					>Name<input
						bind:value={label}
						required
						maxlength="80"
						placeholder={kind === 'oauth' ? 'e.g. OpenAI host sign-in' : 'e.g. Anthropic team key'}
					/></label
				>{#if kind === 'api_key'}<label class="full"
						>API key<input
							bind:value={secret}
							type="password"
							required
							autocomplete="new-password"
							placeholder="Paste API key"
						/></label
					>{:else}<p class="fine-print full">
						This only registers a provider sign-in that already exists in the host broker. It does
						not start a new sign-in. Native Codex and Claude CLI sign-ins remain project-local.
					</p>{/if}
			</div>
			<div class="actions">
				<button class="btn primary" disabled={busy}
					>{busy ? 'Saving…' : kind === 'oauth' ? 'Add broker connection' : 'Save API key'}</button
				><button
					class="btn"
					type="button"
					disabled={busy}
					onclick={() => {
						addOpen = false;
						secret = '';
					}}>Cancel</button
				>
			</div>
		</form>
	{/if}
	{#if loading}<p class="loading" role="status">Loading connections…</p>
	{:else if connections.length}
		<section class="connection-list" aria-label="Account connections">
			{#each connections as item (item.id)}
				<article class="connection">
					<div class="connection-head">
						<div>
							<h2>{item.label}</h2>
							<span>{providerName(item.provider)}</span>
						</div>
						<span class="status" class:connected={item.status === 'present'}
							>{statusText(item)}</span
						>
					</div>
					{#if item.detail}<p class="connection-detail">{item.detail}</p>{/if}
					<div class="uses">
						<strong>Available to</strong>{#if item.companies.length}<div class="company-list">
								{#each item.companies as company (company.id)}<a
										href={`/${encodeURIComponent(company.id)}/company/provider`}
										>{company.name}<span aria-hidden="true">↗</span></a
									>{/each}
							</div>{:else}<span class="unused">No project access yet</span>{/if}
					</div>
					<div class="connection-controls">
						<span class="connection-kind"
							>{item.kind === 'oauth' ? 'Host broker sign-in' : 'API key'}</span
						><button class="text-button" onclick={() => toggleManage(item)}
							>{managingId === item.id ? 'Close access' : 'Manage project access'}</button
						>
					</div>
					{#if managingId === item.id}
						<div class="access-manager" aria-label={`Project access for ${item.label}`}>
							{#if companyError}<span class="replace-warning" role="alert">{companyError}</span
								>{/if}
							{#each companies as company (company.id)}
								{@const granted = item.companies.find((use) => use.id === company.id)}
								<div class="access-row">
									<strong>{company.name}</strong>
									{#if granted}<span class="access-state"
											>Available{granted.in_use ? ' · in use' : ''}</span
										><button
											class="text-button danger"
											disabled={busyCompany || granted.in_use}
											title={granted.in_use
												? 'Choose another model for this provider before removing access.'
												: 'Remove this project’s access'}
											onclick={() => void revoke(item, granted)}>Remove access</button
										>
									{:else if selectedCompany === company.id}
										<label class="model-picker"
											><span>Model</span><select
												bind:value={selectedModel}
												aria-label={`Model for ${item.label} in ${company.name}`}
												><option value="">Choose a model…</option
												>{#each modelChoices(item.provider) as model}<option
														value={routeModel(item.provider, model.id)}
														>{model.name ?? model.id}</option
													>{/each}</select
											></label
										>
										{#if replaceRequired}<span class="replace-warning" role="alert"
												>This replaces the current {providerName(item.provider)} connection for {company.name}.</span
											><button
												class="btn primary small"
												disabled={busyCompany || !selectedModel}
												onclick={() => void grant(item, company.id, true)}
												>{busyCompany ? 'Replacing…' : 'Confirm replacement'}</button
											><button
												class="text-button"
												disabled={busyCompany}
												onclick={() => {
													selectedCompany = '';
													replaceRequired = false;
												}}>Keep current</button
											>
										{:else}<button
												class="btn primary small"
												disabled={busyCompany || !selectedModel || item.status !== 'present'}
												onclick={() => void grant(item, company.id)}
												>{busyCompany ? 'Granting…' : 'Grant access'}</button
											><button
												class="text-button"
												disabled={busyCompany}
												onclick={() => (selectedCompany = '')}>Cancel</button
											>{/if}
									{:else}<button
											class="btn small"
											disabled={busyCompany || item.status !== 'present'}
											onclick={() => {
												selectedCompany = company.id;
												selectedModel = defaultModel(item.provider);
												replaceRequired = false;
											}}>Choose model and grant</button
										>{/if}
								</div>
							{/each}
							{#if !companies.length}<span class="unused">No active projects available.</span>{/if}
						</div>
					{/if}
				</article>
			{/each}
		</section>
	{:else if !addOpen}
		<div class="empty">
			<h2>No account connections yet</h2>
			<p>Save an API key or register an existing host broker sign-in, then grant project access.</p>
			<button class="btn primary" onclick={() => (addOpen = true)}>Add your first connection</button
			>
		</div>
	{/if}
</main>

<style>
	.account-connections {
		width: 100%;
		min-height: calc(100svh - 58px);
		margin: 0;
		padding: 42px clamp(20px, 5vw, 72px);
		box-sizing: border-box;
	}
	.page-head,
	.connection-head,
	.actions,
	.company-list,
	.uses {
		display: flex;
		align-items: center;
	}
	.page-head,
	.connection-head {
		justify-content: space-between;
		gap: var(--space-4);
	}
	h1 {
		margin: 0;
		font-size: var(--t-title);
		letter-spacing: -0.035em;
	}
	.add-form p,
	.connection-head span,
	.connection-detail,
	.unused,
	.fine-print {
		color: var(--text-tertiary);
		font-size: var(--t-label);
		line-height: 1.5;
	}
	.connection-list {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(100%, 360px), 1fr));
		gap: 14px;
		margin-top: 30px;
	}
	.connection,
	.add-form,
	.empty {
		padding: 20px;
		background: var(--surface-pane);
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
	}
	.connection {
		min-height: 150px;
		box-shadow: 0 1px 2px rgb(20 24 30 / 3%);
	}
	.connection-head h2,
	.add-form h2,
	.empty h2 {
		margin: 0 0 3px;
		font-size: var(--t-head);
	}
	.connection-head > div {
		display: grid;
		gap: 2px;
	}
	.status {
		flex: none;
		padding: 4px 8px;
		border-radius: 99px;
		background: var(--surface-alt);
	}
	.status.connected {
		color: var(--state-success);
	}
	.uses {
		flex-wrap: wrap;
		gap: var(--space-3);
		padding-top: var(--space-3);
		margin-top: var(--space-3);
		border-top: 1px solid var(--border);
		font-size: var(--t-label);
	}
	.uses > strong {
		color: var(--text-secondary);
		font-weight: 500;
	}
	.company-list {
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.company-list a {
		display: inline-flex;
		gap: var(--space-1);
		padding: 5px 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		color: var(--text-primary);
		text-decoration: none;
	}
	.company-list a:hover {
		border-color: var(--intent-conversation);
	}
	.connection-controls {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		margin-top: 12px;
	}
	.connection-kind {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.text-button {
		padding: 4px 0;
		border: 0;
		color: var(--intent-conversation);
		background: transparent;
		font: inherit;
		font-size: var(--t-label);
		cursor: pointer;
	}
	.text-button:hover {
		text-decoration: underline;
	}
	.text-button:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 3px;
	}
	.text-button.danger {
		color: var(--state-danger);
	}
	.access-manager {
		display: grid;
		gap: 8px;
		margin-top: 12px;
		padding-top: 12px;
		border-top: 1px solid var(--border);
	}
	.access-row {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 10px;
		min-height: 38px;
	}
	.access-row > strong {
		margin-right: auto;
		font-size: var(--t-label);
		font-weight: 500;
	}
	.access-state {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.model-picker {
		display: inline-grid;
		gap: 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.model-picker select {
		min-height: 36px;
		padding: 4px 8px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		color: var(--ink);
		background: var(--surface-pane);
		font: inherit;
	}
	.replace-warning {
		flex-basis: 100%;
		color: var(--state-danger);
		font-size: var(--t-label);
		line-height: 1.45;
	}
	.add-form {
		max-width: 900px;
		margin-top: 26px;
	}
	.add-form p {
		margin: var(--space-2) 0 var(--space-4);
	}
	.form-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
	}
	.form-grid label {
		display: grid;
		gap: var(--space-1);
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.form-grid .full {
		grid-column: 1 / -1;
	}
	.form-grid input,
	.form-grid select {
		width: 100%;
		min-width: 0;
		min-height: 40px;
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		color: var(--ink);
		background: var(--surface-pane);
		font: inherit;
	}
	.actions {
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-top: var(--space-4);
	}
	.fine-print {
		margin: var(--space-3) 0 0 !important;
	}
	.error {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-3);
		padding: var(--space-3);
		margin-top: var(--space-4);
		color: var(--state-danger);
		background: color-mix(in srgb, var(--state-danger) 7%, var(--surface-pane));
		border-radius: var(--radius-control);
	}
	.loading,
	.empty {
		max-width: 900px;
		margin-top: 30px;
		color: var(--text-secondary);
	}
	.empty p {
		max-width: 540px;
		color: var(--text-tertiary);
		line-height: 1.5;
	}
	@media (max-width: 620px) {
		.account-connections {
			min-height: auto;
			padding: 26px 20px 36px;
		}
		.page-head {
			align-items: flex-start;
			flex-direction: column;
		}
		.page-head > button {
			width: 100%;
		}
		.form-grid {
			grid-template-columns: 1fr;
		}
		.form-grid .full {
			grid-column: auto;
		}
		.connection-head {
			align-items: flex-start;
			flex-direction: row;
		}
	}
</style>
