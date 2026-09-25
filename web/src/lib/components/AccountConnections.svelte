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
		account_identity?: string;
		status?: 'present' | 'absent' | 'invalid';
		detail?: string | null;
		companies: CompanyUse[];
	};
	type NativeSignIn = {
		companyId: string;
		companyName: string;
		harness: string;
		state: string;
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
	let codexSaved = $derived(connections.some((connection) => connection.kind === 'oauth' && connection.provider === 'openai-codex'));
	let claudeSaved = $derived(connections.some((connection) => connection.kind === 'oauth' && connection.provider === 'anthropic'));
	let codexConnected = $derived(connections.some((connection) => connection.kind === 'oauth' && connection.provider === 'openai-codex' && connection.status === 'present'));
	let claudeConnected = $derived(connections.some((connection) => connection.kind === 'oauth' && connection.provider === 'anthropic' && connection.status === 'present'));
	let accountScope = $state<'account' | 'company'>('account');
	let manageUrl = $state('/account/settings/connections');
	let loading = $state(true);
	let error = $state('');
	let addOpen = $state(false);
	let busy = $state(false);
	let label = $state('');
	let provider = $state('anthropic');
	let secret = $state('');
	let oauthJob = $state('');
	let oauthProvider = $state<'codex' | 'claude'>('codex');
	let oauthUrl = $state('');
	let oauthCode = $state('');
	let oauthCallback = $state('');
	let callbackBusy = $state(false);
	let oauthState = $state('');
	let oauthMessage = $state('');
	let companies = $state<CompanyCatalogEntry[]>([]);
	let companyError = $state('');
	let nativeSignIns = $state<NativeSignIn[]>([]);
	let nativeLoading = $state(true);
	let nativeError = $state('');
	let managingId = $state('');
	let companyRevisions = $state<Record<string, string>>({});
	let selectedCompany = $state('');
	let selectedModel = $state('');
	let selectedMakeDefault = $state(false);
	let replaceRequired = $state(false);
	let busyCompany = $state(false);
	let confirmRevocation = $state('');

	async function refresh() {
		loading = true;
		error = '';
		try {
			const response = await fetch('/api/connections', { cache: 'no-store' });
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not load account connections.');
			connections = body.connections ?? [];
			accountScope = body.scope === 'company' ? 'company' : 'account';
			manageUrl = body.manage_url ?? '/account/settings/connections';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not load account connections.';
		} finally {
			loading = false;
		}
	}
	async function refreshNativeSignIns(rows: CompanyCatalogEntry[]) {
		nativeLoading = true;
		const results = await Promise.allSettled(
			rows.map(async (company) => {
				const response = await fetch(
					`/api/companies/${encodeURIComponent(company.id)}/harness-auth`,
					{ cache: 'no-store' }
				);
				if (!response.ok) throw new Error(`Could not check ${company.name}.`);
				const body = await response.json();
				return (body.connections ?? [])
					.filter((connection: { mode: string }) => connection.mode === 'oauth')
					.map((connection: { harness: string; auth: { state: string } }) => ({
						companyId: company.id,
						companyName: company.name,
						harness: connection.harness,
						state: connection.auth.state
					}));
			})
		);
		nativeSignIns = results.flatMap((result) =>
			result.status === 'fulfilled' ? result.value : []
		);
		nativeError = results.some((result) => result.status === 'rejected')
			? 'Some company sign-ins could not be checked.'
			: '';
		nativeLoading = false;
	}
	function nativeStatus(state: string) {
		return (
			(
				{
					connected: 'Signed in',
					expired: 'Expired',
					unavailable: 'Unable to check',
					failed: 'Sign-in failed',
					waiting: 'Waiting for sign-in',
					starting: 'Starting sign-in'
				} as Record<string, string>
			)[state] ?? state
		);
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
				body: JSON.stringify({ label: label.trim(), provider, secret })
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
	async function startSignIn(provider: 'codex' | 'claude') {
		if (oauthJob) return;
		oauthProvider = provider;
		oauthMessage = '';
		oauthUrl = '';
		oauthCode = '';
		oauthCallback = '';
		oauthState = '';
		try {
			const response = await fetch(`/api/connections/oauth/${provider}`, { method: 'POST' });
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not start sign-in.');
			oauthJob = body.job;
			oauthState = 'starting';
			void pollSignIn(body.job);
		} catch (cause) {
			oauthMessage = cause instanceof Error ? cause.message : 'Could not start sign-in.';
		}
	}
	async function completeClaudeSignIn() {
		if (!oauthJob || !oauthCallback.trim() || callbackBusy) return;
		callbackBusy = true;
		oauthMessage = '';
		try {
			const response = await fetch(`/api/connections/oauth/jobs/${encodeURIComponent(oauthJob)}/callback`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ callback_url: oauthCallback.trim() })
			});
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not finish Claude sign-in.');
			oauthCallback = '';
			oauthState = 'completing';
		} catch (cause) {
			oauthMessage = cause instanceof Error ? cause.message : 'Could not finish Claude sign-in.';
		} finally {
			callbackBusy = false;
		}
	}
	async function pollSignIn(job: string) {
		for (let attempt = 0; attempt < 300 && oauthJob === job; attempt++) {
			try {
				const response = await fetch(`/api/connections/oauth/jobs/${encodeURIComponent(job)}`, { cache: 'no-store' });
				const body = await response.json();
				if (!response.ok) throw new Error(body.message ?? 'Could not check sign-in.');
				oauthState = body.state;
				oauthUrl = body.url ?? '';
				oauthCode = body.code ?? '';
				oauthMessage = body.message ?? '';
				if (body.state === 'connected' || body.state === 'failed') {
					oauthJob = '';
					if (body.state === 'connected') {
						await refresh();
					}
					return;
				}
			} catch (cause) {
				oauthMessage = cause instanceof Error ? cause.message : 'Could not check sign-in.';
				oauthJob = '';
				return;
			}
			await new Promise((resolve) => setTimeout(resolve, 1000));
		}
		oauthJob = '';
		oauthMessage = 'Sign-in timed out. Start again.';
	}
	function statusText(item: AccountConnection) {
		const credential = item.kind === 'oauth' ? 'Sign-in' : 'Key';
		if (item.status === 'present') return item.kind === 'oauth' ? 'Signed in' : 'Key stored';
		if (item.status === 'invalid') return `${credential} unavailable`;
		return `${credential} missing`;
	}
	function providerName(id: string) {
		if (id === 'openai-codex') return 'ChatGPT / Codex';
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
		selectedMakeDefault = false;
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
						make_default: selectedMakeDefault,
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
		if (busyCompany) return;
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
			confirmRevocation = '';
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
				void refreshNativeSignIns(companies);
			})
			.catch(() => {
				companyError = 'Projects could not be loaded. Reload to manage access.';
				nativeError = 'Company sign-ins could not be loaded.';
				nativeLoading = false;
			});
	});
</script>

<svelte:head><title>Account connections — {PRODUCT_NAME}</title></svelte:head>
<main class="account-connections">
	<header class="page-head">
		<div class="page-intro">
			<h1 title="Connect an account once, then grant individual companies access.">
				Connections
			</h1>
		</div>
		<!-- The empty state carries the one first action; the header takes over
		     once there is a list to add to. -->
		{#if accountScope === 'account'}{#if addOpen || connections.length}<button class="btn primary" onclick={() => (addOpen = !addOpen)}
			>{addOpen ? 'Close' : 'Add connection'}</button
		>{/if}{:else if connections.length}<a class="btn primary" href={manageUrl}>Open account ↗</a>{/if}
	</header>
	{#if accountScope === 'account'}<section class="native-section" aria-label="Account Codex sign-in">
		<div class="section-head"><h2>ChatGPT / Codex</h2><button class="btn primary" disabled={!!oauthJob} onclick={() => void startSignIn('codex')}>{oauthJob && oauthProvider === 'codex' ? 'Signing in…' : codexSaved ? 'Reconnect Codex' : 'Connect Codex'}</button></div>
		<p>{codexConnected ? 'Connected to this account. Grant company access below, or reconnect the same account if its sign-in stops working.' : codexSaved ? 'The saved Codex sign-in is unavailable. Reconnect the same account to restore company access.' : 'Sign in once with a device code, then grant this account connection to the companies that need it.'}</p>
		{#if oauthProvider === 'codex'}
			{#if oauthUrl}<p><a href={oauthUrl} target="_blank" rel="noreferrer">Open Codex sign-in ↗</a>{#if oauthCode} · Enter code <strong>{oauthCode}</strong>{/if}</p>{/if}
			{#if oauthMessage}<p role="status">{oauthMessage}</p>{:else if oauthState === 'connected'}<p role="status">Codex connected. Choose company access below.</p>{/if}
		{/if}
	</section>
	<section class="native-section" aria-label="Account Claude sign-in">
		<div class="section-head"><h2>Claude</h2><button class="btn primary" disabled={!!oauthJob} onclick={() => void startSignIn('claude')}>{oauthJob && oauthProvider === 'claude' ? 'Signing in…' : claudeSaved ? 'Reconnect Claude' : 'Connect Claude'}</button></div>
		<p>{claudeConnected ? 'Connected to this account. Grant Claude Agent access below, or reconnect the same account if its sign-in stops working.' : claudeSaved ? 'The saved Claude sign-in is unavailable. Reconnect the same account to restore company access.' : 'Sign in once with Claude, then grant its Claude Agent model route to individual companies.'}</p>
		{#if oauthProvider === 'claude'}
			{#if oauthUrl}<p><a href={oauthUrl} target="_blank" rel="noreferrer">Open Claude sign-in ↗</a></p>
				{#if oauthState === 'waiting'}<label class="callback-label">If your browser cannot reach the callback on this computer, copy its final localhost URL and paste it here.<input type="url" bind:value={oauthCallback} placeholder="http://localhost:54545/callback?code=…" autocomplete="off" /></label><button class="btn primary small" disabled={!oauthCallback.trim() || callbackBusy} onclick={() => void completeClaudeSignIn()}>{callbackBusy ? 'Finishing…' : 'Finish sign-in'}</button>{/if}
			{/if}
			{#if oauthMessage}<p role="status">{oauthMessage}</p>{:else if oauthState === 'completing'}<p role="status">Finishing Claude sign-in…</p>{:else if oauthState === 'connected'}<p role="status">Claude connected. Choose company access below.</p>{/if}
		{/if}
	</section>{/if}
	{#if error}<div class="error" role="alert">
			{error}<button class="btn small" onclick={() => void refresh()}>Try again</button>
		</div>{/if}
	{#if addOpen && accountScope === 'account'}
		<form class="add-form" onsubmit={create}>
			<h2>New provider connection</h2>
			<p>Save an API key at account level, then grant individual companies access.</p>
			<div class="form-grid">
				<label
					>Provider<select bind:value={provider}
						>{#each providerOptions as [id, name]}<option value={id}>{name}</option>{/each}</select
					></label
				><label class="full"
					>Name<input
						bind:value={label}
						required
						maxlength="80"
						placeholder="e.g. Anthropic team key"
					/></label
				><label class="full"
						>API key<input
							bind:value={secret}
							type="password"
							required
							autocomplete="new-password"
							placeholder="Paste API key"
					/></label>
			</div>
			<div class="actions">
				<button class="btn primary" disabled={busy}
					>{busy ? 'Saving…' : 'Save API key'}</button
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
							{#if item.account_identity}<small class="account-identity">{item.account_identity}</small>{/if}
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
							>{item.kind === 'oauth' ? 'Account sign-in' : 'API key'}</span
						>{#if accountScope === 'account'}<button class="text-button" onclick={() => toggleManage(item)}
							>{managingId === item.id ? 'Close access' : 'Manage project access'}</button
						>{/if}
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
										>{#if granted.in_use && confirmRevocation === `${item.id}:${company.id}`}
											<span class="replace-warning" role="alert"
												>AI work using this connection will stop until another is selected.</span
											>
											<button
												class="text-button danger"
												disabled={busyCompany}
												onclick={() => void revoke(item, granted)}>Remove access now</button
											>
											<button
												class="text-button"
												disabled={busyCompany}
												onclick={() => (confirmRevocation = '')}>Keep access</button
											>
										{:else}<button
												class="text-button danger"
												disabled={busyCompany}
												onclick={() => {
													if (granted.in_use) confirmRevocation = `${item.id}:${company.id}`;
													else void revoke(item, granted);
												}}>Remove access</button
											>{/if}
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
										<label class="model-picker"><input type="checkbox" bind:checked={selectedMakeDefault} /> Use as this company’s default model</label>
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
												selectedMakeDefault = false;
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
	{:else if accountScope === 'company'}
		<div class="empty"><h2>No account connection is granted here yet</h2><p>Open your account, then choose Account settings to connect a provider and grant access.</p><a class="btn primary" href={manageUrl}>Open account ↗</a></div>
	{:else if !addOpen}
		<div class="empty">
			<h2>No reusable connections yet</h2>
			<p>
				Save an API key or connect Codex or Claude above, then choose
				which companies can use it.
			</p>
			<button class="btn primary" onclick={() => (addOpen = true)}>Add your first connection</button
			>
		</div>
	{/if}
	<section class="native-section" aria-label="Native sign-ins by company">
		<div class="section-head">
			<h2>Company sign-ins</h2>
			<button class="text-button" disabled={nativeLoading} onclick={() => void refreshNativeSignIns(companies)}>Refresh status</button>
		</div>
		<p>These older sign-ins belong to the company shown here. Connect a provider above to share one account sign-in with other companies.</p>
		{#if nativeError}<p class="native-error" role="alert">{nativeError}</p>{/if}
		{#if nativeLoading}<p role="status">Checking company sign-ins…</p>
		{:else if nativeSignIns.length}
			<div class="native-list">
				{#each nativeSignIns as signIn (`${signIn.companyId}:${signIn.harness}`)}
					<div class="native-row">
						<strong>{signIn.harness === 'codex' ? 'ChatGPT / Codex' : 'Claude Code'}</strong>
						<span>{signIn.companyName}</span>
						<span class="native-status" class:connected={signIn.state === 'connected'}>{nativeStatus(signIn.state)}</span>
						<a href={`/${encodeURIComponent(signIn.companyId)}/company/provider`}>Manage in company ↗</a>
					</div>
				{/each}
			</div>
		{:else}<p>No company OAuth sign-ins are configured.</p>{/if}
	</section>
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
	.native-section {
		margin-top: 28px;
		padding: 20px;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
	}
	.section-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
	}
	.section-head h2 {
		margin: 0;
		font-size: var(--t-head);
	}
	.native-section p {
		margin: 8px 0 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		line-height: 1.5;
	}
	.native-section .native-error {
		color: var(--state-danger);
	}
	.callback-label {
		display: grid;
		gap: var(--space-2);
		max-width: 720px;
		margin: 16px 0 12px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.callback-label input {
		width: 100%;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--text-primary);
		font: inherit;
	}
	.native-list {
		margin-top: 16px;
		border-top: 1px solid var(--border);
	}
	.native-row {
		display: grid;
		grid-template-columns: minmax(150px, 1.2fr) minmax(140px, 1fr) 120px auto;
		align-items: center;
		gap: var(--space-3);
		min-height: 48px;
		padding: 8px 0;
		border-bottom: 1px solid var(--border);
		font-size: var(--t-label);
	}
	.native-row strong {
		font-weight: 600;
	}
	.native-row > span:not(.native-status) {
		color: var(--text-secondary);
	}
	.native-status {
		color: var(--text-secondary);
	}
	.native-status.connected {
		color: var(--state-success);
	}
	.native-row a {
		justify-self: end;
		color: var(--intent-conversation);
		text-decoration: none;
	}
	.native-row a:hover {
		text-decoration: underline;
	}
	.native-row a:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 3px;
	}
	.add-form p,
	.connection-head span,
	.connection-detail,
	.unused {
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
	.account-identity {
		color: var(--text-secondary);
		font-size: var(--t-label);
		overflow-wrap: anywhere;
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
		.native-row {
			grid-template-columns: 1fr auto;
		}
		.native-row > span:not(.native-status) {
			grid-column: 1;
			grid-row: 2;
		}
		.native-status {
			grid-column: 2;
			grid-row: 1;
		}
		.native-row a {
			grid-column: 2;
			grid-row: 2;
		}
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
