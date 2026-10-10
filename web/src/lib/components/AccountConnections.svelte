<script lang="ts">
	import { failureSentence } from '$lib/model/failure';
	import { hostPlace, planeStatus, type ApplianceStatus } from '$lib/model/appliance';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { onMount } from 'svelte';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import { getCompanies, type CompanyCatalogEntry } from '$lib/model/cockpit';
	import { accountGrantFix } from '$lib/model/company-start';
	import { page } from '$app/state';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import {
		announceIntelligenceChange,
		watchIntelligenceChanges
	} from '$lib/model/intelligence-events';
	let plane = $state<ApplianceStatus | null>(null);
	void planeStatus().then((value) => (plane = value));
	const catalog = modelCatalog();
	type CompanyUse = { id: string; name: string; in_use?: boolean };
	type AccountConnection = {
		id: string;
		label: string;
		provider: string;
		kind?: 'api_key' | 'oauth';
		account_identity?: string;
		status?: 'present' | 'absent' | 'invalid' | 'checking';
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
	let codexSaved = $derived(
		connections.some(
			(connection) => connection.kind === 'oauth' && connection.provider === 'openai-codex'
		)
	);
	let claudeSaved = $derived(
		connections.some(
			(connection) => connection.kind === 'oauth' && connection.provider === 'anthropic'
		)
	);
	let codexConnected = $derived(
		connections.some(
			(connection) =>
				connection.kind === 'oauth' &&
				connection.provider === 'openai-codex' &&
				connection.status === 'present'
		)
	);
	let claudeConnected = $derived(
		connections.some(
			(connection) =>
				connection.kind === 'oauth' &&
				connection.provider === 'anthropic' &&
				connection.status === 'present'
		)
	);
	let codexChecking = $derived(
		connections.some(
			(connection) =>
				connection.kind === 'oauth' &&
				connection.provider === 'openai-codex' &&
				connection.status === 'checking'
		)
	);
	let claudeChecking = $derived(
		connections.some(
			(connection) =>
				connection.kind === 'oauth' &&
				connection.provider === 'anthropic' &&
				connection.status === 'checking'
		)
	);
	let accountScope = $state<'account' | 'company'>('account');
	let manageUrl = $state('/account/connections');
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
	let importingCompany = $state('');
	let managingId = $state('');
	let companyRevisions = $state<Record<string, string>>({});
	let selectedCompany = $state('');
	let selectedModel = $state('');
	let selectedCustomModel = $state(false);
	let modelTouched = $state(false);
	let selectedMakeDefault = $state(false);
	let replaceRequired = $state(false);
	let busyCompany = $state(false);
	let confirmRevocation = $state('');
	let statusRefreshTimer: ReturnType<typeof setTimeout> | undefined;
	let statusRefreshAttempts = 0;
	let refreshSequence = 0;

	async function refresh() {
		const sequence = ++refreshSequence;
		loading = true;
		error = '';
		try {
			const response = await fetch('/api/connections', { cache: 'no-store' });
			const body = await response.json();
			if (sequence !== refreshSequence) return;
			if (!response.ok) throw new Error(body.message ?? 'Could not load account connections.');
			const previouslySignedIn = new Set(
				connections
					.filter((item) => item.kind === 'oauth' && item.status === 'present')
					.map((item) => item.provider)
			);
			connections = body.connections ?? [];
			if (
				connections.some(
					(item) =>
						item.kind === 'oauth' &&
						item.status === 'present' &&
						!previouslySignedIn.has(item.provider)
				)
			)
				void catalog.refreshConnected();
			accountScope = body.scope === 'company' ? 'company' : 'account';
			manageUrl = body.manage_url ?? '/account/connections';
			if (statusRefreshTimer) clearTimeout(statusRefreshTimer);
			if (connections.some((connection) => connection.status === 'checking')) {
				statusRefreshAttempts = Math.min(statusRefreshAttempts + 1, 12);
				const delay = Math.min(2500 * 2 ** Math.floor(statusRefreshAttempts / 3), 30_000);
				statusRefreshTimer = setTimeout(() => void refresh(), delay);
			} else {
				statusRefreshAttempts = 0;
				statusRefreshTimer = undefined;
			}
		} catch (cause) {
			if (sequence === refreshSequence)
				error = failureSentence(cause, 'Could not load account connections.');
		} finally {
			if (sequence === refreshSequence) loading = false;
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
	async function importCompanyCodex(companyId: string) {
		if (importingCompany) return;
		importingCompany = companyId;
		error = '';
		try {
			const response = await fetch('/api/connections/import/company-codex', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ company: companyId })
			});
			const body = await response.json();
			if (!response.ok)
				throw new Error(body.message ?? 'Could not use this sign-in for the account.');
			await refresh();
			announceIntelligenceChange();
		} catch (cause) {
			error = failureSentence(cause, 'Could not use this sign-in for the account.');
		} finally {
			importingCompany = '';
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
				body: JSON.stringify({ label: label.trim(), provider, secret })
			});
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not save this connection.');
			label = '';
			secret = '';
			addOpen = false;
			await refresh();
			announceIntelligenceChange();
		} catch (cause) {
			secret = '';
			error = failureSentence(cause, 'Could not save this connection.');
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
			oauthMessage = failureSentence(cause, 'Could not start sign-in.');
		}
	}
	async function completeClaudeSignIn() {
		if (!oauthJob || !oauthCallback.trim() || callbackBusy) return;
		callbackBusy = true;
		oauthMessage = '';
		try {
			const response = await fetch(
				`/api/connections/oauth/jobs/${encodeURIComponent(oauthJob)}/callback`,
				{
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ callback_url: oauthCallback.trim() })
				}
			);
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not finish Claude sign-in.');
			oauthCallback = '';
			oauthState = 'completing';
		} catch (cause) {
			oauthMessage = failureSentence(cause, 'Could not finish Claude sign-in.');
		} finally {
			callbackBusy = false;
		}
	}
	async function pollSignIn(job: string) {
		for (let attempt = 0; attempt < 300 && oauthJob === job; attempt++) {
			try {
				const response = await fetch(`/api/connections/oauth/jobs/${encodeURIComponent(job)}`, {
					cache: 'no-store'
				});
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
						announceIntelligenceChange();
					}
					return;
				}
			} catch (cause) {
				oauthMessage = failureSentence(cause, 'Could not check sign-in.');
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
		if (item.status === 'checking') return 'Checking sign-in';
		if (item.status === 'invalid') return `${credential} unavailable`;
		return `${credential} missing`;
	}
	function providerMark(id: string) {
		if (id === 'openai-codex') return 'GPT';
		if (id === 'anthropic') return 'Cl';
		return providerName(id).slice(0, 2);
	}
	function providerName(id: string) {
		if (id === 'openai-codex') return 'ChatGPT / Codex';
		return providerOptions.find(([key]) => key === id)?.[1] ?? id;
	}
	function modelChoices(providerId: string, kind: 'api_key' | 'oauth' = 'api_key') {
		return catalog.models(providerId, kind);
	}
	function routeModel(providerId: string, model: string) {
		return model.startsWith(`${providerId}/`) ? model : `${providerId}/${model}`;
	}
	function validModel(providerId: string, model: string) {
		return (
			model.startsWith(`${providerId}/`) &&
			model.length > providerId.length + 1 &&
			model.length <= 200 &&
			!/\s/.test(model)
		);
	}
	function defaultModel(providerId: string, kind: 'api_key' | 'oauth' = 'api_key') {
		const models = modelChoices(providerId, kind);
		const model = models.find((item) => 'default' in item && item.default)?.id ?? models[0]?.id;
		return model ? routeModel(providerId, model) : '';
	}
	function toggleManage(item: AccountConnection) {
		managingId = managingId === item.id ? '' : item.id;
		selectedCompany = '';
		selectedModel = defaultModel(item.provider, item.kind);
		selectedCustomModel = false;
		modelTouched = false;
		selectedMakeDefault = false;
		replaceRequired = false;
	}
	$effect(() => {
		if (!managingId || !selectedCompany || modelTouched) return;
		const item = connections.find((row) => row.id === managingId);
		if (item) selectedModel = defaultModel(item.provider, item.kind);
	});
	async function grant(item: AccountConnection, companyId: string, replace = false) {
		if (busyCompany || !validModel(item.provider, selectedModel)) return;
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
					throw new Error(state.message ?? 'Could not load this company’s connection settings.');
				revision = state.revision;
				companyRevisions = { ...companyRevisions, [companyId]: revision };
			}
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/connections/${encodeURIComponent(item.id)}`,
				{
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({
						model: selectedModel,
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
			if (!response.ok) throw new Error(body.message ?? 'Could not grant this company access.');
			companyRevisions = { ...companyRevisions, [companyId]: body.provider?.revision ?? revision };
			await refresh();
			announceIntelligenceChange(companyId);
			selectedCompany = '';
			replaceRequired = false;
		} catch (cause) {
			error = failureSentence(cause, 'Could not grant this company access.');
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
					throw new Error(state.message ?? 'Could not load this company’s connection settings.');
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
			if (!response.ok) throw new Error(body.message ?? 'Could not remove this company’s access.');
			companyRevisions = { ...companyRevisions, [company.id]: body.provider?.revision ?? revision };
			confirmRevocation = '';
			await refresh();
			announceIntelligenceChange(company.id);
		} catch (cause) {
			error = failureSentence(cause, 'Could not remove this company’s access.');
		} finally {
			busyCompany = false;
		}
	}

	function beginGrant(item: AccountConnection, companyId: string) {
		managingId = item.id;
		selectedCompany = companyId;
		selectedModel = defaultModel(item.provider, item.kind);
		selectedCustomModel = false;
		modelTouched = false;
		selectedMakeDefault = false;
		replaceRequired = false;
		/* A company with no default takes the first connection it is given as its default. */
		void fetch(`/api/companies/${encodeURIComponent(companyId)}/intelligence`, {
			cache: 'no-store'
		})
			.then((response) => (response.ok ? response.json() : null))
			.then((view: { default?: unknown } | null) => {
				if (view && managingId === item.id && selectedCompany === companyId)
					selectedMakeDefault = !view.default;
			})
			.catch(() => {});
		requestAnimationFrame(() =>
			document
				.querySelector(`[data-connection="${CSS.escape(item.id)}"]`)
				?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
		);
	}

	/* Companies that cannot start only because an account sign-in that already
	 * works has not been shared with them: one grant each fixes them. */
	const blocked = $derived(
		companies.flatMap((company) => {
			const fix = company.unstartable_reason
				? accountGrantFix(company.unstartable_reason, company.id, connections)
				: null;
			const connection = fix && connections.find((item) => item.id === fix.connection.id);
			return connection ? [{ company, connection }] : [];
		})
	);

	/* The Companies page links a blocked company here with the fix chosen;
	 * open that grant once both lists have loaded. */
	let deepLinkHandled = false;
	$effect(() => {
		if (deepLinkHandled || loading || !companies.length) return;
		const companyId = page.url.searchParams.get('grant');
		const connectionId = page.url.searchParams.get('connection');
		if (!companyId) return;
		deepLinkHandled = true;
		const item =
			connections.find((row) => row.id === connectionId) ??
			blocked.find((row) => row.company.id === companyId)?.connection;
		if (item && companies.some((company) => company.id === companyId)) beginGrant(item, companyId);
	});

	type ProviderRow = {
		key: string;
		title: string;
		oauth?: 'codex' | 'claude';
		connection?: AccountConnection;
	};
	const providerRows = $derived<ProviderRow[]>([
		{
			key: 'oauth:codex',
			title: 'ChatGPT / Codex',
			oauth: 'codex',
			connection: connections.find(
				(item) => item.kind === 'oauth' && item.provider === 'openai-codex'
			)
		},
		{
			key: 'oauth:claude',
			title: 'Claude',
			oauth: 'claude',
			connection: connections.find((item) => item.kind === 'oauth' && item.provider === 'anthropic')
		},
		...connections
			.filter(
				(item) => !(item.kind === 'oauth' && ['openai-codex', 'anthropic'].includes(item.provider))
			)
			.map((item) => ({ key: item.id, title: providerName(item.provider), connection: item }))
	]);
	const legacyProblems = $derived(
		nativeSignIns.filter((signIn) => signIn.state !== 'connected').length
	);

	onMount(() => {
		void refresh();
		const stopWatching = watchIntelligenceChanges(() => void refresh(), true);
		void getCompanies()
			.then((rows) => {
				companies = rows.filter((company) => company.lifecycle_status === 'active');
				void refreshNativeSignIns(companies);
			})
			.catch(() => {
				companyError = 'Companies could not be loaded. Reload to manage access.';
				nativeError = 'Company sign-ins could not be loaded.';
				nativeLoading = false;
			});
		return () => {
			if (statusRefreshTimer) clearTimeout(statusRefreshTimer);
			stopWatching();
		};
	});
</script>

<svelte:head><title>Account connections — {PRODUCT_NAME}</title></svelte:head>

{#snippet accessManager(item: AccountConnection)}
	<div class="access-manager" aria-label={`Company access for ${item.label}`}>
		{#if companyError}<span class="replace-warning" role="alert">{companyError}</span>{/if}
		{#each companies as company (company.id)}
			{@const granted = item.companies.find((use) => use.id === company.id)}
			<div class="access-row" class:selected={selectedCompany === company.id}>
				<strong>{company.name}</strong>
				{#if granted}<span class="access-state">Has access{granted.in_use ? ' · in use' : ''}</span
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
							}}>Remove</button
						>{/if}
				{:else if selectedCompany === company.id}
					<label class="model-picker"
						><span>Model</span><select
							value={selectedCustomModel ? '__custom' : selectedModel}
							onchange={(event) => {
								modelTouched = true;
								selectedCustomModel = event.currentTarget.value === '__custom';
								selectedModel = selectedCustomModel ? '' : event.currentTarget.value;
							}}
							aria-label={`Model for ${item.label} in ${company.name}`}
							><option value="">Choose a model…</option
							>{#each modelChoices(item.provider, item.kind) as model}<option
									value={routeModel(item.provider, model.id)}>{model.name ?? model.id}</option
								>{/each}<option value="__custom">Custom model ID…</option></select
						></label
					>
					{#if selectedCustomModel}<label class="model-picker"
							><span>Full model ID</span><input
								aria-label="Full model ID"
								placeholder={`${item.provider}/model-id`}
								bind:value={selectedModel}
							/></label
						>{/if}
					<label
						class="model-picker inline"
						title={catalog.source(item.provider, item.kind) === 'connected'
							? 'Models from this account’s connected runtime'
							: 'Model suggestions; availability depends on this connection'}
						><input type="checkbox" bind:checked={selectedMakeDefault} /> Company default</label
					>
					{#if replaceRequired}<span class="replace-warning" role="alert"
							>This replaces the current {providerName(item.provider)} connection for {company.name}.</span
						><button
							class="btn primary small"
							disabled={busyCompany || !validModel(item.provider, selectedModel)}
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
							disabled={busyCompany ||
								!validModel(item.provider, selectedModel) ||
								item.status !== 'present'}
							onclick={() => void grant(item, company.id)}
							>{busyCompany ? 'Giving access…' : 'Give access'}</button
						><button
							class="text-button"
							disabled={busyCompany}
							onclick={() => (selectedCompany = '')}>Cancel</button
						>{/if}
				{:else}<button
						class="btn small"
						disabled={busyCompany || item.status !== 'present'}
						onclick={() => beginGrant(item, company.id)}>Give access…</button
					>{/if}
			</div>
		{/each}
		{#if !companies.length}<span class="unused">No active companies available.</span>{/if}
	</div>
{/snippet}

{#snippet signInProgress(provider: 'codex' | 'claude')}
	{#if oauthProvider === provider && (oauthUrl || oauthMessage || ['connected', 'completing', 'waiting'].includes(oauthState))}
		<div class="sign-in-progress" aria-live="polite">
			{#if oauthUrl}<a class="btn small" href={oauthUrl} target="_blank" rel="noreferrer"
					>Open {provider === 'codex' ? 'Codex' : 'Claude'} sign-in ↗</a
				>{/if}
			{#if provider === 'codex' && oauthCode}<span
					>Enter code <strong class="code">{oauthCode}</strong></span
				>{/if}
			{#if provider === 'claude' && oauthUrl && oauthState === 'waiting'}<label
					class="callback-label"
					title={plane?.hosted
						? 'Sign-in finishes on a localhost address your browser cannot open from here. After you approve, copy that address from the browser bar and paste it here.'
						: 'If your browser cannot reach the callback on this computer, copy its final localhost URL and paste it here.'}
					><span>Callback URL</span><input
						type="url"
						bind:value={oauthCallback}
						placeholder="http://localhost:54545/callback?code=…"
						autocomplete="off"
					/></label
				><button
					class="btn primary small"
					disabled={!oauthCallback.trim() || callbackBusy}
					onclick={() => void completeClaudeSignIn()}
					>{callbackBusy ? 'Finishing…' : 'Finish sign-in'}</button
				>{/if}
			{#if oauthMessage}<span role="status">{oauthMessage}</span
				>{:else if oauthState === 'completing'}<span role="status">Finishing sign-in…</span
				>{:else if oauthState === 'connected'}<span role="status"
					>Connected. Give companies access below.</span
				>{/if}
		</div>
	{/if}
{/snippet}

<div class="account-connections">
	<div class="page-head">
		<span class="page-head-actions">
			{#if accountScope === 'account'}<button
					class="btn small"
					aria-expanded={addOpen}
					onclick={() => (addOpen = !addOpen)}>{addOpen ? 'Cancel' : 'Add API key'}</button
				>{:else if connections.length}<a class="btn small primary" href={manageUrl}
					>Open account ↗</a
				>{/if}
		</span>
	</div>

	{#if error}<div class="error" role="alert">
			{error}<button class="btn small" onclick={() => void refresh()}>Try again</button>
		</div>{/if}

	{#if blocked.length && accountScope === 'account'}
		<section class="needs-access" aria-label="Companies waiting for access">
			{#each blocked as row (row.company.id)}
				<div class="needs-access-row">
					<i aria-hidden="true"></i>
					<span
						><strong>{row.company.name}</strong> can’t start without {providerName(
							row.connection.provider
						)}</span
					>
					<button
						class="btn small primary"
						disabled={busyCompany}
						onclick={() => beginGrant(row.connection, row.company.id)}>Give access</button
					>
				</div>
			{/each}
		</section>
	{/if}

	{#if addOpen && accountScope === 'account'}
		<form class="add-form" onsubmit={create}>
			<div class="form-grid">
				<label
					>Provider<select bind:value={provider}
						>{#each providerOptions as [id, name]}<option value={id}>{name}</option>{/each}</select
					></label
				><label
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
					/></label
				>
			</div>
			<div class="actions">
				<button class="btn primary small" disabled={busy}
					>{busy ? 'Saving…' : 'Save API key'}</button
				><span class="unused">Saved to this account; no company uses it until you give access.</span
				>
			</div>
		</form>
	{/if}

	{#if loading && !connections.length}<Skeleton
			label="Loading connections"
			variant="list"
			count={2}
		/>
	{:else if accountScope === 'company' && !connections.length}
		<div class="empty">
			<h2>No account connection is shared here yet</h2>
			<a class="btn primary small" href={manageUrl}>Open account ↗</a>
		</div>
	{:else}
		<section class="provider-list" aria-label="Model connections">
			{#each providerRows.filter((row) => accountScope === 'account' || row.connection) as row (row.key)}
				{@const item = row.connection}
				{@const signedIn = item?.status === 'present'}
				{@const checking = item?.status === 'checking'}
				<article
					class="provider-row"
					data-connection={item?.id ?? row.key}
					class:open={!!item && managingId === item.id}
				>
					<div class="provider-main">
						<span class="provider-mark" class:live={signedIn} aria-hidden="true"
							>{providerMark(
								item?.provider ?? (row.oauth === 'codex' ? 'openai-codex' : 'anthropic')
							)}</span
						>
						<div class="provider-copy">
							<strong
								>{row.title}{#if item && item.label
										.trim()
										.toLowerCase() !== row.title.toLowerCase()}<span class="provider-label"
										>{item.label}</span
									>{/if}</strong
							>
							<small title={item?.detail ?? undefined}
								>{item?.account_identity ??
									(item
										? item.kind === 'oauth'
											? 'Account sign-in'
											: 'API key'
										: row.oauth === 'codex'
											? 'Sign in with ChatGPT'
											: 'Sign in with Claude')}</small
							>
						</div>
						<div class="provider-uses">
							{#if item?.companies.length}{#each item.companies as company (company.id)}<a
										class="company-chip"
										class:in-use={company.in_use}
										title={company.in_use ? 'In use by this company' : 'Available to this company'}
										href={`/${encodeURIComponent(company.id)}/company/provider`}>{company.name}</a
									>{/each}{:else if item}<span class="unused">No company access</span>{/if}
						</div>
						<span
							class="status"
							class:connected={signedIn}
							class:failed={item?.status === 'invalid' || item?.status === 'absent'}
							>{item ? statusText(item) : 'Not connected'}</span
						>
						<div class="provider-actions">
							{#if row.oauth && (!item || !signedIn) && accountScope === 'account'}<button
									class="btn small"
									class:primary={!!item}
									disabled={!!oauthJob || checking}
									onclick={() => void startSignIn(row.oauth!)}
									>{oauthJob && oauthProvider === row.oauth
										? 'Signing in…'
										: item
											? 'Reconnect'
											: 'Connect'}</button
								>{/if}
							{#if item && signedIn && accountScope === 'account'}<button
									class="btn small"
									aria-expanded={managingId === item.id}
									onclick={() => toggleManage(item)}
									>{managingId === item.id ? 'Done' : 'Manage access'}</button
								>{/if}
							{#if item && accountScope === 'account'}<ActionMenu label={`${row.title} options`}
									>{#if row.oauth && signedIn}<button
											disabled={!!oauthJob}
											onclick={() => void startSignIn(row.oauth!)}>Reconnect same account</button
										>{/if}<button onclick={() => void refresh()}>Refresh status</button></ActionMenu
								>{/if}
						</div>
					</div>
					{#if row.oauth}{@render signInProgress(row.oauth)}{/if}
					{#if item && managingId === item.id}{@render accessManager(item)}{/if}
				</article>
			{/each}
		</section>
	{/if}

	<!-- Legacy sign-ins are shown only when some exist: "None." is not news. -->
	{#if nativeLoading || nativeError || nativeSignIns.length}<details class="legacy">
			<summary
				><span class="legacy-chevron" aria-hidden="true">›</span>Company-only sign-ins
				<span class="legacy-count"
					>{nativeLoading ? 'Checking…' : nativeSignIns.length}{legacyProblems ===
						nativeSignIns.length && legacyProblems
						? ' · none signed in'
						: legacyProblems
							? ` · ${legacyProblems} not signed in`
							: ''}</span
				>
				<InfoTip
					text="Older sign-ins saved inside individual company computers. They do not affect the account connections above; give companies an account connection instead."
				/></summary
			>
			<div class="legacy-body">
				{#if nativeError}<p class="native-error" role="alert">{nativeError}</p>{/if}
				{#each nativeSignIns as signIn (`${signIn.companyId}:${signIn.harness}`)}
					<div class="native-row">
						<strong>{signIn.companyName}</strong>
						<span>{signIn.harness === 'codex' ? 'ChatGPT / Codex' : 'Claude Code'}</span>
						<span class="native-status" class:connected={signIn.state === 'connected'}
							>{nativeStatus(signIn.state)}</span
						>
						<div class="native-actions">
							{#if accountScope === 'account' && signIn.harness === 'codex' && signIn.state === 'connected' && !codexSaved}
								<button
									class="btn small"
									disabled={!!importingCompany}
									onclick={() => void importCompanyCodex(signIn.companyId)}
									>{importingCompany === signIn.companyId ? 'Adding…' : 'Add to account'}</button
								>
							{/if}
							<a href={`/${encodeURIComponent(signIn.companyId)}/company/provider`}>Open ↗</a>
						</div>
					</div>
				{/each}
				<button
					class="text-button"
					disabled={nativeLoading}
					onclick={() => void refreshNativeSignIns(companies)}>Check again</button
				>
			</div>
		</details>{/if}
</div>

<style>
	.account-connections {
		min-width: 0;
	}
	.page-head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: 14px;
	}
	.page-head:not(:has(.btn)) {
		display: none;
	}
	.page-head-actions {
		margin-left: auto;
		display: flex;
		gap: var(--space-2);
	}

	/* What blocks a company comes first, as one row and one action each. */
	.needs-access {
		display: grid;
		margin-bottom: 18px;
		border: 1px solid color-mix(in srgb, var(--intent-authority) 30%, var(--border));
		border-radius: var(--radius-pane);
		background: color-mix(in srgb, var(--intent-authority-soft) 60%, var(--surface-pane));
	}
	.needs-access-row {
		display: grid;
		grid-template-columns: 8px minmax(0, 1fr) auto;
		align-items: center;
		gap: 12px;
		min-height: 48px;
		padding: 8px 10px 8px 14px;
		font-size: var(--t-body);
	}
	.needs-access-row + .needs-access-row {
		border-top: 1px solid color-mix(in srgb, var(--intent-authority) 18%, var(--border));
	}
	.needs-access-row i {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--intent-authority);
	}
	.needs-access-row span {
		color: var(--text-secondary);
	}
	.needs-access-row strong {
		color: var(--ink);
		font-weight: 600;
	}

	/* One list, one row per provider: who it is, which companies use it, its
	 * state and one action. Detail opens inside the row it belongs to. */
	.provider-list {
		display: grid;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
		overflow: hidden;
	}
	.provider-row + .provider-row {
		border-top: 1px solid var(--border);
	}
	.provider-row.open {
		background: color-mix(in srgb, var(--surface-alt) 45%, var(--surface-pane));
	}
	.provider-main {
		display: grid;
		grid-template-columns: 32px minmax(150px, 1fr) minmax(0, 1.1fr) auto auto;
		align-items: center;
		gap: 14px;
		min-height: 62px;
		padding: 10px 12px 10px 14px;
	}
	.provider-mark {
		width: 32px;
		height: 32px;
		display: grid;
		place-items: center;
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font: 600 var(--t-label)/1 var(--font-mono);
		letter-spacing: 0.02em;
	}
	.provider-mark.live {
		border-color: color-mix(in srgb, var(--state-success) 35%, var(--border));
		color: var(--state-success);
	}
	.provider-copy {
		display: grid;
		gap: 1px;
		min-width: 0;
	}
	.provider-copy strong {
		display: flex;
		align-items: baseline;
		gap: 6px;
		font-size: var(--t-body);
		font-weight: 600;
	}
	.provider-label {
		color: var(--text-tertiary);
		font-weight: 400;
	}
	.provider-copy small {
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.provider-uses {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		min-width: 0;
	}
	.company-chip {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 3px 8px;
		border: 1px solid var(--border);
		border-radius: 99px;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.company-chip.in-use::before {
		content: '';
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--state-success);
	}
	.company-chip:hover {
		border-color: var(--intent-conversation);
		color: var(--ink);
	}
	.status {
		justify-self: end;
		padding: 3px 8px;
		border-radius: 99px;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font-size: var(--t-label);
		white-space: nowrap;
	}
	.status.connected {
		color: var(--state-success);
	}
	.status.failed {
		color: var(--state-danger);
	}
	.provider-actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 6px;
		min-width: 132px;
	}
	.sign-in-progress {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 10px 14px;
		margin: 0 14px 12px 60px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.code {
		font-family: var(--font-mono);
		letter-spacing: 0.08em;
		color: var(--ink);
	}
	.callback-label {
		display: grid;
		gap: 4px;
		flex: 1 1 320px;
		color: var(--text-tertiary);
	}
	.callback-label input,
	.model-picker select,
	.model-picker input:not([type='checkbox']),
	.form-grid input,
	.form-grid select {
		width: 100%;
		min-width: 0;
		min-height: 34px;
		padding: 5px 9px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--ink);
		font: inherit;
		box-sizing: border-box;
	}
	.access-manager {
		display: grid;
		margin: 0 12px 12px 60px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
	}
	.access-row {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px 10px;
		min-height: 44px;
		padding: 6px 8px 6px 12px;
	}
	.access-row + .access-row {
		border-top: 1px solid var(--border);
	}
	.access-row.selected {
		background: color-mix(in srgb, var(--intent-conversation-soft) 55%, transparent);
	}
	.access-row > strong {
		margin-right: auto;
		font-size: var(--t-body);
		font-weight: 500;
	}
	.access-state,
	.unused {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.model-picker {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.model-picker select {
		width: auto;
		max-width: 220px;
	}
	.replace-warning {
		flex-basis: 100%;
		color: var(--state-danger);
		font-size: var(--t-label);
		line-height: 1.45;
	}
	.text-button {
		padding: 4px 6px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-label);
		cursor: pointer;
	}
	.text-button:hover {
		background: var(--surface-alt);
		color: var(--ink);
	}
	.text-button.danger {
		color: var(--state-danger);
	}
	.add-form {
		margin-bottom: 18px;
		padding: 16px;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
		animation: bridge-popover-in var(--motion-disclosure) var(--ease-spring) both;
	}
	.form-grid {
		display: grid;
		grid-template-columns: 220px 1fr;
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
	.actions {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-3);
		margin-top: var(--space-3);
	}
	.error {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-3);
		margin-bottom: 18px;
		padding: var(--space-3);
		color: var(--state-danger);
		background: color-mix(in srgb, var(--state-danger) 7%, var(--surface-pane));
		border-radius: var(--radius-control);
	}
	.empty {
		display: grid;
		justify-items: start;
		gap: 12px;
		padding: 20px;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
	}
	.empty h2 {
		margin: 0;
		font-size: var(--t-head);
	}

	/* Older company-only sign-ins: history, folded away with a count. */
	.legacy {
		margin-top: 22px;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.legacy summary {
		display: flex;
		align-items: center;
		gap: 8px;
		width: fit-content;
		padding: 4px 6px 4px 2px;
		border-radius: var(--radius-control);
		cursor: pointer;
		list-style: none;
	}
	.legacy summary::-webkit-details-marker {
		display: none;
	}
	.legacy summary::before {
		content: none !important;
	}
	.legacy summary:hover {
		color: var(--ink);
	}
	.legacy-chevron {
		display: inline-block;
		transition: transform var(--motion-state) var(--ease-standard);
	}
	.legacy[open] .legacy-chevron {
		transform: rotate(90deg);
	}
	.legacy-count {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.legacy-body {
		display: grid;
		justify-items: start;
		margin-top: 8px;
		padding: 4px 0 0;
	}
	.native-error {
		margin: 0 0 8px;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
	.native-row {
		display: grid;
		grid-template-columns: minmax(140px, 1fr) minmax(120px, 1fr) 120px auto;
		align-items: center;
		gap: var(--space-3);
		width: 100%;
		min-height: 40px;
		border-bottom: 1px solid var(--border);
		font-size: var(--t-label);
	}
	.native-row strong {
		color: var(--ink);
		font-weight: 500;
	}
	.native-status.connected {
		color: var(--state-success);
	}
	.native-actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-3);
		white-space: nowrap;
	}
	.native-row a {
		color: var(--intent-conversation);
		text-decoration: none;
	}
	.native-row a:hover {
		text-decoration: underline;
	}
	.legacy-body .text-button {
		margin-top: 8px;
	}
	:is(.native-row a, .company-chip, .text-button):focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	@media (max-width: 860px) {
		.provider-main {
			grid-template-columns: 32px minmax(0, 1fr) auto;
		}
		.provider-uses {
			grid-column: 2 / -1;
			grid-row: 2;
		}
		.status {
			grid-column: 3;
			grid-row: 1;
		}
		.provider-actions {
			grid-column: 2 / -1;
			grid-row: 3;
			justify-content: flex-start;
			min-width: 0;
		}
		.sign-in-progress,
		.access-manager {
			margin-left: 14px;
		}
		.native-row {
			grid-template-columns: 1fr auto;
			padding: 6px 0;
		}
		.native-row > span:not(.native-status) {
			grid-column: 1;
			grid-row: 2;
		}
		.native-actions {
			grid-column: 2;
			grid-row: 2;
		}
	}
	@media (max-width: 620px) {
		.form-grid {
			grid-template-columns: 1fr;
		}
		.needs-access-row {
			grid-template-columns: 8px minmax(0, 1fr);
		}
		.needs-access-row button {
			grid-column: 2;
			justify-self: start;
		}
	}
</style>
