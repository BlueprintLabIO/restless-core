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
			if ((cause as { status?: number }).status === 404) available = false;
			else failure = failureSentence(cause, 'Your accounts could not be loaded.');
		} finally {
			loaded = true;
		}
	}
	$effect(() => {
		untrack(() => void refresh());
	});

	/* ---- adding an account ---- */
	const ADDABLE: AccountKind[] = ['website_login', 'api', 'browser_profile'];
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
	async function add(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
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
			toast(`${created.label} added. Give a company access below.`);
		} catch (cause) {
			toast(failureSentence(cause, 'The account was not added.'), { tone: 'failure' });
		} finally {
			busy = false;
			form.password = '';
			form.secret = '';
			form.totp = '';
		}
	}
	const canAdd = $derived(
		addKind === 'website_login'
			? !!domainOf(form.url) && !!form.username.trim() && !!form.password
			: addKind === 'api'
				? !!domainOf(form.baseUrl) &&
					!!form.secret &&
					(form.auth === 'bearer' || form.auth === 'basic' || !!form.authName.trim())
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
	$effect(() => {
		if (linked || !loaded) return;
		const params = page.url.searchParams;
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
	}
	function usePreset(abilities: Abilities) {
		if (draft) draft.abilities = { ...abilities };
	}
	async function saveGrant(event: SubmitEvent) {
		event.preventDefault();
		if (!draft || busy || !draft.company) return;
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
		<form class="grant-editor" onsubmit={saveGrant}>
			<div class="grant-editor-head">
				<strong>{draft.grant ? companyName(draft.company) : 'Give a company access'}</strong>
				{#if !draft.grant}
					<select bind:value={draft.company} aria-label="Company">
						<option value="" disabled>Choose a company…</option>
						{#each ungranted(account) as company (company.id)}<option value={company.id}
								>{company.name}</option
							>{/each}
					</select>
				{/if}
			</div>
			<div class="presets" role="group" aria-label="Quick choices">
				{#each PRESETS as preset (preset.key)}
					<button
						type="button"
						class:on={presetOf(draft.abilities) === preset.key}
						aria-pressed={presetOf(draft.abilities) === preset.key}
						onclick={() => usePreset(preset.abilities)}>{preset.label}</button
					>
				{/each}
				{#if !presetOf(draft.abilities)}<span class="custom">Custom</span>{/if}
			</div>
			<div class="tiers">
				{#each TIERS as { tier, label, hint } (tier)}
					<label class="tier">
						<span title={hint}>{label}</span>
						<select bind:value={draft.abilities[tier]} aria-label={`${label}: ${hint}`}>
							{#each MODES as { mode, label: modeLabel } (mode)}<option value={mode}
									>{modeLabel}</option
								>{/each}
						</select>
					</label>
				{/each}
			</div>
			{#if draft.abilities.spend !== 'off'}
				<label class="cap">
					<input type="checkbox" bind:checked={draft.capOn} />
					<span>Spend without asking up to</span>
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
				</label>
			{/if}
			<button type="button" class="link" onclick={() => (limitsOpen = !limitsOpen)}
				>{limitsOpen ? 'Hide limits' : 'Add limits (optional)'}</button
			>
			{#if limitsOpen}
				<div class="limits">
					<label
						><span>Only these (repos, pages, stores, recipient domains)</span><input
							bind:value={draft.scope}
							placeholder="Anything, when empty"
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
					<label class="inline"
						><input type="checkbox" bind:checked={draft.quietOn} /><span>Quiet hours</span><input
							type="time"
							bind:value={draft.quietStart}
							disabled={!draft.quietOn}
							aria-label="From"
						/><span>to</span><input
							type="time"
							bind:value={draft.quietEnd}
							disabled={!draft.quietOn}
							aria-label="To"
						/></label
					>
					<label class="inline"
						><input type="checkbox" bind:checked={draft.everyone} /><span
							>Everyone in the company, and their agents</span
						></label
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
			{/if}
			<div class="grant-actions">
				<button class="btn small primary" disabled={busy || !draft.company}
					>{busy ? 'Saving…' : draft.grant ? 'Save' : 'Give access'}</button
				>
				<button type="button" class="btn small ghost" onclick={() => (draft = null)}>Cancel</button>
			</div>
		</form>
	{/if}
{/snippet}

{#if available}<section class="owner-accounts" aria-label="Other accounts">
		<header class="head">
			<h3>Other accounts</h3>
			<InfoTip
				text="Logins, API keys and apps you hold once and let companies use. Companies act with them through Restless and never see the password or key."
			/>
			<button
				type="button"
				class="btn small"
				aria-expanded={adding}
				onclick={() => {
					adding = !adding;
					draft = null;
				}}>{adding ? 'Cancel' : 'Add account'}</button
			>
		</header>

		{#if adding}
			<form class="add" onsubmit={add}>
				<div class="kinds" role="radiogroup" aria-label="Kind of account">
					{#each ACCOUNT_KINDS.filter((entry) => ADDABLE.includes(entry.kind) || entry.kind === 'github') as entry (entry.kind)}
						<button
							type="button"
							role="radio"
							aria-checked={addKind === entry.kind}
							class:on={addKind === entry.kind}
							disabled={entry.kind === 'github'}
							title={entry.kind === 'github' ? 'The GitHub App arrives next' : entry.hint}
							onclick={() => (addKind = entry.kind)}
							><span class="mark" aria-hidden="true">{entry.mark}</span
							>{entry.label}{#if entry.kind === 'github'}<small>Soon</small>{/if}</button
						>
					{/each}
				</div>
				<p class="hint">{kindOf(addKind).hint}</p>
				<div class="fields">
					{#if addKind === 'website_login' || addKind === 'browser_profile'}
						<label
							><span>{addKind === 'website_login' ? 'Sign-in page' : 'Website'}</span><input
								bind:value={form.url}
								placeholder="ebay.com.au/signin"
								autocomplete="off"
							/></label
						>
					{/if}
					{#if addKind === 'website_login'}
						<label
							><span>Username or email</span><input
								bind:value={form.username}
								autocomplete="off"
							/></label
						>
						<label
							><span>Password</span><input
								type="password"
								bind:value={form.password}
								autocomplete="new-password"
							/></label
						>
						<label
							><span>Authenticator secret (optional)</span><input
								bind:value={form.totp}
								autocomplete="off"
								placeholder="For sites that ask for a 6-digit code"
							/></label
						>
					{:else if addKind === 'api'}
						<label
							><span>Service address</span><input
								bind:value={form.baseUrl}
								placeholder="api.example.com"
								autocomplete="off"
							/></label
						>
						<label
							><span>How the key is sent</span><select bind:value={form.auth}>
								<option value="bearer">Bearer token</option>
								<option value="header">A header</option>
								<option value="query">A query parameter</option>
								<option value="basic">Basic sign-in</option>
							</select></label
						>
						{#if form.auth === 'header' || form.auth === 'query'}
							<label
								><span>{form.auth === 'header' ? 'Header name' : 'Parameter name'}</span><input
									bind:value={form.authName}
									placeholder={form.auth === 'header' ? 'X-Api-Key' : 'api_key'}
								/></label
							>
						{/if}
						<label
							><span>Key or token</span><input
								type="password"
								bind:value={form.secret}
								autocomplete="off"
							/></label
						>
					{:else}
						<p class="hint">
							After you add it, sign in once in your browser. Companies use that session, never the
							password.
						</p>
					{/if}
					<label><span>Name (optional)</span><input bind:value={form.label} /></label>
				</div>
				{#if addKind !== 'api'}
					<label class="personal"
						><input type="checkbox" bind:checked={form.personal} /><span
							>This is a personal login</span
						></label
					>
					{#if form.personal}<p class="notice">{PERSONAL_NOTICE}</p>{/if}
				{/if}
				<div class="add-actions">
					<button class="btn small primary" disabled={busy || !canAdd}
						>{busy ? 'Adding…' : 'Add account'}</button
					>
					<small
						>Saved to your account’s vault. It is never shown again, here or to a company.</small
					>
				</div>
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
	.kinds {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.kinds button {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		min-height: 36px;
		padding: 0 12px 0 6px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
	}
	.kinds button.on {
		border-color: var(--intent-conversation);
		color: var(--ink);
		box-shadow: 0 0 0 1px var(--intent-conversation);
	}
	.kinds button:disabled {
		cursor: default;
		opacity: 0.55;
	}
	.kinds small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
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
	.hint,
	.empty {
		margin: 0;
		color: var(--text-tertiary);
	}
	.fields {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 10px 14px;
	}
	label {
		display: grid;
		gap: 4px;
		min-width: 0;
	}
	label > span {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	label input:not([type='checkbox']),
	label select {
		width: 100%;
		font: inherit;
	}
	.personal,
	label.inline {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	label.inline > span,
	.personal > span {
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	label.inline input[type='time'] {
		width: auto;
	}
	.notice {
		margin: -4px 0 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.add-actions,
	.grant-actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 10px;
	}
	.add-actions small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
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
		padding: 2px 8px;
		border-radius: 99px;
		background: var(--surface-alt);
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
	.grant-editor {
		display: grid;
		gap: 12px;
		padding: 12px;
		background: color-mix(in srgb, var(--surface-alt) 40%, var(--surface-raised));
	}
	.grant-editor + .add-grant {
		display: none;
	}
	.grant-editor-head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 10px;
	}
	.grant-editor-head strong {
		font-weight: 600;
	}
	.presets {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px;
	}
	.presets button {
		min-height: 30px;
		padding: 0 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
	}
	.presets button.on {
		border-color: var(--intent-conversation);
		color: var(--ink);
	}
	.custom {
		margin-left: 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.tiers {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
		gap: 8px 12px;
	}
	.tier > span {
		color: var(--text-secondary);
		font-size: var(--t-body);
		font-weight: 500;
	}
	.cap {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}
	.cap > span {
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.cap .money {
		width: 80px;
	}
	.cap select {
		width: auto;
	}
	.limits {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 10px 14px;
	}
	@media (max-width: 760px) {
		.main {
			grid-template-columns: 36px minmax(0, 1fr) auto;
		}
		.uses,
		.status {
			grid-column: 2 / -1;
		}
		.detail {
			margin-left: 12px;
		}
	}
</style>
