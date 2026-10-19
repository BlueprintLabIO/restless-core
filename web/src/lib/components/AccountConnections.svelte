<script lang="ts">
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { reenterIfLapsed } from '$lib/model/reenter';
	import { failureSentence } from '$lib/model/failure';
	import { hostPlace, planeStatus, type ApplianceStatus } from '$lib/model/appliance';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { onMount, untrack } from 'svelte';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import { getCompanies, type CompanyCatalogEntry } from '$lib/model/cockpit';
	import { settingQuery } from '$lib/model/setting.svelte';
	import { readHarnessAuth } from '$lib/model/harness-auth';
	import {
		accountConnectionsKey,
		addApiKeyConnection,
		cancelSignIn,
		companyHasDefaultModel,
		companyProviderRevision,
		completeSignInCallback,
		grantAccountConnection,
		importCompanyCodexSignIn,
		readAccountConnections,
		readSignInJob,
		removeAccountConnection,
		revokeAccountConnection,
		startOAuthSignIn,
		type AccountConnection,
		type CompanyUse
	} from '$lib/model/account-connections';
	import { accountGrantFix } from '$lib/model/company-start';
	import { page } from '$app/state';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import Field from '$lib/ui/forms/Field.svelte';
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import {
		announceIntelligenceChange,
		watchIntelligenceChanges
	} from '$lib/model/intelligence-events';
	let plane = $state<ApplianceStatus | null>(null);
	void planeStatus().then((value) => (plane = value));
	const callbackId = $props.id();
	const catalog = modelCatalog();
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
	const source = settingQuery({
		key: () => accountConnectionsKey,
		read: async (signal) => {
			try {
				return await readAccountConnections(signal);
			} catch (cause) {
				// A hosted session that lapsed re-enters through the account's door.
				reenterIfLapsed((cause as { status?: number })?.status);
				throw cause;
			}
		},
		poll: (current) =>
			current?.connections.some((connection) => connection.status === 'checking') ? 3_000 : false
	});
	const connections = $derived(source.view?.connections ?? []);
	/* Providers the plan's included AI serves; they take no key of the owner's own. */
	const includedProviders = $derived(source.view?.includedProviders ?? []);
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
	const accountScope = $derived(source.view?.scope ?? 'account');
	const manageUrl = $derived(source.view?.manageUrl ?? '/account/connections');
	/* Only the first load shows a skeleton; a background read keeps the page as it is. */
	const loading = $derived(source.loading);
	let actionError = $state('');
	const error = $derived(
		actionError ||
			(source.failure && !source.view
				? failureSentence(source.failure, 'Could not load account connections.')
				: '')
	);
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

	const refresh = () => source.refresh();
	/* A sign-in that just completed brings its models into every picker. */
	let signedIn = '';
	$effect(() => {
		const now = connections
			.filter((item) => item.kind === 'oauth' && item.status === 'present')
			.map((item) => item.provider)
			.sort()
			.join();
		if (
			signedIn &&
			now !== signedIn &&
			now.split(',').some((p) => p && !signedIn.split(',').includes(p))
		)
			void catalog.refreshConnected();
		signedIn = now;
	});
	async function refreshNativeSignIns(rows: CompanyCatalogEntry[]) {
		nativeLoading = true;
		const results = await Promise.allSettled(
			rows.map(async (company) => {
				const rows = await readHarnessAuth(company.id).catch(() => {
					throw new Error(`Could not check ${company.name}.`);
				});
				return rows
					.filter((connection) => connection.mode === 'oauth')
					.map((connection) => ({
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
					asleep: 'Asleep',
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
		actionError = '';
		try {
			await importCompanyCodexSignIn(companyId);
			await refresh();
			announceIntelligenceChange();
		} catch (cause) {
			actionError = failureSentence(cause, 'Could not use this sign-in for the account.');
		} finally {
			importingCompany = '';
		}
	}
	/* Removing revokes the connection from every company and deletes its key. The server refuses
	 * while an agent still runs on it and says which company to change first. */
	async function removeConnection(item: AccountConnection) {
		const yes = await confirmAction({
			title: `Remove ${item.label}?`,
			body: 'Every company loses access to it, and its key is deleted from your vault.',
			action: 'Remove',
			danger: true
		});
		if (!yes) return;
		actionError = '';
		try {
			const body = await removeAccountConnection(item.id);
			if (managingId === item.id) managingId = '';
			await refresh();
			announceIntelligenceChange();
			toast(
				body.vault_deleted === false
					? `${item.label} removed. Its key is still in your vault; delete it there if you no longer need it.`
					: `${item.label} removed`
			);
		} catch (cause) {
			actionError = failureSentence(cause, 'Could not remove this connection.');
		}
	}
	let addError = $state('');
	let grantError = $state('');
	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		addError = !label.trim()
			? 'Add a name for this key.'
			: !secret.trim()
				? 'Paste the API key.'
				: '';
		if (addError) return;
		busy = true;
		actionError = '';
		try {
			const body = await addApiKeyConnection({ label: label.trim(), provider, secret });
			label = '';
			secret = '';
			addOpen = false;
			await refresh();
			announceIntelligenceChange();
			/* Saved, but the provider could not be asked whether it accepts the key. */
			if (body.unverified)
				toast(
					`${providerName(provider)} key saved. Restless couldn't check it yet, so the first reply will tell.`
				);
		} catch (cause) {
			secret = '';
			actionError = failureSentence(cause, 'Could not save this connection.');
		} finally {
			busy = false;
		}
	}
	/* The provider's page opens in its own tab. It is opened on the click itself, before the
	 * address exists, because a tab opened later is a popup the browser blocks. */
	let signInTab: Window | null = null;
	let signInTabSent = false;
	function sendSignInTab(url: string) {
		if (signInTabSent || !signInTab || signInTab.closed) return;
		signInTabSent = true;
		signInTab.opener = null;
		signInTab.location.replace(url);
	}
	/* Cancelled or finished, the provider's tab has nothing left to show. A provider page that cuts
	 * itself off from its opener stays open; the owner closes it. */
	function closeSignInTab() {
		if (signInTab && !signInTab.closed) signInTab.close();
		signInTab = null;
		signInTabSent = false;
	}
	async function startSignIn(provider: 'codex' | 'claude') {
		if (oauthJob) return;
		oauthProvider = provider;
		oauthMessage = '';
		oauthUrl = '';
		oauthCode = '';
		oauthCallback = '';
		oauthState = '';
		closeSignInTab();
		signInTab = window.open('', '_blank');
		try {
			const body = await startOAuthSignIn(provider);
			oauthJob = body.job;
			oauthState = 'starting';
			void pollSignIn(body.job);
		} catch (cause) {
			closeSignInTab();
			oauthMessage = failureSentence(cause, 'Could not start sign-in.');
		}
	}
	async function cancelSignInJob() {
		const job = oauthJob;
		oauthJob = '';
		oauthUrl = '';
		oauthCode = '';
		oauthCallback = '';
		oauthState = '';
		oauthMessage = '';
		closeSignInTab();
		if (job) await cancelSignIn(job).catch(() => {});
	}
	/* Claude's last page carries the code in its address. Pasting a whole one finishes at once. */
	function pastedCallback() {
		try {
			const url = new URL(oauthCallback.trim());
			if (url.searchParams.get('code') && url.searchParams.get('state'))
				void completeClaudeSignIn();
		} catch {
			/* Not a whole address yet. */
		}
	}
	async function completeClaudeSignIn() {
		if (!oauthJob || !oauthCallback.trim() || callbackBusy) return;
		callbackBusy = true;
		oauthMessage = '';
		try {
			await completeSignInCallback(oauthJob, oauthCallback.trim());
			oauthCallback = '';
			oauthState = 'completing';
		} catch (cause) {
			oauthMessage = failureSentence(cause, 'Could not finish Claude sign-in.');
		} finally {
			callbackBusy = false;
		}
	}
	async function pollSignIn(job: string) {
		/* Core stops an unfinished sign-in after 15 minutes. */
		for (let attempt = 0; attempt < 900 && oauthJob === job; attempt++) {
			try {
				const body = await readSignInJob(job);
				if (oauthJob !== job) return;
				oauthState = body.state;
				oauthUrl = body.url ?? '';
				if (oauthUrl) sendSignInTab(oauthUrl);
				oauthCode = body.code ?? '';
				oauthMessage = body.message ?? '';
				if (body.state === 'connected' || body.state === 'failed') {
					oauthJob = '';
					closeSignInTab();
					if (body.state === 'connected') {
						await refresh();
						announceIntelligenceChange();
					}
					return;
				}
			} catch (cause) {
				oauthMessage = failureSentence(cause, 'Could not check sign-in.');
				oauthJob = '';
				closeSignInTab();
				return;
			}
			await new Promise((resolve) => setTimeout(resolve, 1000));
		}
		if (oauthJob !== job) return;
		await cancelSignInJob();
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
	/* Until the person picks, the suggestion follows the catalog as it loads. A refresh of the
	 * connection list is not a reason to touch it. */
	$effect(() => {
		if (!managingId || !selectedCompany || modelTouched) return;
		const item = untrack(() => connections.find((row) => row.id === managingId));
		if (!item) return;
		const suggested = defaultModel(item.provider, item.kind);
		if (suggested !== untrack(() => selectedModel)) selectedModel = suggested;
	});
	async function grant(item: AccountConnection, companyId: string, replace = false) {
		if (busyCompany || !validModel(item.provider, selectedModel)) return;
		busyCompany = true;
		actionError = '';
		try {
			let revision = companyRevisions[companyId];
			if (!revision) {
				revision = await companyProviderRevision(companyId);
				companyRevisions = { ...companyRevisions, [companyId]: revision };
			}
			let body: Awaited<ReturnType<typeof grantAccountConnection>>;
			try {
				body = await grantAccountConnection(companyId, item.id, {
					model: selectedModel,
					make_default: selectedMakeDefault,
					revision,
					...(replace ? { replace_existing: true } : {})
				});
			} catch (cause) {
				const failure = cause as { status?: number; code?: string };
				if (failure?.status === 409 && failure.code === 'provider_conflict' && !replace) {
					replaceRequired = true;
					selectedCompany = companyId;
					return;
				}
				throw cause;
			}
			companyRevisions = { ...companyRevisions, [companyId]: body.provider?.revision ?? revision };
			await refresh();
			announceIntelligenceChange(companyId);
			selectedCompany = '';
			replaceRequired = false;
		} catch (cause) {
			actionError = failureSentence(cause, 'Could not grant this company access.');
		} finally {
			busyCompany = false;
		}
	}
	async function revoke(item: AccountConnection, company: CompanyUse) {
		if (busyCompany) return;
		busyCompany = true;
		actionError = '';
		try {
			let revision = companyRevisions[company.id];
			if (!revision) revision = await companyProviderRevision(company.id);
			const body = await revokeAccountConnection(company.id, item.id, revision);
			companyRevisions = { ...companyRevisions, [company.id]: body.provider?.revision ?? revision };
			confirmRevocation = '';
			await refresh();
			announceIntelligenceChange(company.id);
		} catch (cause) {
			actionError = failureSentence(cause, 'Could not remove this company’s access.');
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
		void companyHasDefaultModel(companyId).then((hasDefault) => {
			if (hasDefault !== null && managingId === item.id && selectedCompany === companyId)
				selectedMakeDefault = !hasDefault;
		});
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
		nativeSignIns.filter((signIn) => !['connected', 'asleep'].includes(signIn.state)).length
	);

	onMount(() => {
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
		return () => stopWatching();
	});
</script>

<svelte:head><title>Accounts — {PRODUCT_NAME}</title></svelte:head>

{#snippet accessManager(item: AccountConnection)}
	<div class="access-manager" aria-label={`Company access for ${item.label}`}>
		{#if companyError}<span class="replace-warning" role="alert">{companyError}</span>{/if}
		{#each companies as company (company.id)}
			{@const granted = item.companies.find((use) => use.id === company.id)}
			<div class="access-row" class:selected={selectedCompany === company.id}>
				<strong>{company.name}</strong>
				{#if granted}<span
						class="access-state"
						title={granted.models?.length
							? `${company.name} runs ${granted.models.map(modelLabel).join(', ')} through this connection.`
							: undefined}
						>{granted.in_use
							? granted.models?.length
								? granted.models.map(modelLabel).join(', ')
								: 'In use'
							: 'Has access'}</span
					><a
						class="text-button"
						href={`/${encodeURIComponent(company.id)}/company/provider`}
						title={`Choose which model ${company.name} uses, for everyone or per agent`}
						>{granted.in_use ? 'Change model' : 'Use it'}</a
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
					<span class="access-state">Choosing a model</span>
				{:else}<button
						class="btn small"
						disabled={busyCompany || item.status !== 'present'}
						onclick={() => beginGrant(item, company.id)}>Give access…</button
					>{/if}
			</div>
			{#if !granted && selectedCompany === company.id}
				<form
					class="form plain access-grant"
					onsubmit={(event) => {
						event.preventDefault();
						if (!validModel(item.provider, selectedModel)) {
							grantError = 'Choose a model first.';
							return;
						}
						grantError = '';
						void grant(item, company.id, replaceRequired);
					}}
				>
					<Field label="Model" for={`grant-model-${item.id}`}>
						<select
							id={`grant-model-${item.id}`}
							value={selectedCustomModel ? '__custom' : selectedModel}
							onchange={(event) => {
								modelTouched = true;
								grantError = '';
								selectedCustomModel = event.currentTarget.value === '__custom';
								selectedModel = selectedCustomModel ? '' : event.currentTarget.value;
							}}
							><option value="">Choose a model</option
							>{#each modelChoices(item.provider, item.kind) as model (model.id)}<option
									value={routeModel(item.provider, model.id)}>{model.name ?? model.id}</option
								>{/each}<option value="__custom">Custom model ID…</option></select
						>
						{#snippet help()}{catalog.source(item.provider, item.kind) === 'connected'
								? 'Models from this account’s connected runtime.'
								: 'Suggestions; what is available depends on this connection.'}{/snippet}
					</Field>
					{#if selectedCustomModel}
						<Field label="Full model ID" for={`grant-custom-${item.id}`}>
							<input
								id={`grant-custom-${item.id}`}
								placeholder={`${item.provider}/model-id`}
								bind:value={selectedModel}
							/>
						</Field>
					{/if}
					<Field label="Company default">
						<label class="form-check"
							><input type="checkbox" bind:checked={selectedMakeDefault} />Use it for everyone in {company.name}
							unless an agent has its own</label
						>
					</Field>
					{#if replaceRequired}<p class="form-error" role="alert">
							This replaces the current {providerName(item.provider)} connection for {company.name}.
						</p>{:else if grantError}<p class="form-error" role="alert">{grantError}</p>{/if}
					<FormFoot
						submit={replaceRequired ? 'Replace connection' : 'Give access'}
						busy={busyCompany}
						busyLabel={replaceRequired ? 'Replacing…' : 'Giving access…'}
						oncancel={() => {
							selectedCompany = '';
							replaceRequired = false;
							grantError = '';
						}}
						cancelLabel={replaceRequired ? 'Keep current' : 'Cancel'}
					/>
				</form>
			{/if}
		{/each}
		{#if !companies.length}<span class="unused">No active companies available.</span>{/if}
	</div>
{/snippet}

{#snippet signInProgress(provider: 'codex' | 'claude')}
	{#if oauthProvider === provider && (oauthUrl || oauthMessage || ['starting', 'connected', 'completing', 'waiting'].includes(oauthState))}
		{@const tabName = provider === 'codex' ? 'ChatGPT' : 'Claude'}
		<div class="sign-in-progress" aria-live="polite">
			{#if oauthJob && !oauthUrl}<p class="sign-in-step">Opening {tabName} sign-in…</p>
			{:else if provider === 'codex' && oauthUrl}<p class="sign-in-step">
					Enter <strong class="code">{oauthCode || '…'}</strong> in the {tabName} tab, then come back
					here.
					<a href={oauthUrl} target="_blank" rel="noreferrer">Open it again ↗</a>
				</p>
			{:else if provider === 'claude' && oauthUrl && oauthState === 'waiting'}<p
					class="sign-in-step"
				>
					{#if plane?.hosted}Approve in the {tabName} tab. It ends on a page that won’t load: copy that
						page’s address and paste it here.{:else}Approve in the {tabName} tab. If it ends on a page
						that won’t load, paste that page’s address here.{/if}
					<a href={oauthUrl} target="_blank" rel="noreferrer">Open it again ↗</a>
				</p>
				<div class="callback-field">
					<input
						id={callbackId}
						type="url"
						aria-label="Address of the page that won’t load"
						bind:value={oauthCallback}
						oninput={pastedCallback}
						placeholder="http://localhost:…/callback?code=…"
						autocomplete="off"
						spellcheck="false"
						disabled={callbackBusy}
					/>
					<button
						class="btn primary small"
						disabled={!oauthCallback.trim() || callbackBusy}
						onclick={() => void completeClaudeSignIn()}
						>{callbackBusy ? 'Finishing…' : 'Finish'}</button
					>
				</div>
			{/if}
			{#if oauthMessage}<p class="sign-in-step" role="status">{oauthMessage}</p>
			{:else if oauthState === 'completing'}<p class="sign-in-step" role="status">
					Finishing sign-in…
				</p>
			{:else if oauthState === 'connected'}<p class="sign-in-step" role="status">
					Connected. Give companies access with Manage access.
				</p>{/if}
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
		<form class="form add-form" onsubmit={create}>
			<header class="form-head"><strong>Add API key</strong></header>
			<Field label="Provider" for="key-provider">
				<select id="key-provider" bind:value={provider}
					>{#each providerOptions.filter(([id]) => !includedProviders.includes(id)) as [id, name]}<option
							value={id}>{name}</option
						>{/each}</select
				>
			</Field>
			<Field label="Name" for="key-label">
				<input
					id="key-label"
					bind:value={label}
					maxlength="80"
					placeholder={`${providerName(provider)} team key`}
				/>
			</Field>
			<Field label="API key" for="key-secret">
				<input
					id="key-secret"
					bind:value={secret}
					type="password"
					autocomplete="new-password"
					placeholder="Paste the key"
				/>
			</Field>
			{#if addError}<p class="form-error" role="alert">{addError}</p>{/if}
			<FormFoot
				note="No company uses it until you give access."
				submit="Save API key"
				{busy}
				busyLabel="Saving…"
				oncancel={() => {
					addOpen = false;
					addError = '';
				}}
			/>
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
										class="company-use"
										class:in-use={company.in_use}
										title={company.in_use ? 'In use by this company' : 'Available to this company'}
										href={`/${encodeURIComponent(company.id)}/company/provider`}>{company.name}</a
									>{/each}{:else if item}<span class="unused">No company access</span>{/if}
						</div>
						<span
							class="status"
							class:connected={signedIn}
							class:failed={item?.status === 'invalid' || item?.status === 'absent'}
							>{oauthJob && oauthProvider === row.oauth
								? 'Signing in'
								: item
									? statusText(item)
									: 'Not connected'}</span
						>
						<div class="provider-actions">
							{#if row.oauth && oauthJob && oauthProvider === row.oauth}<button
									class="btn small"
									onclick={() => void cancelSignInJob()}>Cancel</button
								>{:else if row.oauth && (!item || !signedIn) && accountScope === 'account'}<button
									class="btn small"
									class:primary={!!item}
									disabled={!!oauthJob || checking}
									onclick={() => void startSignIn(row.oauth!)}
									>{item ? 'Reconnect' : 'Connect'}</button
								>{/if}
							{#if item && accountScope === 'account'}<button
									class="btn small"
									aria-expanded={managingId === item.id}
									onclick={() => toggleManage(item)}
									>{managingId === item.id ? 'Done' : 'Manage access'}</button
								>{/if}
							{#if item && accountScope === 'account'}<ActionMenu label={`${row.title} options`}
									>{#if row.oauth && signedIn}<button
											disabled={!!oauthJob}
											onclick={() => void startSignIn(row.oauth!)}>Reconnect same account</button
										>{/if}<button onclick={() => void refresh()}>Refresh status</button><button
										class="danger"
										onclick={() => void removeConnection(item)}>Remove…</button
									></ActionMenu
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
	/* Companies as a short list of plain rows: a dot when one is using it now. */
	.provider-uses {
		display: grid;
		align-content: center;
		min-width: 0;
	}
	.company-use {
		display: flex;
		align-items: center;
		gap: 7px;
		min-width: 0;
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 20px;
		text-decoration: none;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.company-use::before {
		content: '';
		flex: none;
		width: 6px;
		height: 6px;
		border: 1px solid var(--text-tertiary);
		border-radius: 50%;
		box-sizing: border-box;
	}
	.company-use.in-use::before {
		border-color: var(--state-success);
		background: var(--state-success);
	}
	.company-use:hover {
		color: var(--ink);
	}
	.status {
		justify-self: end;
		color: var(--text-tertiary);
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
	/* Under the provider's name, in its column, never wider than a sentence and a field. */
	.sign-in-progress {
		display: grid;
		gap: 8px;
		max-width: 560px;
		margin: -2px 14px 14px 60px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.sign-in-step {
		margin: 0;
		line-height: 1.5;
	}
	.sign-in-step a {
		margin-left: 4px;
		color: var(--text-tertiary);
		white-space: nowrap;
	}
	.sign-in-step a:hover {
		color: var(--ink);
	}
	.code {
		font-family: var(--font-mono);
		letter-spacing: 0.08em;
		color: var(--ink);
	}
	.callback-field {
		display: flex;
		gap: 8px;
	}
	.callback-field .btn {
		flex: none;
	}
	.callback-field input {
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
		text-decoration: none;
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
		animation: bridge-popover-in var(--motion-disclosure) var(--ease-spring) both;
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
	:is(.native-row a, .company-use, .sign-in-step a, .text-button):focus-visible {
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
		.needs-access-row {
			grid-template-columns: 8px minmax(0, 1fr);
		}
		.needs-access-row button {
			grid-column: 2;
			justify-self: start;
		}
	}
</style>
