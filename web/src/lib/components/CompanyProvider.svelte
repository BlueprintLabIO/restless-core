<script lang="ts">
	import { Page, Section, Row, Item, Notice, Empty, Dot, Fold } from '$lib/ui/page';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { failureSentence } from '$lib/model/failure';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { onMount, tick } from 'svelte';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	const catalog = modelCatalog();
	import HarnessConnections from './HarnessConnections.svelte';
	import HarnessDiagnostics from './HarnessDiagnostics.svelte';
	import CustomHarnesses from './CustomHarnesses.svelte';
	import AgentIntelligence from './AgentIntelligence.svelte';
	import CopyCompanySetting from './CopyCompanySetting.svelte';
	import { companiesQuery } from '$lib/model/queries.svelte';
	import { startLinkLabel } from '$lib/model/company-start';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import {
		announceIntelligenceChange,
		watchIntelligenceChanges,
		affectsCompany
	} from '$lib/model/intelligence-events';
	import { getCompanies, type CompanyCatalogEntry } from '$lib/model/cockpit';
	let { companyId }: { companyId: string } = $props();
	const catalogProjection = companiesQuery();
	const setupIssue = $derived(
		catalogProjection.view.find((company) => company.id === companyId)?.unstartable_reason
	);
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
	let advancedModel = $state('');
	let advancedCustomModel = $state(false);
	let advancedModelTouched = $state(false);
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
		status?: 'present' | 'absent' | 'invalid' | 'checking';
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
	let connectionCustomModel = $state(false);
	let connectionModelTouched = $state(false);
	let connectionMakeDefault = $state(false);
	let connectionSecret = $state('');
	let grantSelection = $state('');
	let replaceGrantId = $state('');
	let grantMakeDefault = $state(false);
	let grantModels = $state<Record<string, string>>({});
	let grantCustomModels = $state<Record<string, boolean>>({});
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
	/* Company-only keys stay folded until the company has one or one is being
	 * added; custom harnesses open when a link asks for them. */
	let keysOpen = $state(false);
	let harnessOpen = $state(false);
	$effect(() => {
		if (savedCount || editorOpen) keysOpen = true;
	});
	onMount(() => {
		if (location.hash === '#harnesses') harnessOpen = true;
	});
	function defaultReference(provider: string) {
		return `infisical:/companies/${companyId}/MODEL_${provider.replace(/[^a-zA-Z0-9_]/g, '_')}_API_KEY`;
	}
	function choose(provider: string, reveal = false) {
		if (provider !== selected) {
			advancedModel = defaultModel(provider, mode === 'oauth' ? 'oauth' : 'api_key');
			advancedCustomModel = false;
			advancedModelTouched = false;
		}
		selected = provider;
		edited = false;
		secret = '';
		error = '';
		notice = '';
		const row = status?.connections.find((c) => c.provider === provider);
		mode = row?.reference?.startsWith('omp-oauth:')
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
				error = failureSentence(cause, 'Could not read connections.');
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
		if (!disconnect && !validModel(selected, advancedModel)) {
			error = 'Choose a model from the list or enter its full provider/model ID.';
			return;
		}
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
					model: advancedModel.slice(selected.length + 1),
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
			announceIntelligenceChange(companyId);
			editorOpen = false;
			choose(selected);
			notice = disconnect
				? 'Disconnected. Restart Restless to unload this route.'
				: 'Connection saved. Restart Restless to activate this provider.';
		} catch (cause) {
			secret = '';
			error = failureSentence(cause, 'Could not save this connection.');
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
			manageUrl =
				accountScope === 'company'
					? (body.manage_url ?? '/account/settings/connections')
					: '/account/settings/connections';
			companies = companyRows.filter((company) => company.lifecycle_status === 'active');
		} catch (cause) {
			accountError = failureSentence(cause, 'Could not load reusable connections.');
		} finally {
			accountLoading = false;
		}
	}
	function modelChoices(provider: string, kind: 'api_key' | 'oauth' = 'api_key') {
		return catalog.models(provider, kind);
	}
	function routeModel(provider: string, model: string) {
		return model.startsWith(`${provider}/`) ? model : `${provider}/${model}`;
	}
	function validModel(provider: string, model: string) {
		return (
			model.startsWith(`${provider}/`) &&
			model.length > provider.length + 1 &&
			model.length <= 200 &&
			!/\s/.test(model)
		);
	}
	function keyStatus(connection: ReusableConnection) {
		const noun = connection.kind === 'oauth' ? 'Sign-in' : 'Key';
		if (connection.status === 'present')
			return connection.kind === 'oauth' ? 'Signed in' : 'Key stored';
		if (connection.status === 'checking') return `Checking ${noun.toLowerCase()}`;
		if (connection.status === 'invalid') return `${noun} unavailable`;
		return `${noun} missing`;
	}
	function defaultModel(provider: string, kind: 'api_key' | 'oauth' = 'api_key') {
		const models = modelChoices(provider, kind);
		const model = models.find((item) => 'default' in item && item.default)?.id ?? models[0]?.id;
		return model ? routeModel(provider, model) : '';
	}
	function modelForGrant(connection: ReusableConnection) {
		return (
			grantModels[connection.id] ?? defaultModel(connection.provider, connection.kind ?? 'api_key')
		);
	}
	function beginGrant(connection: ReusableConnection) {
		delete grantModels[connection.id];
		grantCustomModels[connection.id] = false;
		grantSelection = connection.id;
		grantMakeDefault = false;
		replaceGrantId = '';
		accountError = '';
	}
	function toggleAddConnection() {
		addConnectionOpen = !addConnectionOpen;
		if (addConnectionOpen) {
			connectionModel = defaultModel(connectionProvider);
			connectionCustomModel = false;
			connectionModelTouched = false;
			connectionMakeDefault = false;
			accountError = '';
		}
	}
	$effect(() => {
		if (addConnectionOpen && !connectionModelTouched)
			connectionModel = defaultModel(connectionProvider);
	});
	$effect(() => {
		if (editorOpen && selected && !advancedModelTouched)
			advancedModel = defaultModel(selected, mode === 'oauth' ? 'oauth' : 'api_key');
	});
	async function requestGrant(
		id: string,
		model: string,
		replaceExisting = false,
		makeDefault = false
	) {
		if (!status) throw new Error('Company settings are still loading. Try again in a moment.');
		const connection = reusableConnections.find((row) => row.id === id);
		if (!connection || !validModel(connection.provider, model))
			throw new Error('Choose a model from the list or enter its full provider/model ID.');
		const response = await fetch(
			`/api/companies/${encodeURIComponent(companyId)}/connections/${encodeURIComponent(id)}`,
			{
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({
					model,
					make_default: makeDefault,
					revision: status.revision,
					...(replaceExisting ? { replace_existing: true } : {})
				})
			}
		);
		const body = await response.json();
		if (
			response.status === 409 &&
			(body.code ?? body.error) === 'provider_conflict' &&
			!replaceExisting
		)
			return { replaceRequired: true as const };
		if (!response.ok)
			throw new Error(body.message ?? 'Could not use this connection in the company.');
		return { replaceRequired: false as const, provider: body.provider };
	}
	async function createReusableConnection(event: SubmitEvent) {
		event.preventDefault();
		if (accountBusy) return;
		if (!validModel(connectionProvider, connectionModel)) {
			accountError = 'Choose a model from the list or enter its full provider/model ID.';
			return;
		}
		accountBusy = true;
		accountError = '';
		try {
			const response = await fetch('/api/connections', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({
					label: connectionLabel.trim(),
					provider: connectionProvider,
					secret: connectionSecret
				})
			});
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not save this connection.');
			const created = body.connection ?? body;
			await refreshReusableConnections();
			if (!status || !created.id)
				throw new Error(
					'Connection saved, but could not enable it in this company yet. Choose it from Account API keys to finish.'
				);
			const grantResult = await requestGrant(
				created.id,
				connectionModel,
				false,
				connectionMakeDefault
			);
			connectionSecret = '';
			connectionLabel = '';
			connectionModel = '';
			addConnectionOpen = false;
			await Promise.all([refreshReusableConnections(), intelligence.refresh()]);
			announceIntelligenceChange(companyId);
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
			accountError = failureSentence(cause, 'Could not save this connection.');
		} finally {
			accountBusy = false;
		}
	}
	async function grantConnection(connection: ReusableConnection) {
		if (!status || accountBusy) return;
		accountBusy = true;
		accountError = '';
		try {
			const result = await requestGrant(
				connection.id,
				modelForGrant(connection),
				replaceGrantId === connection.id,
				grantMakeDefault
			);
			if (result.replaceRequired) {
				replaceGrantId = connection.id;
				accountError = '';
				return;
			}
			status = result.provider;
			grantSelection = '';
			replaceGrantId = '';
			await Promise.all([refreshReusableConnections(), intelligence.refresh()]);
			announceIntelligenceChange(companyId);
		} catch (cause) {
			accountError = failureSentence(cause, 'Could not use this connection in the company.');
		} finally {
			accountBusy = false;
		}
	}
	async function revokeConnection(connection: ReusableConnection) {
		if (!status || accountBusy) return;
		accountBusy = true;
		accountError = '';
		try {
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/connections/${encodeURIComponent(connection.id)}`,
				{
					method: 'DELETE',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ revision: status.revision })
				}
			);
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not remove this company’s access.');
			status = body.provider;
			await Promise.all([refreshReusableConnections(), intelligence.refresh()]);
			announceIntelligenceChange(companyId);
		} catch (cause) {
			accountError = failureSentence(cause, 'Could not remove this company’s access.');
		} finally {
			accountBusy = false;
		}
	}
	async function loadImportProviders() {
		importProviders = [];
		importProvider = '';
		if (!importSource || importSource === companyId) return;
		try {
			const response = await fetch(`/api/companies/${encodeURIComponent(importSource)}/provider`, {
				cache: 'no-store'
			});
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not inspect that company.');
			importProviders = (body.connections ?? []).filter(
				(row: Connection) => row.shareable_api_key && row.credential_status === 'present'
			);
			importProvider = importProviders[0]?.provider ?? '';
			accountError = importProviders.length
				? ''
				: 'No available API-key connections were found in that company.';
		} catch (cause) {
			accountError = failureSentence(cause, 'Could not inspect that company.');
		}
	}
	async function importReusableConnection() {
		if (accountBusy || !importSource || !importProvider) return;
		accountBusy = true;
		accountError = '';
		try {
			const response = await fetch('/api/connections/import', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({
					source_company: importSource,
					provider: importProvider,
					label: importLabel.trim() || `${labels[importProvider] ?? importProvider} API`
				})
			});
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not make this connection reusable.');
			const imported = body.connection ?? body;
			await refreshReusableConnections();
			if (!status || !imported.id)
				throw new Error(
					'Connection saved to your account, but it could not be granted to this company. Choose it from Account API keys to finish.'
				);
			const grantResult = await requestGrant(imported.id, defaultModel(importProvider));
			importOpen = false;
			importSource = '';
			importProviders = [];
			await Promise.all([refreshReusableConnections(), intelligence.refresh()]);
			announceIntelligenceChange(companyId);
			if (grantResult.replaceRequired) {
				grantSelection = imported.id;
				replaceGrantId = imported.id;
				accountError = '';
			} else {
				status = grantResult.provider;
				notice = 'Connection imported and made available to this company.';
			}
		} catch (cause) {
			accountError = failureSentence(cause, 'Could not make this connection reusable.');
		} finally {
			accountBusy = false;
		}
	}
	onMount(() => {
		void refresh();
		void refreshReusableConnections();
		const stopWatching = watchIntelligenceChanges((changed) => {
			if (!affectsCompany(changed, companyId)) return;
			void refresh();
			void refreshReusableConnections();
		}, true);
		return () => {
			stopWatching();
			++requestSequence;
			secret = '';
		};
	});
</script>

<Page
	title="Intelligence"
	info="Which models the company's agents use, and the sign-ins and keys they reach them through. Model choices stay company-specific."
>
	{#snippet actions()}
		<CopyCompanySetting
			{companyId}
			setting="models"
			label="Model choices"
			oncopied={async () => {
				await Promise.all([refresh(), intelligence.refresh()]);
			}}
		/>
		<a class="btn small" href={manageUrl}
			>{accountScope === 'company' ? 'Open account' : 'Account connections'}
			<span aria-hidden="true">↗</span></a
		>
	{/snippet}

	{#if setupIssue}
		<Notice tone="warning" title={startLinkLabel(setupIssue)} details={setupIssue}>
			{#snippet actions()}<a class="btn small primary" href={manageUrl}>Fix</a>{/snippet}
		</Notice>
	{/if}
	{#if error}<Notice tone="danger" title="That change was not saved" details={error} />{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}

	<AgentIntelligence {companyId} />

	<Section
		title="Connections"
		info="Sign-ins and API keys saved in your account. A company can only use one after you give it access here."
		count={reusableConnections.length || null}
	>
		{#snippet actions()}
			{#if accountScope === 'account'}<button
					class="btn small"
					disabled={accountBusy}
					aria-expanded={addConnectionOpen}
					onclick={toggleAddConnection}>{addConnectionOpen ? 'Cancel' : 'Add API key'}</button
				>{/if}
		{/snippet}
		{#if accountLoading}
			<Empty compact title="Loading connections…" />
		{:else}
			{#each reusableConnections as item (item.id)}
				{@const companyGrant = item.companies.find((company) => company.id === companyId)}
				{@const broken = item.status !== 'present' || (companyGrant?.in_use && !!setupIssue)}
				{@const name = labels[item.provider] ?? item.provider}
				<Item
					title={item.label}
					meta={[
						item.kind === 'oauth' ||
						item.label.toLowerCase().includes(name.toLowerCase()) ||
						name.toLowerCase().includes(item.label.toLowerCase())
							? null
							: name,
						item.kind === 'oauth' ? 'Account sign-in' : 'API key',
						item.companies.length && !companyGrant ? `used by ${item.companies.length} other` : null
					]
						.filter(Boolean)
						.join(' · ')}
					selected={grantSelection === item.id}
				>
					{#snippet leading()}<Dot
							tone={broken
								? 'danger'
								: companyGrant?.in_use
									? 'success'
									: companyGrant
										? 'progress'
										: 'muted'}
							label={broken
								? item.status === 'checking'
									? 'Checking'
									: 'Unavailable'
								: companyGrant?.in_use
									? 'In use'
									: companyGrant
										? 'Available'
										: 'Not given access'}
						/>{/snippet}
					{#snippet trailing()}
						{#if broken}
							<span class="bad" title={item.detail ?? 'Check this connection in your account.'}
								>{item.status === 'checking'
									? 'Checking'
									: item.kind === 'oauth'
										? 'Sign-in unavailable'
										: 'Key unavailable'}</span
							>
							<a class="btn small primary" href={manageUrl}
								>{item.kind === 'oauth' ? 'Reconnect' : 'Replace key'}</a
							>
						{:else if companyGrant}
							<span
								title={companyGrant.in_use
									? 'Choose another model for this provider before removing access.'
									: undefined}>{companyGrant.in_use ? 'In use' : 'Has access'}</span
							>
						{:else if grantSelection !== item.id}
							<button
								class="btn small primary"
								disabled={accountBusy || !status || item.status !== 'present'}
								onclick={() => beginGrant(item)}>Use here</button
							>
						{/if}
					{/snippet}
					{#snippet actions()}
						{#if companyGrant && !companyGrant.in_use && accountScope === 'account'}<ActionMenu
								label={`${item.label} options`}
								><button disabled={accountBusy || !status} onclick={() => revokeConnection(item)}
									>Remove access</button
								></ActionMenu
							>{/if}
					{/snippet}
					{#if grantSelection === item.id}
						<div class="grant">
							<label class="field-label"
								><span>Model</span><select
									aria-label={`Model for ${item.label}`}
									value={grantCustomModels[item.id] ? '__custom' : modelForGrant(item)}
									title={catalog.source(item.provider, item.kind ?? 'api_key') === 'connected'
										? 'Models from this account’s connected runtime'
										: 'Model suggestions; availability depends on this connection'}
									onchange={(event) => {
										grantCustomModels[item.id] = event.currentTarget.value === '__custom';
										grantModels[item.id] = grantCustomModels[item.id]
											? ''
											: event.currentTarget.value;
									}}
								>
									{#each modelChoices(item.provider, item.kind ?? 'api_key') as model}<option
											value={routeModel(item.provider, model.id)}>{model.name ?? model.id}</option
										>{/each}
									<option value="__custom">Custom model ID…</option>
								</select></label
							>
							{#if grantCustomModels[item.id]}<label class="field-label"
									><span>Model ID</span><input
										aria-label={`Full model ID for ${item.label}`}
										placeholder={`${item.provider}/model-id`}
										bind:value={grantModels[item.id]}
									/></label
								>{/if}
							{#if status?.primary_provider !== 'unconfigured'}<label class="check"
									><input type="checkbox" bind:checked={grantMakeDefault} />Company default</label
								>{/if}
							{#if replaceGrantId === item.id}<Notice
									tone="warning"
									title={`This replaces the company's current ${name} connection`}
								/>{/if}
							<div class="bar">
								<button
									class="btn primary small"
									disabled={accountBusy ||
										!status ||
										!validModel(item.provider, modelForGrant(item)) ||
										item.status !== 'present'}
									onclick={() => grantConnection(item)}
									>{accountBusy
										? 'Saving…'
										: replaceGrantId === item.id
											? 'Replace'
											: 'Use in this company'}</button
								>
								<button
									class="btn small ghost"
									disabled={accountBusy}
									onclick={() => {
										grantSelection = '';
										replaceGrantId = '';
									}}>Cancel</button
								>
							</div>
						</div>
					{/if}
				</Item>
			{:else}
				<Empty
					compact
					title="No connection is available to this company yet"
					info={accountScope === 'company'
						? 'Open your account, then choose Account settings to grant one.'
						: 'Sign in or save an API key in your account, then give this company access.'}
				/>
			{/each}

			{#if addConnectionOpen && accountScope === 'account'}
				<form class="add" onsubmit={createReusableConnection}>
					<Row label="Provider">
						<select
							bind:value={connectionProvider}
							aria-label="Provider"
							onchange={() => {
								connectionModel = defaultModel(connectionProvider);
								connectionCustomModel = false;
								connectionModelTouched = false;
							}}
							>{#each Object.entries(labels).filter(([id]) => id !== 'openai-codex') as [id, label]}<option
									value={id}>{label}</option
								>{/each}</select
						>
					</Row>
					<Row label="Name">
						<input
							class="field"
							bind:value={connectionLabel}
							aria-label="Name"
							placeholder="Anthropic team key"
							required
							maxlength="80"
						/>
					</Row>
					<Row
						label="API key"
						info="One API key per provider is shared across companies. Each company still needs its own access. This saves the key; it does not test a sign-in."
					>
						<input
							class="field"
							type="password"
							bind:value={connectionSecret}
							aria-label="API key"
							autocomplete="new-password"
							placeholder="Paste API key"
							required
						/>
					</Row>
					<Row label="Model">
						<select
							value={connectionCustomModel ? '__custom' : connectionModel}
							aria-label="Model"
							onchange={(event) => {
								connectionModelTouched = true;
								connectionCustomModel = event.currentTarget.value === '__custom';
								connectionModel = connectionCustomModel ? '' : event.currentTarget.value;
							}}
							><option value="">Choose a model…</option
							>{#each modelChoices(connectionProvider) as model}<option
									value={routeModel(connectionProvider, model.id)}>{model.name ?? model.id}</option
								>{/each}<option value="__custom">Custom model ID…</option></select
						>
						{#if connectionCustomModel}<input
								class="field"
								aria-label="Full model ID"
								placeholder={`${connectionProvider}/model-id`}
								bind:value={connectionModel}
							/>{/if}
					</Row>
					{#if status?.primary_provider !== 'unconfigured'}<Row label="Company default"
							><input
								type="checkbox"
								aria-label="Make this the company default"
								bind:checked={connectionMakeDefault}
							/></Row
						>{/if}
					<div class="bar pad">
						<button class="btn primary small" disabled={accountBusy}
							>{accountBusy ? 'Saving…' : 'Save key'}</button
						>
						<button
							class="btn small ghost"
							type="button"
							disabled={accountBusy}
							onclick={() => {
								addConnectionOpen = false;
								connectionSecret = '';
							}}>Cancel</button
						>
					</div>
				</form>
			{/if}

			{#if accountScope === 'account' && otherCompanies.length}
				<Fold label="Bring in a key from another company" bind:open={importOpen}>
					<div class="import">
						<p class="quiet">
							This makes an account copy of the key. The other company keeps its own. One API key
							per provider can be shared across companies.
						</p>
						<div class="bar">
							<select
								bind:value={importSource}
								aria-label="Source company"
								onchange={() => void loadImportProviders()}
								><option value="">Choose a company…</option
								>{#each companies.filter((company) => company.id !== companyId) as company}<option
										value={company.id}>{company.name}</option
									>{/each}</select
							>
							{#if importProviders.length}<select
									bind:value={importProvider}
									aria-label="API connection"
									onchange={() => (importLabel = '')}
									>{#each importProviders as item}<option value={item.provider}
											>{labels[item.provider] ?? item.provider}</option
										>{/each}</select
								><input
									class="field"
									bind:value={importLabel}
									aria-label="Name"
									placeholder="Optional name"
									maxlength="80"
								/>{/if}
							<button
								class="btn primary small"
								disabled={accountBusy || !importProvider}
								onclick={() => void importReusableConnection()}
								>{accountBusy ? 'Importing…' : 'Bring in'}</button
							>
						</div>
					</div>
				</Fold>
			{/if}
		{/if}
	</Section>
	{#if accountError}<Notice
			tone="danger"
			title="That connection change failed"
			details={accountError}
		/>{/if}

	<Section
		title="Advanced"
		info="Sign-ins and keys that belong to this company only, the model catalog, diagnostics and custom harnesses."
	>
		<Fold label="Company-only sign-ins">
			<div class="pad"><HarnessConnections {companyId} /></div>
		</Fold>
		<Fold label="Company-only API keys" count={savedCount || null} bind:open={keysOpen}>
			<div class="pad keys">
				<div class="bar">
					<Dot
						show
						tone={status?.infisical_status === 'present' ? 'success' : status ? 'danger' : 'muted'}
						label={status?.infisical_status === 'present'
							? 'Secure key storage ready'
							: status
								? 'Key storage unavailable'
								: 'Checking key storage…'}
					/>
					<span class="spacer"></span>
					<a class="btn small ghost" href={`/${companyId}/company/vault`}>Vault</a>
					<button class="btn small ghost" disabled={busy || refreshing} onclick={() => refresh()}
						>{refreshing ? 'Checking…' : 'Refresh'}</button
					>
					<button
						class="btn small"
						disabled={!status || busy}
						onclick={() => {
							choose('');
							editorOpen = true;
						}}>Add key</button
					>
				</div>
				{#if status}
					{#each status.connections.filter((c) => c.reference) as row (row.provider)}
						<div class="key-row">
							<strong>{labels[row.provider] ?? row.provider}</strong>
							<span class="badge {tone(row)}">{stateLabel(row)}</span>
							<button
								class="btn small"
								disabled={busy}
								onclick={() => choose(row.provider, true)}
								aria-label={`Edit ${labels[row.provider] ?? row.provider}`}>Edit</button
							>
						</div>
					{:else}<p class="quiet">None yet.</p>{/each}
				{/if}
				{#if editorOpen && status}
					<form
						class="editor"
						bind:this={editor}
						aria-label="Key settings"
						onsubmit={(e) => connect(e)}
					>
						<label class="field-label"
							><span>Provider</span><select
								required
								disabled={busy}
								value={selected ? (labels[selected] ? selected : 'custom') : ''}
								onchange={(e) => choose(e.currentTarget.value)}
							>
								<option value="" disabled>Choose a provider…</option>
								{#each Object.entries(labels).filter(([id]) => id !== 'openai-codex' || status?.connections.some((c) => c.provider === id && c.reference)) as [id, label]}<option
										value={id}>{label}</option
									>{/each}<option value="custom">Custom provider…</option>
							</select></label
						>
						{#if selected && !labels[selected]}<label class="field-label"
								><span>Provider ID</span><input
									placeholder="Provider ID"
									value={selected === 'custom' ? '' : selected}
									oninput={(e) => choose(e.currentTarget.value || 'custom')}
									pattern="[a-zA-Z0-9_.\-]+"
									required
									disabled={busy}
								/></label
							>{/if}
						{#if mode === 'infisical'}<label class="field-label"
								><span>API key</span><input
									type="password"
									bind:value={secret}
									oninput={() => (edited = true)}
									autocomplete="new-password"
									placeholder={connection?.reference
										? 'Paste a replacement, or leave blank to keep it'
										: 'Paste your API key'}
									disabled={busy || status.infisical_status !== 'present'}
									required={!connection?.reference}
								/></label
							>{/if}
						{#if connection?.credential_detail}<Notice
								tone="warning"
								title="This key needs attention"
								details={connection.credential_detail}
							/>{/if}
						<label class="field-label"
							><span>Model</span><select
								value={advancedCustomModel ? '__custom' : advancedModel}
								title={catalog.source(selected, mode === 'oauth' ? 'oauth' : 'api_key') ===
								'connected'
									? 'Models from this account’s connected runtime'
									: 'Model suggestions; availability depends on this connection'}
								onchange={(event) => {
									advancedModelTouched = true;
									advancedCustomModel = event.currentTarget.value === '__custom';
									advancedModel = advancedCustomModel ? '' : event.currentTarget.value;
								}}
								disabled={busy}
							>
								<option value="">Choose a model…</option
								>{#each modelChoices(selected, mode === 'oauth' ? 'oauth' : 'api_key') as model}<option
										value={routeModel(selected, model.id)}>{model.name ?? model.id}</option
									>{/each}<option value="__custom">Custom model ID…</option>
							</select></label
						>
						{#if advancedCustomModel}<label class="field-label"
								><span>Model ID</span><input
									placeholder={`${selected}/model-id`}
									bind:value={advancedModel}
									disabled={busy}
								/></label
							>{/if}
						<label class="field-label"
							><span>Credential source</span><select
								bind:value={mode}
								onchange={selectMode}
								disabled={busy}
								><option value="infisical">API key stored securely</option><option value="env"
									>Host environment variable</option
								>{#if connection?.reference?.startsWith('omp-oauth:')}<option value="oauth"
										>Existing company OAuth reference</option
									>{/if}</select
							></label
						>
						<label
							class="field-label"
							title={mode === 'oauth'
								? 'Kept for compatibility. Give reusable sign-ins from Account → Connections.'
								: undefined}
							><span>Reference</span><input
								bind:value={reference}
								oninput={() => (edited = true)}
								disabled={busy}
								required
								spellcheck="false"
							/></label
						>
						<div class="bar">
							<button
								class="btn primary small"
								disabled={busy ||
									refreshing ||
									!selected ||
									selected === 'custom' ||
									!validModel(selected, advancedModel) ||
									(mode === 'infisical' && status.infisical_status !== 'present')}
								>{busy ? 'Saving…' : connection?.reference ? 'Save' : 'Connect'}</button
							>
							<button
								class="btn small ghost"
								type="button"
								disabled={busy}
								onclick={() => {
									editorOpen = false;
									secret = '';
								}}>Cancel</button
							>
							{#if connection?.reference}<button
									type="button"
									class="btn small danger"
									disabled={busy}
									onclick={() => connect(undefined, true)}>Disconnect</button
								>{/if}
						</div>
					</form>
				{/if}
			</div>
		</Fold>
		<Fold
			label="Model catalog"
			hint={catalog.pending
				? 'Refreshing…'
				: catalog.updatedAt
					? `Updated ${new Date(catalog.updatedAt).toLocaleDateString([], { dateStyle: 'medium' })}`
					: 'Bundled suggestions'}
		>
			<div class="pad bar">
				<span
					class="quiet"
					title="Restless checks models.dev hourly and keeps the last good catalog. Saved model choices are never changed automatically."
					>{catalog.failed
						? catalog.updatedAt
							? 'Using the cached model catalog'
							: 'Using bundled model suggestions'
						: 'Model list is current'}</span
				>
				<span class="spacer"></span>
				<button class="btn small" disabled={catalog.pending} onclick={() => catalog.refresh()}
					>Refresh models</button
				>
			</div>
		</Fold>
		<Fold label="Diagnostics">
			<div class="pad"><HarnessDiagnostics {companyId} /></div>
		</Fold>
		<Fold label="Custom harnesses" bind:open={harnessOpen}>
			<div class="pad" id="harnesses"><CustomHarnesses {companyId} /></div>
		</Fold>
	</Section>
</Page>

<style>
	.bad {
		color: var(--state-danger);
	}
	.grant {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-end;
		gap: 12px;
		padding-top: 4px;
	}
	.grant :global(.notice) {
		flex-basis: 100%;
	}
	.field-label {
		display: grid;
		gap: 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.field-label :is(select, input) {
		min-width: 200px;
	}
	.check {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-height: 32px;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
	}
	.bar.pad,
	.pad {
		padding: 12px 16px;
	}
	.spacer {
		flex: 1;
	}
	.field {
		width: min(280px, 100%);
	}
	.quiet {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-body);
		line-height: 1.55;
	}
	.import {
		display: grid;
		gap: 10px;
		padding: 12px 16px;
	}
	.keys {
		display: grid;
		gap: 10px;
	}
	.key-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		min-height: 40px;
		border-top: 1px solid var(--border);
		font-size: var(--t-body);
	}
	.key-row strong {
		flex: 1;
		font-weight: 500;
	}
	.badge {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.badge.success {
		color: var(--state-success);
	}
	.badge.warning {
		color: var(--intent-authority);
	}
	.badge.error {
		color: var(--state-danger);
	}
	.editor {
		display: grid;
		gap: 12px;
		padding: 14px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-pane);
	}
</style>
