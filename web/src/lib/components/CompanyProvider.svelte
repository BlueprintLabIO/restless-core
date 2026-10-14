<script lang="ts">
	import { Page, Section, Item, Notice, Empty, Dot, Fold } from '$lib/ui/page';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import { failureSentence } from '$lib/model/failure';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import Pending from '$lib/ui/feedback/Pending.svelte';
	import { onMount, tick, untrack } from 'svelte';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	const catalog = modelCatalog();
	import HarnessConnections from './HarnessConnections.svelte';
	import HarnessDiagnostics from './HarnessDiagnostics.svelte';
	import CustomHarnesses from './CustomHarnesses.svelte';
	import AgentIntelligence from './AgentIntelligence.svelte';
	import CopyCompanySetting from './CopyCompanySetting.svelte';
	import { companiesQuery } from '$lib/model/queries.svelte';
	import { needsAnotherModel, startLinkLabel } from '$lib/model/company-start';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import {
		announceIntelligenceChange,
		watchIntelligenceChanges,
		affectsCompany
	} from '$lib/model/intelligence-events';
	import { settingQuery } from '$lib/model/setting.svelte';
	import {
		grantCompanyConnection,
		importConnection,
		providerKey,
		readProvider,
		readReusableConnections,
		reusableConnectionsKey,
		revokeCompanyConnection,
		saveProvider,
		type ProviderConnection as Connection,
		type ProviderStatus,
		type ReusableConnection
	} from '$lib/model/provider';
	import { getApplianceStatus } from '$lib/model/appliance';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	let { companyId }: { companyId: string } = $props();
	/* On Cloud the account supplies intelligence: this computer has no harness sign-ins, key storage,
	 * diagnostics or custom harnesses of its own, so those stay out of view instead of retrying. */
	let hosted = $state(false);
	onMount(() => {
		void getApplianceStatus()
			.then((status) => (hosted = !!status.hosted))
			.catch(() => {});
	});
	const catalogProjection = companiesQuery();
	const setupIssue = $derived(
		catalogProjection.view.find((company) => company.id === companyId)?.unstartable_reason
	);
	/* A model the connection does not serve is fixed in Models; the connection itself is fine. */
	const connectionIssue = $derived(setupIssue && !needsAnotherModel(setupIssue) ? setupIssue : '');
	const intelligence = $derived(intelligenceQuery(companyId));
	let editorOpen = $state(false);
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
	const providerSource = settingQuery<ProviderStatus>({
		key: () => providerKey(companyId),
		read: (signal) => readProvider(companyId, signal)
	});
	const status = $derived(providerSource.view);
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
	const reusableSource = settingQuery({
		key: () => reusableConnectionsKey(companyId),
		read: (signal) => readReusableConnections(signal)
	});
	const reusableConnections = $derived(reusableSource.view?.connections ?? []);
	/* Providers the plan's included AI serves; they take no key of the owner's own. */
	const accountScope = $derived(reusableSource.view?.scope ?? 'account');
	const manageUrl = $derived(reusableSource.view?.manageUrl ?? '/account/connections');
	let accountActionError = $state('');
	const accountError = $derived(
		accountActionError ||
			(reusableSource.failure
				? failureSentence(reusableSource.failure, 'Could not load reusable connections.')
				: '')
	);
	let accountBusy = $state(false);
	/* Only the first load shows a loading row; a refresh keeps the rows where they are. */
	const accountLoading = $derived(reusableSource.loading);
	let grantSelection = $state('');
	let replaceGrantId = $state('');
	let grantMakeDefault = $state(false);
	let grantModels = $state<Record<string, string>>({});
	let grantCustomModels = $state<Record<string, boolean>>({});
	const companies = $derived(reusableSource.view?.companies ?? []);
	const otherCompanies = $derived(companies.filter((company) => company.id !== companyId));
	let importOpen = $state(false);
	let importSource = $state('');
	let importProviders = $state<Connection[]>([]);
	let importProvider = $state('');
	let importLabel = $state('');
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
		refreshing = true;
		try {
			await providerSource.refresh();
		} finally {
			refreshing = false;
		}
	}
	/* The editor follows the saved state unless the owner is changing it. */
	$effect(() => {
		const body = providerSource.view;
		if (!body) return;
		untrack(() => {
			if (!selected && !editorOpen) {
				choose(body.primary_provider === 'unconfigured' ? '' : body.primary_provider);
			} else if (!edited && !busy) {
				const previousNotice = notice;
				choose(selected);
				notice = previousNotice;
			}
		});
	});
	const providerError = $derived(
		providerSource.failure
			? failureSentence(providerSource.failure, 'Could not read connections.')
			: ''
	);
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
		try {
			const saved = await saveProvider(companyId, {
				provider: selected,
				model: advancedModel.slice(selected.length + 1),
				reference,
				secret: secret || undefined,
				revision: status.revision,
				disconnect
			});
			secret = '';
			providerSource.accept(saved);
			await intelligence.refresh();
			announceIntelligenceChange(companyId);
			editorOpen = false;
			choose(selected);
			notice = disconnect
				? 'Disconnected. The route unloads within a few seconds.'
				: 'Connection saved. It loads within a few seconds.';
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
	const refreshReusableConnections = () => reusableSource.refresh();
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
		grantMakeDefault = !(intelligence.view?.default ?? intelligence.view?.base_default);
		replaceGrantId = '';
		accountActionError = '';
	}
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
		return grantCompanyConnection(companyId, id, {
			model,
			make_default: makeDefault,
			revision: status.revision,
			...(replaceExisting ? { replace_existing: true } : {})
		});
	}
	/* A key just added is checked and its company restarts on it, for some seconds. Until that has
	 * settled the row says so steadily, instead of passing through "Key unavailable". */
	let settling = $state(false);
	let settleTimer: ReturnType<typeof setTimeout> | undefined;
	function settle() {
		settling = true;
		const until = Date.now() + 60_000;
		clearTimeout(settleTimer);
		const tick = async () => {
			await Promise.all([refreshReusableConnections(), catalogProjection.refresh?.()]).catch(
				() => {}
			);
			const ready =
				!connectionIssue && reusableConnections.every((item) => item.status === 'present');
			if (ready || Date.now() > until) settling = false;
			else settleTimer = setTimeout(() => void tick(), 2_000);
		};
		settleTimer = setTimeout(() => void tick(), 1_000);
	}
	async function grantConnection(connection: ReusableConnection) {
		if (!status || accountBusy) return;
		accountBusy = true;
		accountActionError = '';
		try {
			const result = await requestGrant(
				connection.id,
				modelForGrant(connection),
				replaceGrantId === connection.id,
				grantMakeDefault
			);
			if (result.replaceRequired) {
				replaceGrantId = connection.id;
				accountActionError = '';
				return;
			}
			providerSource.accept(result.provider);
			grantSelection = '';
			replaceGrantId = '';
			settle();
			await Promise.all([refreshReusableConnections(), intelligence.refresh()]);
			announceIntelligenceChange(companyId);
		} catch (cause) {
			accountActionError = failureSentence(cause, 'Could not use this connection in the company.');
		} finally {
			accountBusy = false;
		}
	}
	async function revokeConnection(connection: ReusableConnection) {
		if (!status || accountBusy) return;
		accountBusy = true;
		accountActionError = '';
		try {
			providerSource.accept(
				await revokeCompanyConnection(companyId, connection.id, status.revision)
			);
			await Promise.all([refreshReusableConnections(), intelligence.refresh()]);
			announceIntelligenceChange(companyId);
		} catch (cause) {
			accountActionError = failureSentence(cause, 'Could not remove this company’s access.');
		} finally {
			accountBusy = false;
		}
	}
	async function loadImportProviders() {
		importProviders = [];
		importProvider = '';
		if (!importSource || importSource === companyId) return;
		try {
			const body = await readProvider(importSource);
			importProviders = (body.connections ?? []).filter(
				(row: Connection) => row.shareable_api_key && row.credential_status === 'present'
			);
			importProvider = importProviders[0]?.provider ?? '';
			accountActionError = importProviders.length
				? ''
				: 'No available API-key connections were found in that company.';
		} catch (cause) {
			accountActionError = failureSentence(cause, 'Could not inspect that company.');
		}
	}
	async function importReusableConnection() {
		if (accountBusy || !importSource || !importProvider) return;
		accountBusy = true;
		accountActionError = '';
		try {
			const imported = await importConnection({
				source_company: importSource,
				provider: importProvider,
				label: importLabel.trim() || `${labels[importProvider] ?? importProvider} API`
			});
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
				accountActionError = '';
			} else {
				providerSource.accept(grantResult.provider);
				notice = 'Connection imported and made available to this company.';
			}
		} catch (cause) {
			accountActionError = failureSentence(cause, 'Could not make this connection reusable.');
		} finally {
			accountBusy = false;
		}
	}
	onMount(() => {
		const stopWatching = watchIntelligenceChanges((changed) => {
			if (!affectsCompany(changed, companyId)) return;
			void refresh();
			void refreshReusableConnections();
		}, true);
		return () => {
			stopWatching();
			secret = '';
		};
	});
	holdQueries(() => [intelligence]);
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
			>{accountScope === 'company' ? 'Open account' : 'AI accounts'}
			<span aria-hidden="true">↗</span></a
		>
	{/snippet}

	{#if setupIssue}
		<Notice tone="warning" title={startLinkLabel(setupIssue)} details={setupIssue}>
			{#snippet actions()}<a
					class="btn small primary"
					href={connectionIssue ? manageUrl : '#models'}>Fix</a
				>{/snippet}
		</Notice>
	{/if}
	{#if error}<Notice tone="danger" title="That change was not saved" details={error} />{/if}
	{#if providerError && !status}<Notice
			tone="danger"
			title="Could not read this company’s connections"
			details={providerError}
		/>{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}

	<AgentIntelligence {companyId} />

	<Section
		title="AI accounts"
		info="Sign-ins and API keys for AI models belong to your account, not to one company, so one sign-in can serve every company. Add or connect them in Account → AI accounts, then give each company access. A company uses only the connections you give it."
		count={reusableConnections.length || null}
	>
		{#snippet actions()}
			<a
				class="btn small"
				href="/account/connections"
				title="Sign-ins and API keys for AI models are added once, in your account, and then given to each company that should use them."
				>{accountScope === 'account' ? 'Add in your account' : 'AI accounts'}</a
			>
		{/snippet}
		{#if accountLoading}
			<Empty compact title="Loading connections…" />
		{:else}
			{#each reusableConnections as item (item.id)}
				{@const companyGrant = item.companies.find((company) => company.id === companyId)}
				{@const pending =
					item.status === 'checking' ||
					(settling && (item.status !== 'present' || (companyGrant?.in_use && !!connectionIssue)))}
				{@const broken =
					!pending && (item.status !== 'present' || (companyGrant?.in_use && !!connectionIssue))}
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
							tone={pending
								? 'progress'
								: broken
									? 'danger'
									: companyGrant?.in_use
										? 'success'
										: companyGrant
											? 'progress'
											: 'muted'}
							label={pending
								? 'Getting ready'
								: broken
									? 'Unavailable'
									: companyGrant?.in_use
										? 'In use'
										: companyGrant
											? 'Available'
											: 'Not given access'}
						/>{/snippet}
					{#snippet trailing()}
						{#if pending}
							<Pending
								label={item.status === 'present' ? 'Starting with this key…' : 'Checking key…'}
							/>
						{:else if broken}
							<span class="bad" title={item.detail ?? 'Check this connection in your account.'}
								>{item.kind === 'oauth' ? 'Sign-in unavailable' : 'Key unavailable'}</span
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
		info={hosted
			? 'The model catalog this company chooses from.'
			: 'Sign-ins and keys that belong to this company only, the model catalog, diagnostics and custom harnesses.'}
	>
		{#if !hosted}<Fold label="Company-only sign-ins">
				<div class="pad"><HarnessConnections {companyId} /></div>
			</Fold>
			<Fold label="Company-only API keys" count={savedCount || null} bind:open={keysOpen}>
				<div class="pad keys">
					<div class="bar">
						<Dot
							show
							tone={status?.infisical_status === 'present'
								? 'success'
								: status
									? 'danger'
									: 'muted'}
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
							title="Only for a key this company alone uses, or a host environment variable. Keys most companies share are added in Account → AI accounts."
							onclick={() => {
								choose('');
								editorOpen = true;
							}}>Add company-only key</button
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
									? 'Kept for compatibility. Give reusable sign-ins from Account → AI accounts.'
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
		{/if}
		<Fold
			label="Model catalog"
			hint={catalog.pending
				? 'Refreshing…'
				: catalog.updatedAt
					? `Updated ${new Date(catalog.updatedAt).toLocaleDateString([], { dateStyle: 'medium' })}`
					: 'Bundled suggestions'}
		>
			<div class="pad bar">
				<span class="quiet"
					>{catalog.failed
						? catalog.updatedAt
							? 'Using the cached model catalog'
							: 'Using bundled model suggestions'
						: 'Model list is current'}</span
				>
				<InfoTip
					text={`${PRODUCT_NAME} checks models.dev hourly and keeps the last good catalog. Saved model choices are never changed automatically.`}
				/>
				<span class="spacer"></span>
				<button class="btn small" disabled={catalog.pending} onclick={() => catalog.refresh()}
					>Refresh models</button
				>
			</div>
		</Fold>
		{#if !hosted}
			<Fold label="Diagnostics">
				<div class="pad"><HarnessDiagnostics {companyId} /></div>
			</Fold>
			<Fold label="Custom harnesses" bind:open={harnessOpen}>
				<div class="pad" id="harnesses"><CustomHarnesses {companyId} /></div>
			</Fold>
		{/if}
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
