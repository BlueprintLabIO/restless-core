<script lang="ts">
	/* Accounts the owner holds beyond AI models (S98-07): website logins, API keys, browser
	 * sign-ins, GitHub and tool connections. One list, one row per account: what it is, which
	 * companies may use it, its health, and Manage. Manage opens the account's credentials and a
	 * grant per company, each edited in place: what the company may do (permissive by default,
	 * the owner's choice), optional limits, and who. Secrets go in and never come back out. */
	import { untrack } from 'svelte';
	import { page } from '$app/state';
	import AccountActivity from '$lib/components/AccountActivity.svelte';
	import AccountSignIn from '$lib/components/AccountSignIn.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import Field from '$lib/ui/forms/Field.svelte';
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
	import Segmented from '$lib/ui/forms/Segmented.svelte';
	import {
		ACCOUNT_KINDS,
		DEFAULT_ABILITIES,
		EMPTY_LIMITS,
		HEALTH_LABEL,
		MODES,
		PERSONAL_NOTICE,
		PRESETS,
		TIERS,
		grantSummary,
		kindOf,
		limitsSummary,
		money,
		presetOf,
		type Abilities,
		type Account,
		type AccountKind,
		type Grant,
		type SpendPeriod
	} from '$lib/model/owner-accounts';
	import type { AccountsClient } from '$lib/model/owner-accounts-client';

	let {
		client,
		companies
	}: {
		client: AccountsClient;
		/** The owner's companies, to grant to. */
		companies: { id: string; name: string }[];
	} = $props();

	let accounts = $state<Account[]>([]);
	let loaded = $state(false);
	let failure = $state('');
	let available = $state(true);
	const shown = $derived(accounts.filter((account) => account.kind !== 'ai_provider'));
	const companyName = (id: string) => companies.find((company) => company.id === id)?.name ?? id;

	async function refresh() {
		try {
			accounts = await client.list();
			failure = '';
		} catch (cause) {
			/* Not on this plane, or not this person's to manage: the section is not theirs to see. */
			const status = (cause as { status?: number }).status;
			if (status === 404 || status === 403) available = false;
			else failure = failureSentence(cause, 'Your accounts could not be loaded.');
		} finally {
			loaded = true;
		}
	}
	$effect(() => {
		untrack(() => void refresh());
	});

	/* ---- adding an account ---- */
	const ADDABLE: AccountKind[] = ['website_login', 'api', 'browser_profile', 'github'];
	let adding = $state(false);
	let addKind = $state<AccountKind>('website_login');
	let busy = $state(false);
	let form = $state({
		label: '',
		url: '',
		username: '',
		password: '',
		totp: '',
		personal: true,
		baseUrl: '',
		auth: 'bearer' as 'bearer' | 'header' | 'query' | 'basic',
		authName: '',
		secret: ''
	});
	function resetForm() {
		form = {
			label: '',
			url: '',
			username: '',
			password: '',
			totp: '',
			personal: true,
			baseUrl: '',
			auth: 'bearer',
			authName: '',
			secret: ''
		};
	}
	const domainOf = (url: string) => {
		try {
			return new URL(url.includes('://') ? url : `https://${url}`).hostname;
		} catch {
			return '';
		}
	};
	/* Owners type ebay.com.au/signin; the owner plane wants a full https:// URL. */
	const fullUrl = (url: string) => {
		const trimmed = url.trim();
		return !trimmed || /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
	};
	let addError = $state('');
	async function add(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		if (!canAdd) {
			addError = missingForAdd;
			return;
		}
		addError = '';
		busy = true;
		try {
			const label = form.label.trim();
			const created =
				addKind === 'website_login'
					? await client.create({
							kind: 'website_login',
							label: label || domainOf(form.url) || 'Website login',
							provider: domainOf(form.url) || null,
							personal: form.personal,
							credential: {
								kind: 'password',
								sign_in_url: fullUrl(form.url),
								domains: [domainOf(form.url)].filter(Boolean),
								username: form.username.trim(),
								password: form.password,
								...(form.totp.trim() ? { totp_secret: form.totp.trim() } : {})
							}
						})
					: addKind === 'api'
						? await client.create({
								kind: 'api',
								label: label || domainOf(form.baseUrl) || 'API key',
								provider: domainOf(form.baseUrl) || null,
								personal: false,
								credential: {
									kind: 'api_key',
									base_url: fullUrl(form.baseUrl),
									auth: form.auth,
									...(form.auth === 'header' || form.auth === 'query'
										? { auth_name: form.authName.trim() }
										: {}),
									secret: form.secret
								}
							})
						: addKind === 'github'
							? await client.create({
									kind: 'github',
									label: label || 'GitHub',
									provider: 'github.com',
									personal: form.personal,
									credential: { kind: 'token', secret: form.secret }
								})
							: await client.create({
									kind: 'browser_profile',
									label: label || domainOf(form.url) || 'Browser sign-in',
									provider: domainOf(form.url) || null,
									personal: form.personal,
									credential: { kind: 'browser_profile', url: fullUrl(form.url) }
								});
			accounts = [...accounts, created];
			adding = false;
			resetForm();
			managing = created.id;
			const asked = pendingGrant;
			pendingGrant = null;
			if (asked?.company) {
				request = asked.request;
				editGrant(created, null, asked.company);
				toast(
					`${created.label} added. Save ${companyName(asked.company)}'s access to answer the request.`
				);
			} else toast(`${created.label} added. Give a company access below.`);
		} catch (cause) {
			toast(failureSentence(cause, 'The account was not added.'), { tone: 'failure' });
		} finally {
			busy = false;
			form.password = '';
			form.secret = '';
			form.totp = '';
		}
	}
	/* What the form still needs, said when Add account is pressed too early. */
	const missingForAdd = $derived(
		addKind === 'website_login'
			? !domainOf(form.url)
				? 'Add the sign-in page.'
				: !form.username.trim()
					? 'Add the username or email.'
					: 'Add the password.'
			: addKind === 'api'
				? !domainOf(form.baseUrl)
					? 'Add the service address.'
					: !form.secret
						? 'Add the key or token.'
						: `Add the ${form.auth === 'header' ? 'header' : 'parameter'} name.`
				: addKind === 'github'
					? 'Add the token.'
					: 'Add the website.'
	);
	const canAdd = $derived(
		addKind === 'website_login'
			? !!domainOf(form.url) && !!form.username.trim() && !!form.password
			: addKind === 'api'
				? !!domainOf(form.baseUrl) &&
					!!form.secret &&
					(form.auth === 'bearer' || form.auth === 'basic' || !!form.authName.trim())
				: addKind === 'github'
					? !!form.secret.trim()
					: !!domainOf(form.url)
	);

	/* ---- managing one account ---- */
	let managing = $state('');
	function replace(next: Account) {
		accounts = accounts.map((account) => (account.id === next.id ? next : account));
	}
	async function freeze(account: Account, frozen: boolean) {
		try {
			replace(await client.update(account.id, { frozen }));
			toast(
				frozen ? `${account.label} frozen. No company can use it.` : `${account.label} unfrozen.`
			);
		} catch (cause) {
			toast(failureSentence(cause, 'That did not save.'), { tone: 'failure' });
		}
	}
	async function remove(account: Account) {
		const yes = await confirmAction({
			title: `Remove ${account.label}?`,
			body: 'Every company loses access to it and its saved credentials are deleted. What companies already made with it stays.',
			action: 'Remove account',
			danger: true
		});
		if (!yes) return;
		try {
			await client.remove(account.id);
			accounts = accounts.filter((row) => row.id !== account.id);
			toast(`${account.label} removed.`);
		} catch (cause) {
			toast(failureSentence(cause, 'The account was not removed.'), { tone: 'failure' });
		}
	}

	/* "Approve with limits" on an admin's request lands here (S98-09): the account opens with that
	 * company's grant in the editor, and saving it settles the request. */
	let request = $state('');
	let linked = false;
	/* A request for an account not held yet ("Add it" in the Inbox): Add account opens for its kind
	 * and site, and once it is added the company's access opens, so saving it answers the request. */
	let pendingGrant = $state<{ company: string; request: string } | null>(null);
	$effect(() => {
		if (linked || !loaded) return;
		const params = page.url.searchParams;
		const kind = params.get('kind') as AccountKind | null;
		if (!params.get('account') && kind && ADDABLE.includes(kind) && params.get('request')) {
			linked = true;
			untrack(() => {
				adding = true;
				addKind = kind;
				const site = params.get('site') ?? '';
				if (kind === 'api') form.baseUrl = site;
				else if (kind !== 'github') form.url = site;
				pendingGrant = {
					company: params.get('company') ?? '',
					request: params.get('request') ?? ''
				};
			});
			return;
		}
		const target = accounts.find((account) => account.id === params.get('account'));
		if (!target) return;
		linked = true;
		const company = params.get('company') ?? '';
		untrack(() => {
			managing = target.id;
			request = params.get('request') ?? '';
			if (company)
				editGrant(
					target,
					target.grants.find((grant) => grant.company === company) ?? null,
					company
				);
		});
	});

	/* ---- a grant, edited in place ---- */
	type Draft = {
		account: string;
		grant: string | null;
		company: string;
		abilities: Abilities;
		capOn: boolean;
		capDollars: string;
		capPeriod: SpendPeriod;
		scope: string;
		perDay: string;
		expires: string;
		quietOn: boolean;
		quietStart: string;
		quietEnd: string;
		everyone: boolean;
		actors: string;
	};
	let draft = $state<Draft | null>(null);
	let limitsOpen = $state(false);
	let abilitiesOpen = $state(false);
	let grantError = $state('');
	const SHORT_MODE: Record<string, string> = {
		allow: 'Allowed',
		tell: 'Tell me',
		ask: 'Ask first',
		off: 'Off'
	};
	const MODE_CHOICES = MODES.map(({ mode, label }) => ({
		value: mode,
		label: SHORT_MODE[mode] ?? label
	}));
	/* What an access level allows, in one line: "Read and Draft freely, Act telling you, asks before Spend, no Admin." */
	function abilitySummary(abilities: Abilities) {
		const by = (mode: string) =>
			TIERS.filter(({ tier }) => abilities[tier] === mode).map(({ label }) => label);
		const join = (names: string[]) =>
			names.length > 1 ? `${names.slice(0, -1).join(', ')} and ${names.at(-1)}` : names[0];
		const text = [
			by('allow').length ? `${join(by('allow'))} freely` : '',
			by('tell').length ? `${join(by('tell'))}, telling you` : '',
			by('ask').length ? `asks before ${join(by('ask'))}` : '',
			by('off').length ? `no ${join(by('off'))}` : ''
		]
			.filter(Boolean)
			.join(', ');
		return `${text.charAt(0).toUpperCase()}${text.slice(1)}.`;
	}
	function editGrant(account: Account, grant: Grant | null, company = '') {
		const cap = grant?.spend_cap ?? null;
		const limits = grant?.limits ?? EMPTY_LIMITS;
		draft = {
			account: account.id,
			grant: grant?.id ?? null,
			company: grant?.company ?? company,
			abilities: { ...(grant?.abilities ?? DEFAULT_ABILITIES) },
			capOn: !!cap,
			capDollars: cap ? String(cap.amount_micros / 1_000_000) : '50',
			capPeriod: cap?.period ?? 'week',
			scope: limits.scope.join(', '),
			perDay: limits.actions_per_day ? String(limits.actions_per_day) : '',
			expires: limits.expires_at?.slice(0, 10) ?? '',
			quietOn: !!limits.quiet_hours,
			quietStart: limits.quiet_hours?.start ?? '22:00',
			quietEnd: limits.quiet_hours?.end ?? '07:00',
			everyone: !grant || grant.who === 'everyone',
			actors: grant && grant.who !== 'everyone' ? grant.who.actors.join(', ') : ''
		};
		limitsOpen = limitsSummary(limits, grant?.who ?? 'everyone').length > 0;
		abilitiesOpen = !presetOf(draft.abilities);
		grantError = '';
	}
	function usePreset(abilities: Abilities) {
		if (draft) draft.abilities = { ...abilities };
	}
	async function saveGrant(event: SubmitEvent) {
		event.preventDefault();
		if (!draft || busy) return;
		if (!draft.company) {
			grantError = 'Choose a company first.';
			return;
		}
		grantError = '';
		const d = draft;
		const dollars = Number.parseFloat(d.capDollars);
		const input = {
			abilities: d.abilities,
			spend_cap:
				d.capOn && d.abilities.spend !== 'off' && dollars > 0
					? { amount_micros: Math.round(dollars * 1_000_000), period: d.capPeriod }
					: null,
			limits: {
				scope: d.scope
					.split(',')
					.map((item) => item.trim())
					.filter(Boolean),
				actions_per_day: Number.parseInt(d.perDay, 10) || null,
				expires_at: d.expires ? new Date(`${d.expires}T23:59:59`).toISOString() : null,
				quiet_hours: d.quietOn
					? {
							start: d.quietStart,
							end: d.quietEnd,
							tz: Intl.DateTimeFormat().resolvedOptions().timeZone
						}
					: null
			},
			who: d.everyone
				? ('everyone' as const)
				: {
						actors: d.actors
							.split(',')
							.map((item) => item.trim())
							.filter(Boolean)
					}
		};
		busy = true;
		try {
			const saved = d.grant
				? await client.updateGrant(d.account, d.grant, input)
				: await client.grant(d.account, { company: d.company, ...input });
			accounts = accounts.map((account) =>
				account.id === d.account
					? {
							...account,
							grants: d.grant
								? account.grants.map((grant) => (grant.id === saved.id ? saved : grant))
								: [...account.grants, saved]
						}
					: account
			);
			toast(`${companyName(d.company)} ${d.grant ? 'access updated' : 'can use it now'}.`);
			draft = null;
			if (request) {
				const settled = request;
				request = '';
				await client
					.settleRequest(settled, { decision: 'approved', grant_id: saved.id })
					.catch((cause) =>
						toast(
							failureSentence(cause, 'Access saved, but the request is still open in the Inbox.'),
							{
								tone: 'failure'
							}
						)
					);
			}
		} catch (cause) {
			toast(failureSentence(cause, 'That access was not saved.'), { tone: 'failure' });
		} finally {
			busy = false;
		}
	}
	async function removeGrant(account: Account, grant: Grant) {
		const yes = await confirmAction({
			title: `Stop ${companyName(grant.company)} using ${account.label}?`,
			body: 'It can no longer read or act with this account. What it already made stays.',
			action: 'Remove access'
		});
		if (!yes) return;
		try {
			await client.removeGrant(account.id, grant.id);
			replace({ ...account, grants: account.grants.filter((row) => row.id !== grant.id) });
		} catch (cause) {
			toast(failureSentence(cause, 'Access was not removed.'), { tone: 'failure' });
		}
	}
	async function freezeGrant(account: Account, grant: Grant, frozen: boolean) {
		try {
			const saved = await client.updateGrant(account.id, grant.id, { frozen });
			replace({
				...account,
				grants: account.grants.map((row) => (row.id === saved.id ? saved : row))
			});
		} catch (cause) {
			toast(failureSentence(cause, 'That did not save.'), { tone: 'failure' });
		}
	}
	const ungranted = (account: Account) =>
		companies.filter((company) => !account.grants.some((grant) => grant.company === company.id));
	const CRED_LABEL: Record<string, string> = {
		oauth: 'Sign-in',
		api_key: 'API key',
		password: 'Password',
		totp: 'Authenticator code',
		browser_profile: 'Browser session',
		app_install: 'App install'
	};
</script>

{#snippet grantEditor(account: Account)}
	{#if draft && draft.account === account.id}
		{@const preset = presetOf(draft.abilities)}
		<form class="form plain grant-editor" onsubmit={saveGrant}>
			<header class="form-head">
				<strong
					>{draft.grant
						? `${companyName(draft.company)}’s access`
						: 'Give a company access'}</strong
				>
			</header>
			{#if !draft.grant}
				<Field label="Company" for={`grant-company-${account.id}`}>
					<select id={`grant-company-${account.id}`} bind:value={draft.company}>
						<option value="" disabled>Choose a company</option>
						{#each ungranted(account) as company (company.id)}<option value={company.id}
								>{company.name}</option
							>{/each}
					</select>
				</Field>
			{/if}
			<Field label="Access" hint="What its people and agents may do">
				<Segmented
					label="Access"
					options={[
						...PRESETS.map((entry) => ({ value: entry.key, label: entry.label })),
						...(preset ? [] : [{ value: 'custom' as const, label: 'Custom' }])
					]}
					value={preset ?? 'custom'}
					onchange={(key) => {
						const chosen = PRESETS.find((entry) => entry.key === key);
						if (chosen) usePreset(chosen.abilities);
					}}
				/>
				<small class="field-hint"
					>{abilitySummary(draft.abilities)}
					<button
						type="button"
						class="text-button inline"
						onclick={() => (abilitiesOpen = !abilitiesOpen)}
						>{abilitiesOpen ? 'Hide details' : 'Customize'}</button
					></small
				>
			</Field>
			{#if abilitiesOpen}
				{#each TIERS as { tier, label, hint } (tier)}
					<Field {label} {hint}>
						<Segmented {label} options={MODE_CHOICES} bind:value={draft.abilities[tier]} />
					</Field>
				{/each}
			{/if}
			{#if draft.abilities.spend !== 'off'}
				<Field label="Spending" hint="Optional">
					<div class="field-inline">
						<label class="form-check" style="padding-top:0"
							><input type="checkbox" bind:checked={draft.capOn} />Without asking, up to $</label
						>
						<input
							class="money"
							inputmode="decimal"
							bind:value={draft.capDollars}
							disabled={!draft.capOn}
							aria-label="Spend cap in dollars"
						/>
						<select bind:value={draft.capPeriod} disabled={!draft.capOn} aria-label="Per">
							<option value="day">a day</option>
							<option value="week">a week</option>
							<option value="month">a month</option>
						</select>
					</div>
				</Field>
			{/if}
			<Field label="Limits" hint="Optional">
				{#if limitsOpen}
					<div class="limits">
						<label
							><span>Only these</span><input
								bind:value={draft.scope}
								placeholder="Repos, pages, stores or recipient domains"
							/></label
						>
						<label
							><span>Actions a day, at most</span><input
								inputmode="numeric"
								bind:value={draft.perDay}
								placeholder="No limit"
							/></label
						>
						<label><span>Until</span><input type="date" bind:value={draft.expires} /></label>
						<div class="field-inline">
							<label class="form-check" style="padding-top:0"
								><input type="checkbox" bind:checked={draft.quietOn} />Quiet hours</label
							><input
								type="time"
								bind:value={draft.quietStart}
								disabled={!draft.quietOn}
								aria-label="From"
							/>to<input
								type="time"
								bind:value={draft.quietEnd}
								disabled={!draft.quietOn}
								aria-label="To"
							/>
						</div>
						<label class="form-check"
							><input type="checkbox" bind:checked={draft.everyone} />Everyone in the company, and
							their agents</label
						>
						{#if !draft.everyone}
							<label
								><span>Only these people or agents</span><input
									bind:value={draft.actors}
									placeholder="Names, separated by commas"
								/></label
							>
						{/if}
					</div>
				{:else}<button type="button" class="text-button" onclick={() => (limitsOpen = true)}
						>Add limits</button
					>{/if}
			</Field>
			{#if grantError}<p class="form-error" role="alert">{grantError}</p>{/if}
			<FormFoot
				submit={draft.grant ? 'Save' : 'Give access'}
				{busy}
				busyLabel="Saving…"
				oncancel={() => (draft = null)}
			/>
		</form>
	{/if}
{/snippet}

{#if available}<section class="owner-accounts" aria-label="Other accounts">
		<header class="head">
			<h3>Other accounts</h3>
			<InfoTip
				text="Logins, API keys and apps you hold once and let companies use. Companies act with them through Restless and never see the password or key."
			/>
			{#if !adding}<button
					type="button"
					class="btn small"
					aria-expanded="false"
					onclick={() => {
						adding = true;
						draft = null;
					}}>Add account</button
				>{/if}
		</header>

		{#if adding}
			<form class="form add" onsubmit={add}>
				<header class="form-head">
					<strong>Add account</strong>
					<Segmented
						label="Kind of account"
						options={ACCOUNT_KINDS.filter((entry) => ADDABLE.includes(entry.kind)).map((entry) => ({
							value: entry.kind,
							label: entry.label,
							title: entry.hint
						}))}
						bind:value={addKind}
						onchange={() => (addError = '')}
					/>
				</header>
				<p class="form-intro">{kindOf(addKind).hint}</p>
				{#if addKind === 'website_login' || addKind === 'browser_profile'}
					<Field label={addKind === 'website_login' ? 'Sign-in page' : 'Website'} for="add-url">
						<input
							id="add-url"
							bind:value={form.url}
							placeholder="ebay.com.au/signin"
							autocomplete="off"
						/>
					</Field>
				{/if}
				{#if addKind === 'website_login'}
					<Field label="Username or email" for="add-username">
						<input id="add-username" bind:value={form.username} autocomplete="off" />
					</Field>
					<Field label="Password" for="add-password">
						<input
							id="add-password"
							type="password"
							bind:value={form.password}
							autocomplete="new-password"
						/>
					</Field>
					<Field
						label="Authenticator secret"
						hint="Optional, for sites that ask for a 6-digit code"
						for="add-totp"
					>
						<input id="add-totp" bind:value={form.totp} autocomplete="off" />
					</Field>
				{:else if addKind === 'api'}
					<Field label="Service address" for="add-base">
						<input
							id="add-base"
							bind:value={form.baseUrl}
							placeholder="api.example.com"
							autocomplete="off"
						/>
					</Field>
					<Field label="How the key is sent" for="add-auth">
						<select id="add-auth" bind:value={form.auth}>
							<option value="bearer">Bearer token</option>
							<option value="header">A header</option>
							<option value="query">A query parameter</option>
							<option value="basic">Basic sign-in</option>
						</select>
					</Field>
					{#if form.auth === 'header' || form.auth === 'query'}
						<Field
							label={form.auth === 'header' ? 'Header name' : 'Parameter name'}
							for="add-auth-name"
						>
							<input
								id="add-auth-name"
								bind:value={form.authName}
								placeholder={form.auth === 'header' ? 'X-Api-Key' : 'api_key'}
							/>
						</Field>
					{/if}
					<Field label="Key or token" for="add-secret">
						<input id="add-secret" type="password" bind:value={form.secret} autocomplete="off" />
					</Field>
				{:else if addKind === 'github'}
					<Field label="Token" for="add-token">
						<input
							id="add-token"
							type="password"
							bind:value={form.secret}
							autocomplete="off"
							placeholder="github_pat_…"
						/>
						{#snippet help()}A <a
								href="https://github.com/settings/personal-access-tokens/new"
								target="_blank"
								rel="noreferrer">fine-grained token</a
							> for just the repositories your companies need.{/snippet}
					</Field>
				{/if}
				<Field label="Name" hint="Optional" for="add-label">
					<input id="add-label" bind:value={form.label} placeholder={kindOf(addKind).label} />
				</Field>
				{#if addKind !== 'api'}
					<Field label="Personal login">
						<label class="form-check"
							><input type="checkbox" bind:checked={form.personal} /><span
								>It’s yours, not the business’s<small>{PERSONAL_NOTICE}</small></span
							></label
						>
					</Field>
				{/if}
				{#if addKind === 'browser_profile'}<p class="form-intro" style="padding-bottom:12px">
						After you add it, sign in once in your browser. Companies use that session, never the
						password.
					</p>{/if}
				{#if addError}<p class="form-error" role="alert">{addError}</p>{/if}
				<FormFoot
					note="Kept in your account’s vault and never shown again."
					submit="Add account"
					{busy}
					busyLabel="Adding…"
					oncancel={() => {
						adding = false;
						draft = null;
						addError = '';
					}}
				/>
			</form>
		{/if}

		{#if failure}
			<p class="failure" role="alert">{failure}</p>
		{:else if loaded && !shown.length && !adding}
			<p class="empty">
				No other accounts yet. Add a login, an API key or a browser sign-in, then let your companies
				use it.
			</p>
		{/if}

		{#if shown.length}
			<div class="list">
				{#each shown as account (account.id)}
					{@const open = managing === account.id}
					<article class="row" class:open class:frozen={account.frozen}>
						<div class="main">
							<span
								class="mark"
								class:live={account.health === 'signed_in' && !account.frozen}
								aria-hidden="true">{kindOf(account.kind).mark}</span
							>
							<div class="copy">
								<strong>{account.label}</strong>
								<small
									>{account.identity.display ??
										account.provider ??
										kindOf(account.kind).label}{account.personal ? ' · personal' : ''}</small
								>
							</div>
							<div class="uses">
								{#each account.grants as grant (grant.id)}<span
										class="chip"
										class:paused={grant.frozen}
										title={grantSummary(grant)}>{companyName(grant.company)}</span
									>{:else}<span class="unused">No company access</span>{/each}
							</div>
							{#if account.open_notices}
								<button
									type="button"
									class="notices"
									title="Uses you were told about and have not answered"
									onclick={() => {
										managing = account.id;
										draft = null;
									}}>{account.open_notices} to look at</button
								>
							{/if}
							<span class="status" class:ok={account.health === 'signed_in'}
								>{account.frozen ? 'Frozen' : HEALTH_LABEL[account.health]}</span
							>
							<div class="row-actions">
								<button
									type="button"
									class="btn small"
									aria-expanded={open}
									onclick={() => {
										managing = open ? '' : account.id;
										draft = null;
									}}>{open ? 'Done' : 'Manage'}</button
								>
								<ActionMenu label={`${account.label} options`}>
									<button type="button" onclick={() => void freeze(account, !account.frozen)}
										>{account.frozen ? 'Unfreeze' : 'Freeze everywhere'}</button
									>
									<button type="button" onclick={() => void remove(account)}>Remove account…</button
									>
								</ActionMenu>
							</div>
						</div>
						{#if open}
							<div class="detail">
								{#if account.credentials.length}
									<p class="creds">
										{#each account.credentials as credential, i (credential.id)}{i
												? ' · '
												: ''}{credential.label ?? CRED_LABEL[credential.kind]}: {HEALTH_LABEL[
												credential.health
											]}{/each}
									</p>
								{/if}
								{#if account.kind === 'browser_profile' || account.kind === 'website_login'}
									<div class="detail-block sign-in-block">
										<AccountSignIn {client} {account} onsigned={replace} />
									</div>
								{/if}
								{#each account.grants as grant (grant.id)}
									{#if draft?.grant === grant.id}
										{@render grantEditor(account)}
									{:else}
										<div class="grant" class:paused={grant.frozen}>
											<div class="grant-copy">
												<strong>{companyName(grant.company)}</strong>
												<span>{grant.frozen ? 'Paused' : grantSummary(grant)}</span>
												{#each limitsSummary(grant.limits, grant.who) as limit (limit)}<small
														>{limit}</small
													>{/each}
											</div>
											<div class="grant-buttons">
												<button type="button" class="link" onclick={() => editGrant(account, grant)}
													>Change</button
												>
												<button
													type="button"
													class="link quiet"
													onclick={() => void freezeGrant(account, grant, !grant.frozen)}
													>{grant.frozen ? 'Resume' : 'Pause'}</button
												>
												<button
													type="button"
													class="link quiet"
													onclick={() => void removeGrant(account, grant)}>Remove</button
												>
											</div>
										</div>
									{/if}
								{/each}
								{#if draft && draft.account === account.id && !draft.grant}
									{@render grantEditor(account)}
								{:else if ungranted(account).length}
									<button
										type="button"
										class="btn small add-grant"
										onclick={() => editGrant(account, null)}>Give a company access</button
									>
								{/if}
								<div class="detail-block">
									<AccountActivity
										{client}
										{account}
										{companyName}
										onchanged={() => void refresh()}
									/>
								</div>
							</div>
						{/if}
					</article>
				{/each}
			</div>
		{/if}
	</section>{/if}

<style>
	.owner-accounts {
		display: grid;
		gap: 12px;
		margin-top: 28px;
		min-width: 0;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.notices {
		padding: 2px 8px;
		border: 1px solid var(--state-warning, var(--border));
		border-radius: 999px;
		background: none;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-label);
		white-space: nowrap;
		cursor: pointer;
	}
	.head h3 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.head .btn {
		margin-left: auto;
	}
	.add {
		display: grid;
		gap: 12px;
		padding: 14px;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
	}
	.mark {
		display: grid;
		place-items: center;
		min-width: 28px;
		height: 24px;
		padding: 0 4px;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font: 600 var(--t-label)/1 var(--font-mono);
	}
	.mark.live {
		border-color: color-mix(in srgb, var(--state-success) 35%, var(--border));
		color: var(--state-success);
	}
	.empty {
		margin: 0;
		color: var(--text-tertiary);
	}
	/* The optional limits: small stacked fields inside the Limits row. */
	.limits {
		display: grid;
		gap: 10px;
		width: 100%;
		max-width: 420px;
	}
	.limits > label:not(.form-check) {
		display: grid;
		gap: 4px;
		min-width: 0;
	}
	.limits > label:not(.form-check) > span {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.money {
		width: 80px !important;
	}
	.grant-editor {
		margin: 8px 12px 12px;
	}
	.text-button.inline {
		min-height: 0;
		padding: 0 2px;
		font-size: inherit;
		text-decoration: underline;
		text-underline-offset: 2px;
	}
	.failure {
		margin: 0;
		color: var(--state-danger);
	}
	.list {
		display: grid;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
		overflow: hidden;
	}
	.row + .row {
		border-top: 1px solid var(--border);
	}
	.row.open {
		background: color-mix(in srgb, var(--surface-alt) 45%, var(--surface-pane));
	}
	.row.frozen .copy,
	.row.frozen .uses {
		opacity: 0.6;
	}
	.main {
		display: grid;
		grid-template-columns: 36px minmax(150px, 1fr) minmax(0, 1.1fr) auto auto;
		align-items: center;
		gap: 14px;
		min-height: 62px;
		padding: 10px 12px 10px 14px;
	}
	.main .mark {
		width: 32px;
		height: 32px;
		border-radius: 8px;
	}
	.copy {
		display: grid;
		gap: 1px;
		min-width: 0;
	}
	.copy strong {
		font-weight: 600;
	}
	.copy small {
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.uses {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		min-width: 0;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 3px 8px;
		border: 1px solid var(--border);
		border-radius: 99px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.chip::before {
		content: '';
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--state-success);
	}
	.chip.paused::before {
		background: var(--text-tertiary);
	}
	.unused {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.status {
		justify-self: end;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: nowrap;
	}
	.status.ok {
		color: var(--state-success);
	}
	.row-actions {
		display: flex;
		align-items: center;
		gap: 4px;
	}
	.detail {
		display: grid;
		gap: 0;
		margin: 0 12px 12px 64px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		overflow: hidden;
	}
	.detail-block {
		padding: 8px 12px 10px;
	}
	.sign-in-block {
		border-bottom: 1px solid var(--border);
	}
	.creds {
		margin: 0;
		padding: 8px 12px;
		border-bottom: 1px solid var(--border);
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.grant {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		min-height: 44px;
		padding: 8px 12px;
	}
	.grant + .grant,
	.grant + .grant-editor,
	.grant-editor + .grant {
		border-top: 1px solid var(--border);
	}
	.grant.paused .grant-copy {
		opacity: 0.6;
	}
	.grant-copy {
		display: grid;
		gap: 2px;
		min-width: 0;
	}
	.grant-copy strong {
		font-weight: 600;
	}
	.grant-copy span,
	.grant-copy small {
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.grant-copy small {
		color: var(--text-tertiary);
	}
	.grant-buttons {
		display: flex;
		flex: none;
		gap: 10px;
	}
	.link {
		padding: 0;
		border: 0;
		background: none;
		color: var(--intent-conversation);
		font: inherit;
		font-size: var(--t-label);
		font-weight: 500;
		cursor: pointer;
		justify-self: start;
	}
	.link.quiet {
		color: var(--text-tertiary);
	}
	.link:hover {
		text-decoration: underline;
	}
	.add-grant {
		justify-self: start;
		margin: 10px 12px;
	}
	.grant-editor + .add-grant {
		display: none;
	}
	@media (max-width: 760px) {
		.main {
			grid-template-columns: 36px minmax(0, 1fr) auto;
		}
		.uses,
		.status,
		.row-actions {
			grid-column: 2 / -1;
			justify-self: start;
		}
		.detail {
			margin-left: 12px;
		}
	}
</style>
