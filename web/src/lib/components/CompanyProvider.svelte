<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	const catalog = modelCatalog();
	import HarnessConnections from './HarnessConnections.svelte';
	import HarnessDiagnostics from './HarnessDiagnostics.svelte';
	import CustomHarnesses from './CustomHarnesses.svelte';
	import AgentIntelligence from './AgentIntelligence.svelte';
	import CopyCompanySetting from './CopyCompanySetting.svelte';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { getCompanies, type CompanyCatalogEntry } from '$lib/model/cockpit';
	let { companyId }: { companyId: string } = $props();
	const intelligence = $derived(intelligenceQuery(companyId));
	let editorOpen = $state(false);
	type Connection = {
		provider: string;
		reference: string | null;
		credential_status: string;
		credential_detail: string | null;
		gateway_loaded: boolean;
		shareable_api_key?: boolean;
	};
	type ProviderStatus = {
		revision: string;
		primary_provider: string;
		connections: Connection[];
		infisical_configured: boolean;
		infisical_status: string;
		infisical_detail: string | null;
		startup_issue: string | null;
	};
	const labels: Record<string, string> = {
		anthropic: 'Anthropic',
		openai: 'OpenAI',
		'openai-codex': 'OpenAI Codex',
		google: 'Google Gemini',
		groq: 'Groq',
		mistral: 'Mistral',
		deepseek: 'DeepSeek',
		openrouter: 'OpenRouter',
		xai: 'xAI',
		zai: 'Z.ai',
		moonshot: 'Moonshot',
		litellm: 'OpenAI-compatible gateway'
	};
	let status = $state<ProviderStatus | null>(null);
	let selected = $state('');
	let mode = $state('infisical');
	let reference = $state('');
	let secret = $state('');
	let error = $state('');
	let notice = $state('');
	let busy = $state(false);
	let refreshing = $state(false);
	let edited = false;
	let editor = $state<HTMLElement>();
	let requestSequence = 0;
	type ReusableConnection = {
		id: string;
		label: string;
		provider: string;
		kind?: 'api_key' | 'oauth';
		status?: 'present' | 'absent' | 'invalid';
		detail?: string | null;
		companies: { id: string; name: string; in_use?: boolean }[];
	};
	let reusableConnections = $state<ReusableConnection[]>([]);
	let accountScope = $state<'account' | 'company'>('account');
	let manageUrl = $state('/account/settings/connections');
	let accountError = $state('');
	let accountBusy = $state(false);
	let accountLoading = $state(false);
	let addConnectionOpen = $state(false);
	let connectionLabel = $state('');
	let connectionProvider = $state('anthropic');
	let connectionModel = $state('');
	let connectionMakeDefault = $state(false);
	let connectionSecret = $state('');
	let grantSelection = $state('');
	let replaceGrantId = $state('');
	let grantMakeDefault = $state(false);
	let grantModels = $state<Record<string, string>>({});
	let companies = $state<CompanyCatalogEntry[]>([]);
	const otherCompanies = $derived(companies.filter((company) => company.id !== companyId));
	let importOpen = $state(false);
	let importSource = $state('');
	let importProviders = $state<Connection[]>([]);
	let importProvider = $state('');
	let importLabel = $state('');
	const endpoint = $derived(`/api/companies/${encodeURIComponent(companyId)}/provider`);
	const connection = $derived(status?.connections.find((c) => c.provider === selected));
	const savedCount = $derived(status?.connections.filter((c) => c.reference).length ?? 0);
	/* Advanced stays folded until it holds something of this company's own, or a
	 * link asks for the harnesses inside it. */
	let advancedOpen = $state(false);
	$effect(() => {
		if (savedCount || editorOpen) advancedOpen = true;
	});
	onMount(() => {
		if (location.hash === '#harnesses') advancedOpen = true;
	});
	function defaultReference(provider: string) {
		return `infisical:/companies/${companyId}/MODEL_${provider.replace(/[^a-zA-Z0-9_]/g, '_')}_API_KEY`;
	}
	function choose(provider: string, reveal = false) {
		selected = provider;
		edited = false;
		secret = '';
		error = '';
		notice = '';
		const row = status?.connections.find((c) => c.provider === provider);
		mode =
			row?.reference?.startsWith('omp-oauth:')
				? 'oauth'
				: row?.reference?.startsWith('env:')
					? 'env'
					: 'infisical';
		if (reveal) editorOpen = true;
		if (reveal) void tick().then(() => editor?.scrollIntoView({ block: 'start' }));
		reference =
			row?.reference ?? (mode === 'oauth' ? `omp-oauth:${provider}` : defaultReference(provider));
	}
	async function refresh() {
		const sequence = ++requestSequence;
		refreshing = true;
		try {
			const response = await fetch(endpoint, { cache: 'no-store' });
			const body = await response.json();
			if (sequence !== requestSequence) return;
			if (!response.ok) throw new Error(body.message ?? 'Could not read connections.');
			status = body;
			error = '';
			if (!selected && !editorOpen) {
				choose(body.primary_provider === 'unconfigured' ? '' : body.primary_provider);
			} else if (!edited && !busy) {
				const previousNotice = notice;
				choose(selected);
				notice = previousNotice;
			}
		} catch (cause) {
			if (sequence === requestSequence)
				error = cause instanceof Error ? cause.message : 'Could not read connections.';
		} finally {
			if (sequence === requestSequence) refreshing = false;
		}
	}
	function selectMode() {
		edited = true;
		secret = '';
		error = '';
		notice = '';
		reference =
			mode === 'oauth'
				? `omp-oauth:${selected}`
				: mode === 'env'
					? 'env:'
					: defaultReference(selected);
	}
	async function connect(event?: SubmitEvent, disconnect = false) {
		event?.preventDefault();
		if (!status || busy || refreshing) return;
		busy = true;
		error = '';
		notice = '';
		++requestSequence;
		try {
			const response = await fetch(endpoint, {
				method: 'PUT',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({
					provider: selected,
					model: catalog.models(selected)[0]?.id,
					reference,
					secret: secret || undefined,
					revision: status.revision,
					disconnect
				})
			});
			secret = '';
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not save this connection.');
			status = body;
			await intelligence.refresh();
			editorOpen = false;
			choose(selected);
			notice = disconnect
				? 'Disconnected. Restart Restless to unload this route.'
				: 'Connection saved. Restart Restless to activate this provider.';
		} catch (cause) {
			secret = '';
			error = cause instanceof Error ? cause.message : 'Could not save this connection.';
		} finally {
			busy = false;
		}
	}
	function stateLabel(row: Connection) {
		if (row.credential_status === 'invalid') return 'Connection error';
		if (row.credential_status !== 'present')
			return row.reference ? 'Credential missing' : 'Not connected';
		return row.gateway_loaded ? 'Route loaded' : 'Credential available';
	}
	function tone(row: Connection) {
		return row.credential_status === 'invalid'
			? 'error'
			: row.credential_status === 'present'
				? 'success'
				: row.reference
					? 'warning'
					: 'neutral';
	}
	async function refreshReusableConnections() {
		accountLoading = true;
		accountError = '';
		try {
			const [connectionResponse, companyRows] = await Promise.all([
				fetch('/api/connections', { cache: 'no-store' }),
				getCompanies()
			]);
			const body = await connectionResponse.json();
			if (!connectionResponse.ok)
				throw new Error(body.message ?? 'Could not load reusable connections.');
			reusableConnections = body.connections ?? [];
			accountScope = body.scope === 'company' ? 'company' : 'account';
			manageUrl = accountScope === 'company' ? (body.manage_url ?? '/account/settings/connections') : '/account/settings/connections';
			companies = companyRows.filter((company) => company.lifecycle_status === 'active');
		} catch (cause) {
			accountError = cause instanceof Error ? cause.message : 'Could not load reusable connections.';
		} finally {
			accountLoading = false;
		}
	}
	function modelChoices(provider: string) {
		return catalog.models(provider);
	}
	function routeModel(provider: string, model: string) {
		return model.startsWith(`${provider}/`) ? model : `${provider}/${model}`;
	}
	function keyStatus(connection: ReusableConnection) {
		const noun = connection.kind === 'oauth' ? 'Sign-in' : 'Key';
		if (connection.status === 'present') return connection.kind === 'oauth' ? 'Signed in' : 'Key stored';
		if (connection.status === 'invalid') return `${noun} unavailable`;
		return `${noun} missing`;
	}
	function defaultModel(provider: string) {
		const models = modelChoices(provider);
		const model = models.find((item) => 'default' in item && item.default)?.id ?? models[0]?.id;
		return model ? routeModel(provider, model) : '';
	}
	function modelForGrant(connection: ReusableConnection) {
		return grantModels[connection.id] ?? defaultModel(connection.provider);
	}
	function beginGrant(connection: ReusableConnection) {
		grantModels[connection.id] = defaultModel(connection.provider);
		grantSelection = connection.id;
		grantMakeDefault = false;
		replaceGrantId = '';
		accountError = '';
	}
	function toggleAddConnection() {
		addConnectionOpen = !addConnectionOpen;
		if (addConnectionOpen) {
			connectionModel = defaultModel(connectionProvider);
			connectionMakeDefault = false;
			accountError = '';
		}
	}
	async function requestGrant(id: string, model: string, replaceExisting = false, makeDefault = false) {
		if (!status) throw new Error('Company settings are still loading. Try again in a moment.');
		const response = await fetch(`/api/companies/${encodeURIComponent(companyId)}/connections/${encodeURIComponent(id)}`, {
			method: 'POST', headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ model, make_default: makeDefault, revision: status.revision, ...(replaceExisting ? { replace_existing: true } : {}) })
		});
		const body = await response.json();
		if (response.status === 409 && (body.code ?? body.error) === 'provider_conflict' && !replaceExisting) return { replaceRequired: true as const };
		if (!response.ok) throw new Error(body.message ?? 'Could not use this connection in the company.');
		return { replaceRequired: false as const, provider: body.provider };
	}
	async function createReusableConnection(event: SubmitEvent) {
		event.preventDefault();
		if (accountBusy) return;
		accountBusy = true;
		accountError = '';
		try {
			const response = await fetch('/api/connections', {
				method: 'POST', headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ label: connectionLabel.trim(), provider: connectionProvider, secret: connectionSecret })
			});
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not save this connection.');
			const created = body.connection ?? body;
			await refreshReusableConnections();
			if (!status || !created.id) throw new Error('Connection saved, but could not enable it in this company yet. Choose it from Account API keys to finish.');
			const grantResult = await requestGrant(created.id, connectionModel || defaultModel(connectionProvider), false, connectionMakeDefault);
			connectionSecret = '';
			connectionLabel = '';
			connectionModel = '';
			addConnectionOpen = false;
			await Promise.all([refreshReusableConnections(), intelligence.refresh()]);
			if (grantResult.replaceRequired) {
				grantSelection = created.id;
				replaceGrantId = created.id;
				accountError = '';
			} else {
				status = grantResult.provider;
				notice = 'Connection created and made available to this company.';
			}
		} catch (cause) {
			connectionSecret = '';
			accountError = cause instanceof Error ? cause.message : 'Could not save this connection.';
		} finally { accountBusy = false; }
	}
	async function grantConnection(connection: ReusableConnection) {
		if (!status || accountBusy) return;
		accountBusy = true;
		accountError = '';
		try {
			const result = await requestGrant(connection.id, modelForGrant(connection), replaceGrantId === connection.id, grantMakeDefault);
			if (result.replaceRequired) {
				replaceGrantId = connection.id;
				accountError = '';
				return;
			}
			status = result.provider;
			grantSelection = '';
			replaceGrantId = '';
			await Promise.all([refreshReusableConnections(), intelligence.refresh()]);
		} catch (cause) {
			accountError = cause instanceof Error ? cause.message : 'Could not use this connection in the company.';
		} finally { accountBusy = false; }
	}
	async function revokeConnection(connection: ReusableConnection) {
		if (!status || accountBusy) return;
		accountBusy = true;
		accountError = '';
		try {
			const response = await fetch(`/api/companies/${encodeURIComponent(companyId)}/connections/${encodeURIComponent(connection.id)}`, {
				method: 'DELETE', headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ revision: status.revision })
			});
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not remove this company’s access.');
			status = body.provider;
			await Promise.all([refreshReusableConnections(), intelligence.refresh()]);
		} catch (cause) {
			accountError = cause instanceof Error ? cause.message : 'Could not remove this company’s access.';
		} finally { accountBusy = false; }
	}
	async function loadImportProviders() {
		importProviders = [];
		importProvider = '';
		if (!importSource || importSource === companyId) return;
		try {
			const response = await fetch(`/api/companies/${encodeURIComponent(importSource)}/provider`, { cache: 'no-store' });
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not inspect that company.');
			importProviders = (body.connections ?? []).filter((row: Connection) => row.shareable_api_key && row.credential_status === 'present');
			importProvider = importProviders[0]?.provider ?? '';
			accountError = importProviders.length ? '' : 'No available API-key connections were found in that company.';
		} catch (cause) {
			accountError = cause instanceof Error ? cause.message : 'Could not inspect that company.';
		}
	}
	async function importReusableConnection() {
		if (accountBusy || !importSource || !importProvider) return;
		accountBusy = true;
		accountError = '';
		try {
			const response = await fetch('/api/connections/import', {
				method: 'POST', headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ source_company: importSource, provider: importProvider, label: importLabel.trim() || `${labels[importProvider] ?? importProvider} API` })
			});
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not make this connection reusable.');
			const imported = body.connection ?? body;
			await refreshReusableConnections();
			if (!status || !imported.id) throw new Error('Connection saved to your account, but it could not be granted to this company. Choose it from Account API keys to finish.');
			const grantResult = await requestGrant(imported.id, defaultModel(importProvider));
			importOpen = false;
			importSource = '';
			importProviders = [];
			await Promise.all([refreshReusableConnections(), intelligence.refresh()]);
			if (grantResult.replaceRequired) {
				grantSelection = imported.id;
				replaceGrantId = imported.id;
				accountError = '';
			} else {
				status = grantResult.provider;
				notice = 'Connection imported and made available to this company.';
			}
		} catch (cause) {
			accountError = cause instanceof Error ? cause.message : 'Could not make this connection reusable.';
		} finally { accountBusy = false; }
	}
	onMount(() => {
		void refresh();
		void refreshReusableConnections();
		return () => {
			++requestSequence;
			secret = '';
		};
	});
</script>

<svelte:head><title>Intelligence provider — Company</title></svelte:head>
<div class="company-page provider-page">
	<header class="company-page-head">
		<h1 title="Use an account sign-in or API key in this company. Model choices stay company-specific.">Intelligence</h1>
		<div class="provider-head-actions">
			<CopyCompanySetting {companyId} setting="models" label="Model choices" oncopied={async () => { await Promise.all([refresh(), intelligence.refresh()]); }} />
			<a class="account-link" href={manageUrl}>{accountScope === 'company' ? 'Open account to manage connections' : 'Manage account connections'} <span aria-hidden="true">↗</span></a>
		</div>
	</header>
	<AgentIntelligence {companyId} />
	<section class="connect-section" aria-labelledby="connect-title">
		<h2 id="connect-title" title="Sign in with a subscription, or use an API key. Either can power any agent above.">Connect</h2>
		<HarnessConnections {companyId} />
	<section class="reuse-panel" aria-labelledby="account-connections-title">
		<div class="reuse-section-head">
			<div><h3 id="account-connections-title" title="Sign-ins and API keys saved in your account. Each company needs an explicit grant to use one.">Account connections</h3></div>
			{#if accountScope === 'account'}<button class="btn small" disabled={accountBusy} onclick={toggleAddConnection}>
				{addConnectionOpen ? 'Close' : 'Add API key'}
			</button>{/if}
		</div>
		{#if accountLoading}<p class="inline-status" role="status">Loading account connections…</p>
		{:else if reusableConnections.length}
			<div class="reuse-list">
				{#each reusableConnections as item (item.id)}
					{@const companyGrant = item.companies.find((company) => company.id === companyId)}
					<div class="reuse-row">
						<div class="reuse-identity"><strong>{item.label}</strong><span>{labels[item.provider] ?? item.provider} · {item.kind === 'oauth' ? 'Account sign-in' : 'API key'}</span></div>
						{#if companyGrant}
							<span class="grant-state">Available to this company</span>
							{#if companyGrant.in_use}<span class="grant-count" title="Choose another model for this provider before removing access.">In use</span>{:else if accountScope === 'account'}<button class="text-button danger" disabled={accountBusy || !status} onclick={() => revokeConnection(item)}>Remove access</button>{/if}
						{:else if grantSelection === item.id}
							<label class="model-picker"><span>Model for this connection</span><select aria-label={`Model for ${item.label}`} value={modelForGrant(item)} onchange={(event) => (grantModels[item.id] = event.currentTarget.value)}>
								{#each modelChoices(item.provider) as model}<option value={routeModel(item.provider, model.id)}>{model.name ?? model.id}</option>{/each}
							</select></label>
							{#if status?.primary_provider === 'unconfigured'}<p class="default-note">The first usable connection becomes this company’s default.</p>{:else}<label class="default-option"><input type="checkbox" bind:checked={grantMakeDefault} /><span>Make this the company default</span></label>{/if}
							{#if replaceGrantId === item.id}<p class="replace-confirm" role="alert">This company already has a {labels[item.provider] ?? item.provider} connection. Replace it with <strong>{item.label}</strong> here?</p><button class="btn primary small" disabled={accountBusy || !status || !modelForGrant(item) || item.status !== 'present'} onclick={() => grantConnection(item)}>{accountBusy ? 'Switching…' : 'Replace this company’s connection'}</button><button class="text-button" disabled={accountBusy} onclick={() => { replaceGrantId = ''; grantSelection = ''; }}>Keep current</button>
							{:else}<button class="btn primary small" disabled={accountBusy || !status || !modelForGrant(item) || item.status !== 'present'} onclick={() => grantConnection(item)}>{accountBusy ? 'Saving…' : 'Use in this company'}</button>{/if}
							<button class="text-button" disabled={accountBusy} onclick={() => { grantSelection = ''; replaceGrantId = ''; }}>Cancel</button>
						{:else}
							<span class="grant-count">{keyStatus(item)}{item.companies.length ? ` · used by ${item.companies.length}` : ''}</span>
							<button class="btn primary small" disabled={accountBusy || !status || item.status !== 'present'} onclick={() => beginGrant(item)}>Use</button>
						{/if}
						{#if item.status !== 'present'}<span class="connection-unavailable" title={item.detail ?? 'Check this connection in account settings.'}>Unavailable</span>{/if}
					</div>
				{/each}
			</div>
		{:else}
			<div class="reuse-empty"><p>No account connection is available to this company yet.</p><span>{accountScope === 'company' ? 'Open your account, then choose Account settings to grant one.' : 'Sign in or save an API key in your account, then grant it to this company.'}</span></div>
		{/if}
		{#if addConnectionOpen && accountScope === 'account'}
			<form class="add-form" onsubmit={createReusableConnection}>
				<h3>Save an API connection</h3>
				<div class="form-grid"><label>Provider<select bind:value={connectionProvider} onchange={() => (connectionModel = defaultModel(connectionProvider))}>{#each Object.entries(labels).filter(([id]) => id !== 'openai-codex') as [id, label]}<option value={id}>{label}</option>{/each}</select></label>
					<label>Name<input bind:value={connectionLabel} placeholder="e.g. Anthropic team key" required maxlength="80" /></label>
					<label class="wide">API key<input type="password" bind:value={connectionSecret} autocomplete="new-password" placeholder="Paste API key" required /></label>
					<label class="wide">Model<select bind:value={connectionModel}><option value="">Choose a model…</option>{#each modelChoices(connectionProvider) as model}<option value={routeModel(connectionProvider, model.id)}>{model.name ?? model.id}</option>{/each}</select></label>
					{#if status?.primary_provider === 'unconfigured'}<p class="default-note wide">The first usable connection becomes this company’s default.</p>{:else}<label class="default-option wide"><input type="checkbox" bind:checked={connectionMakeDefault} /><span>Make this the company default</span></label>{/if}
				</div>
				<div class="inline-actions"><button class="btn primary small" disabled={accountBusy}>{accountBusy ? 'Saving…' : 'Save connection'}</button><button class="text-button" type="button" disabled={accountBusy} onclick={() => { addConnectionOpen = false; connectionSecret = ''; }}>Cancel</button></div>
				<p class="form-note">One API key per provider is shared across companies. Each company needs its own grant. This is a stored key check, not a provider sign-in test.</p>
			</form>
		{/if}
		{#if accountScope === 'company'}
			<p class="form-note">Account connections and grants are managed by the account owner.</p>
		{:else if !otherCompanies.length}
			<!-- Nothing to bring in until a second company exists. -->
		{:else if !importOpen}
			<button class="text-button import-link" disabled={accountBusy} onclick={() => { importOpen = true; accountError = ''; }}>Bring in an API connection from another company</button>
		{:else}
			<div class="import-form"><h3>Bring in an existing connection</h3><p>Restless creates an account-level copy of the key. The original company keeps its connection. Only one API key per provider can be used across companies.</p>
				<label>Source company<select bind:value={importSource} onchange={() => void loadImportProviders()}><option value="">Choose a company…</option>{#each companies.filter((company) => company.id !== companyId) as company}<option value={company.id}>{company.name}</option>{/each}</select></label>
				{#if importProviders.length}<label>API connection<select bind:value={importProvider} onchange={() => (importLabel = '')}>{#each importProviders as item}<option value={item.provider}>{labels[item.provider] ?? item.provider}</option>{/each}</select></label><label>Name<input bind:value={importLabel} placeholder="Optional name" maxlength="80" /></label>{/if}
				<div class="inline-actions"><button class="btn primary small" disabled={accountBusy || !importProvider} onclick={() => void importReusableConnection()}>{accountBusy ? 'Importing…' : 'Save as reusable connection'}</button><button class="text-button" disabled={accountBusy} onclick={() => { importOpen = false; importSource = ''; importProviders = []; }}>Cancel</button></div>
			</div>
		{/if}
		{#if accountError}<p class="inline-error" role="alert">{accountError}</p>{/if}
	</section>
	</section>
	{#if error}<p role="alert">{error}</p>{/if}{#if notice}<p class="notice" role="status">{notice}</p>{/if}
	<details class="advanced-settings" bind:open={advancedOpen}>
		<summary>Advanced</summary>
	<section class="company-only-settings">
		<header>
			<h3 title="These keys belong only to this company. Use account API keys when several companies need the same provider.">Company-only API keys</h3>
			<button class="btn small" disabled={!status || busy} onclick={() => { choose(''); editorOpen = true; }}>Add key</button>
		</header>
	<div class="storage">
		<span
			class:good={status?.infisical_status === 'present'}
			title={status?.infisical_detail ??
				'API keys are stored securely in Infisical; only references are saved in company settings.'}
			>● {status?.infisical_status === 'present'
				? 'Secure key storage ready'
				: status
					? 'Key storage unavailable'
					: 'Checking key storage…'}</span
		><a href={`/${companyId}/company/vault`}>Vault →</a><button
			class="text-button"
			disabled={busy || refreshing}
			onclick={() => refresh()}>{refreshing ? 'Checking…' : 'Refresh'}</button
		>
	</div>
	{#if status}
		{#if savedCount}
			<section aria-label="Your connections" class="connections">
				{#each status.connections.filter((c) => c.reference) as row (row.provider)}
					<div class="connection-row">
						<strong>{labels[row.provider] ?? row.provider}</strong><span class="badge {tone(row)}"
							>● {stateLabel(row)}</span
						><button
							class="btn small"
							disabled={busy}
							onclick={() => choose(row.provider, true)}
							aria-label={`Edit ${labels[row.provider] ?? row.provider}`}>Edit</button
						>
					</div>
				{/each}
			</section>
		{:else}<p class="empty">None yet.</p>{/if}
	{/if}
	{#if editorOpen && status}
		<section class="connection-editor" bind:this={editor} aria-label="Connection settings">
			<form class="credentials" onsubmit={(e) => connect(e)}>
				<label for="provider-choice">Provider</label><select
					id="provider-choice"
					required
					disabled={busy}
					value={selected ? (labels[selected] ? selected : 'custom') : ''}
					onchange={(e) => choose(e.currentTarget.value)}
				>
					<option value="" disabled>Choose a provider…</option>
					{#each Object.entries(labels).filter(([id]) => id !== 'openai-codex' || status?.connections.some((c) => c.provider === id && c.reference)) as [id, label]}<option
							value={id}>{label}</option
						>{/each}<option value="custom">Custom provider…</option>
				</select>
				{#if selected && !labels[selected]}<label for="custom-provider">Provider ID</label><input
						id="custom-provider"
						placeholder="Provider ID"
						value={selected === 'custom' ? '' : selected}
						oninput={(e) => choose(e.currentTarget.value || 'custom')}
						pattern="[a-zA-Z0-9_.\-]+"
						required
						disabled={busy}
					/>{/if}
				{#if mode === 'infisical'}<label for="provider-key">API key</label><input
						id="provider-key"
						type="password"
						bind:value={secret}
						oninput={() => (edited = true)}
						autocomplete="new-password"
						placeholder={connection?.reference
							? 'Paste a replacement key, or leave blank to keep it'
							: 'Paste your API key'}
						disabled={busy || status.infisical_status !== 'present'}
						required={!connection?.reference}
					/>{/if}
				{#if connection?.credential_detail}<p role="alert">{connection.credential_detail}</p>{/if}
				<details>
					<summary>Advanced settings</summary><label for="provider-auth">Credential source</label
					><select id="provider-auth" bind:value={mode} onchange={selectMode} disabled={busy}
						><option value="infisical">API key stored securely</option><option value="env"
							>Existing host environment variable</option
						>{#if connection?.reference?.startsWith('omp-oauth:')}<option value="oauth">Existing company OAuth reference</option>{/if}</select
					>
					<label for="provider-reference">Credential reference</label><input
						id="provider-reference"
						bind:value={reference}
						oninput={() => (edited = true)}
						disabled={busy}
						required
						spellcheck="false"
					/>
					{#if mode === 'oauth'}<p>
						This existing company reference is kept for compatibility. Grant reusable sign-ins from Account → Connections.
						</p>{/if}
				</details>
				<div class="actions">
					<button
						class="btn primary small"
						disabled={busy ||
							refreshing ||
							!selected ||
							selected === 'custom' ||
							(mode === 'infisical' && status.infisical_status !== 'present')}
						>{busy ? 'Saving…' : connection?.reference ? 'Save connection' : 'Connect'}</button
					><button
						class="btn small"
						type="button"
						disabled={busy}
						onclick={() => {
							editorOpen = false;
							secret = '';
						}}>Cancel</button
					>{#if connection?.reference}<button
							type="button"
							class="text-button danger"
							disabled={busy}
							onclick={() => connect(undefined, true)}>Disconnect</button
						>{/if}
				</div>
			</form>
		</section>
	{/if}
	</section>
	<div
		class="catalog-status"
		title="Model suggestions refresh hourly from models.dev. Catalog inclusion does not confirm account access or harness compatibility. Local gateways keep bundled suggestions and custom IDs. Saved model choices are never changed automatically."
	>
		<span
			>{catalog.pending
				? 'Refreshing model catalog…'
				: catalog.failed
					? catalog.updatedAt
						? 'Using cached model catalog'
						: 'Using bundled model suggestions'
					: catalog.updatedAt
						? 'Model catalog updated ' + new Date(catalog.updatedAt).toLocaleString()
						: 'Bundled model suggestions'}</span
		>
		<button class="text-button" disabled={catalog.pending} onclick={() => catalog.refresh()}
			>Refresh models</button
		>
	</div>
		<HarnessDiagnostics {companyId} />
		<div id="harnesses"><CustomHarnesses {companyId} /></div>
	</details>
</div>

<style>
	.provider-page {
		box-sizing: border-box;
	}
	.provider-head-actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-3);
	}
	.reuse-panel {
		padding: var(--space-5);
		margin-bottom: var(--space-6);
		background: var(--surface-pane);
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		box-shadow: 0 1px 2px color-mix(in srgb, var(--ink) 4%, transparent);
	}
	.reuse-section-head,
	.inline-actions,
	.reuse-row,
	.reuse-identity,
	.grant-state,
	.grant-count {
		display: flex;
		align-items: center;
	}
	.reuse-section-head,
	.reuse-row {
		justify-content: space-between;
		gap: var(--space-4);
	}
	.reuse-empty span,
	.form-note,
	.import-form > p {
		display: block;
		margin: var(--space-1) 0 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		line-height: 1.5;
	}
	.account-link {
		flex: none;
		padding: var(--space-2) 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.account-link:hover { color: var(--intent-conversation); }
	.reuse-section-head {
		padding-block: 0 var(--space-3);
	}
	.reuse-section-head h3 { margin: 0; font-size: var(--t-head); }
	.reuse-list { border-top: 1px solid var(--border); }
	.reuse-row {
		min-height: 56px;
		padding-block: var(--space-2);
		border-bottom: 1px solid var(--border);
		flex-wrap: wrap;
	}
	.reuse-identity { gap: var(--space-3); min-width: 160px; }
	.reuse-identity span,
	.grant-count,
	.grant-state { color: var(--text-tertiary); font-size: var(--t-label); }
	.grant-state { color: var(--state-success); }
	.model-picker { display: flex; align-items: center; gap: var(--space-2); color: var(--text-secondary); font-size: var(--t-label); }
	.model-picker select { width: auto; min-width: 180px; }
	.default-option { display: flex; align-items: center; gap: var(--space-2); color: var(--text-secondary); font-size: var(--t-label); }
	.default-option input { width: 16px; min-height: 16px; height: 16px; padding: 0; accent-color: var(--intent-conversation); }
	.default-note { flex: 1 1 100%; margin: 0; color: var(--text-tertiary); font-size: var(--t-label); }
	.reuse-empty { padding: var(--space-4) 0; }
	.reuse-empty p { margin: 0; color: var(--text-secondary); }
	.replace-confirm { flex: 1 1 100%; margin: 0; color: var(--company-amber); font-size: var(--t-label); line-height: 1.45; }
	.add-form,
	.import-form {
		padding: var(--space-4);
		margin-top: var(--space-3);
		background: var(--surface-alt);
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
	}
	.add-form h3,
	.import-form h3 { margin: 0 0 var(--space-3); font-size: var(--t-head); }
	.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-3); }
	.form-grid label,
	.import-form > label { display: grid; gap: var(--space-1); color: var(--text-secondary); font-size: var(--t-label); }
	.form-grid .wide { grid-column: 1 / -1; }
	.inline-actions { flex-wrap: wrap; gap: var(--space-2); margin-top: var(--space-3); }
	.form-note { margin-top: var(--space-2); }
	.import-link { margin-top: var(--space-3); color: var(--text-secondary); }
	.import-form > label { margin-top: var(--space-3); }
	.inline-status { color: var(--text-tertiary); }
	.inline-error { padding: var(--space-3); color: var(--state-danger); background: color-mix(in srgb, var(--state-danger) 7%, var(--surface-pane)); border-radius: var(--radius-control); }
	header,
	.storage,
	.actions,
	.connection-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.catalog-status {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		font-size: var(--t-label);
		color: var(--text-tertiary);
		margin-block: var(--space-4);
	}
	header,
	.storage {
		justify-content: space-between;
		flex-wrap: wrap;
	}
	h1 {
		font-size: var(--t-title);
		margin: 0;
	}
	h2 {
		font-size: var(--t-head);
		margin: 0;
	}
	.storage {
		margin-block: var(--space-4) var(--space-5);
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.good {
		color: var(--state-success);
	}
	button,
	input,
	select {
		font: inherit;
		color: var(--ink);
		background: var(--surface-pane);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		min-height: 40px;
		padding: var(--space-2) var(--space-3);
		box-sizing: border-box;
	}
	button {
		cursor: pointer;
		flex: none;
	}
	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.text-button {
		border: 0;
		background: transparent;
		padding-inline: var(--space-1);
	}
	.connection-row {
		padding-block: var(--space-3);
		border-bottom: 1px solid var(--control-edge);
		flex-wrap: wrap;
	}
	.connection-row strong {
		flex: 1;
	}
	.badge {
		font-size: var(--t-label);
	}
	.success {
		color: var(--state-success);
	}
	.warning {
		color: var(--company-amber);
	}
	.error,
	.danger,
	[role='alert'] {
		color: var(--state-danger);
	}
	.neutral {
		color: var(--text-tertiary);
	}
	.empty {
		margin-block: var(--space-5);
	}
	p {
		color: var(--text-tertiary);
		line-height: 1.5;
	}
	.connection-editor {
		background: var(--surface-pane);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-pane);
		padding: var(--space-5);
		margin-block: var(--space-4);
	}
	.credentials {
		display: grid;
		gap: var(--space-2);
	}
	input,
	select {
		width: 100%;
		min-width: 0;
	}
	label {
		margin-top: var(--space-2);
	}
	details {
		margin-block: var(--space-3);
	}
	details label {
		display: block;
		margin-bottom: var(--space-2);
	}
	summary {
		cursor: pointer;
		color: var(--text-secondary);
	}
	.actions {
		flex-wrap: wrap;
		margin-top: var(--space-3);
	}
	.connect-section {
		margin-top: calc(var(--space-6) * 1.5);
	}
	.connect-section > h2 {
		margin-bottom: var(--space-4);
	}
	.connect-section .reuse-panel {
		margin-top: var(--space-4);
	}
	h3 {
		font-size: var(--t-head);
		margin: 0;
	}
	.advanced-settings {
		margin-block: var(--space-6) 0;
		padding-top: var(--space-4);
		border-top: 1px solid var(--control-edge);
	}
	.advanced-settings > summary {
		padding-block: var(--space-2);
		font-weight: 500;
	}
	.company-only-settings {
		margin-top: var(--space-4);
	}
	.provider-page {
		min-height: 0;
		overflow-wrap: anywhere;
	}
	.provider-page :is(button, select, input) {
		max-width: 100%;
	}
	.notice {
		padding: var(--space-3);
		background: var(--surface-alt);
		border-radius: var(--radius-control);
	}
	@container company-canvas (max-width: 640px) {
		.reuse-panel { padding: var(--space-4); }
		.connection-editor {
			padding: var(--space-4);
		}
		.reuse-section-head { align-items: flex-start; }
		.reuse-row { align-items: flex-start; }
		.reuse-identity { flex: 1 1 100%; }
		.model-picker { flex: 1 1 100%; align-items: flex-start; flex-direction: column; }
		.model-picker select { width: 100%; }
		.form-grid { grid-template-columns: 1fr; }
		.form-grid .wide { grid-column: auto; }
	}
</style>
