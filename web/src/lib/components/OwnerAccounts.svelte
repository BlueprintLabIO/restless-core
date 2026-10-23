<script lang="ts">
	/* Accounts the owner holds beyond AI models (S98-07): website logins, API keys, browser
	 * sign-ins, GitHub and tool connections. One list, one row per account: what it is, which
	 * companies may use it, its health, and Manage. Manage opens the account's credentials and a
	 * grant per company, each edited in place: what the company may do (permissive by default,
	 * the owner's choice), optional limits, and who. Secrets go in and never come back out. */
	import { untrack } from 'svelte';
	import AccountActivity from '$lib/components/AccountActivity.svelte';
	import AccountCompanySignIn from '$lib/components/AccountCompanySignIn.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import EditorForm from './editors/EditorForm.svelte';
	import AccountFieldsEditor from './editors/AccountFieldsEditor.svelte';
	import GrantEditor from './editors/GrantEditor.svelte';
	import { ADDABLE_KINDS } from '$lib/model/account-forms';
	import Segmented from '$lib/ui/page/Segmented.svelte';
	import {
		ACCOUNT_KINDS,
		HEALTH_LABEL,
		grantSummary,
		kindOf,
		limitsSummary,
		type Account,
		type AccountKind,
		type Grant,
		type GrantInput
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
	/* Never a raw id: blank while the list loads, then a plain word for one no longer active. */
	const companyName = (id: string) =>
		companies.find((company) => company.id === id)?.name ??
		(companies.length ? 'An archived company' : '');

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

	/* ---- adding an account: the same editor an Inbox card asking for one shows (S101) ---- */
	const ADDABLE = ADDABLE_KINDS;
	let adding = $state(false);
	let addKind = $state<AccountKind>(ADDABLE_KINDS[0]);
	let addSite = $state('');
	function added(created: Account) {
		accounts = [...accounts, created];
		adding = false;
		addSite = '';
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
	}

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

	/* Back from GitHub (S98-05): a new App's code is kept and the App is installed next; an
	 * installation becomes a GitHub account to give companies. The address is cleaned either way. */
	let fromGithub = false;
	$effect(() => {
		if (fromGithub || !loaded) return;
		const params = new URLSearchParams(location.search);
		const step = params.get('github_app');
		if (!step) return;
		fromGithub = true;
		const clean = new URL(location.href);
		for (const key of ['github_app', 'code', 'state', 'installation_id', 'setup_action'])
			clean.searchParams.delete(key);
		history.replaceState(history.state, '', clean);
		untrack(() => void finishGithub(step, params));
	});
	async function finishGithub(step: string, params: URLSearchParams) {
		const state = params.get('state') ?? undefined;
		try {
			if (step === 'created') {
				const code = params.get('code');
				if (!code || !state) throw new Error('GitHub did not send the App back. Try again.');
				const next = await client.githubAppCreated({ code, state });
				toast('The Restless GitHub App is made. Now choose its repositories.');
				location.assign(next.install_url);
			} else if (step === 'installed') {
				const installation = Number(params.get('installation_id'));
				if (!installation) return;
				const account = await client.githubAppInstalled({ installation_id: installation, state });
				if (!accounts.some((held) => held.id === account.id)) accounts = [...accounts, account];
				managing = account.id;
				toast(`${account.label} connected. Give a company access below.`);
			}
		} catch (cause) {
			toast(failureSentence(cause, 'GitHub was not connected.'), { tone: 'failure' });
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
		const params = new URLSearchParams(location.search);
		const kind = params.get('kind') as AccountKind | null;
		if (!params.get('account') && kind && ADDABLE.includes(kind) && params.get('request')) {
			linked = true;
			untrack(() => {
				adding = true;
				addKind = kind;
				addSite = params.get('site') ?? '';
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

	/* ---- a grant, edited in place with the editor a card giving an account shows (S101) ---- */
	let editingGrant = $state<{ account: string; grant: Grant | null; company: string } | null>(null);
	function editGrant(account: Account, grant: Grant | null, company = '') {
		editingGrant = { account: account.id, grant, company: grant?.company ?? company };
	}
	async function saveGrant(
		account: Account,
		grant: Grant | null,
		company: string,
		input: GrantInput
	) {
		const saved = grant
			? await client.updateGrant(account.id, grant.id, input)
			: await client.grant(account.id, { company, ...input });
		accounts = accounts.map((row) =>
			row.id === account.id
				? {
						...row,
						grants: grant
							? row.grants.map((existing) => (existing.id === saved.id ? saved : existing))
							: [...row.grants, saved]
					}
				: row
		);
		toast(`${companyName(company)} ${grant ? 'access updated' : 'can use it now'}.`);
		editingGrant = null;
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
	{#if editingGrant && editingGrant.account === account.id}
		{@const editing = editingGrant}
		<div class="grant-editor">
			{#key editing.grant?.id ?? `new:${editing.company}`}<EditorForm
					title={editing.grant
						? `${companyName(editing.company)}’s access`
						: 'Give a company access'}
					oncancel={() => (editingGrant = null)}
				>
					<GrantEditor
						grant={editing.grant}
						company={editing.company}
						companies={ungranted(account)}
						action={editing.grant ? 'Save' : 'Give access'}
						onsave={(input, company) => saveGrant(account, editing.grant, company, input)}
					/>
				</EditorForm>{/key}
		</div>
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
						editingGrant = null;
					}}>Add account</button
				>{/if}
		</header>

		{#if adding}
			<EditorForm
				framed
				title="Add account"
				note="Kept in your account’s vault and never shown again."
				oncancel={() => {
					adding = false;
					editingGrant = null;
				}}
			>
				{#snippet head()}<Segmented
						label="Kind of account"
						options={ACCOUNT_KINDS.filter((entry) => ADDABLE.includes(entry.kind)).map((entry) => ({
							value: entry.kind,
							label: entry.label,
							title: entry.hint
						}))}
						bind:value={addKind}
					/>{/snippet}
				<AccountFieldsEditor {client} kind={addKind} site={addSite} onadded={added} />
			</EditorForm>
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
							<div class="state">
								{#if account.open_notices}
									<button
										type="button"
										class="notices"
										title="Uses you were told about and have not answered"
										onclick={() => {
											managing = account.id;
											editingGrant = null;
										}}>{account.open_notices} to look at</button
									>
								{/if}
								<span class="status" class:ok={account.health === 'signed_in'}
									>{account.frozen ? 'Frozen' : HEALTH_LABEL[account.health]}</span
								>
							</div>
							<div class="row-actions">
								<button
									type="button"
									class="btn small"
									aria-expanded={open}
									onclick={() => {
										managing = open ? '' : account.id;
										editingGrant = null;
									}}>Manage</button
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
										<AccountCompanySignIn {client} {account} {companyName} onsigned={replace} />
									</div>
								{/if}
								{#each account.grants as grant (grant.id)}
									{#if editingGrant?.grant?.id === grant.id}
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
								{#if editingGrant?.account === account.id && !editingGrant.grant}
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
						{:else if account.grants.length}
							<!-- Who uses it, one quiet row each under the account, so the account row never grows. -->
							<div class="detail uses" aria-label={`Companies using ${account.label}`}>
								{#each account.grants as grant (grant.id)}
									<div class="use" class:paused={grant.frozen}>
										<strong>{companyName(grant.company)}</strong>
										<span>{grant.frozen ? 'Paused' : grantSummary(grant)}</span>
									</div>
								{/each}
							</div>
						{:else}
							<p class="no-uses">No company has access yet.</p>
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

	.grant-editor {
		margin: 8px 12px 12px;
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
	/* A grid item sizes to its longest unwrapped line unless told it may shrink. */
	.row {
		min-width: 0;
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
	/* The account row is one line high whatever it holds; who uses it sits underneath. */
	.main {
		display: grid;
		grid-template-columns: 36px minmax(0, 1fr) auto auto;
		align-items: center;
		gap: 14px;
		min-height: 60px;
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
	.state {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.use {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 40px;
		padding: 4px 12px;
		box-sizing: border-box;
	}
	.use + .use {
		border-top: 1px solid var(--border);
	}
	.use::before {
		content: '';
		flex: none;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--state-success);
	}
	.use.paused::before {
		background: var(--text-tertiary);
	}
	.use strong {
		margin-right: auto;
		font-weight: 500;
		white-space: nowrap;
	}
	.use span {
		min-width: 0;
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.no-uses {
		margin: -4px 14px 14px 64px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.row-actions .btn[aria-expanded='true'] {
		border-color: var(--control-edge);
		background: var(--surface-alt);
		box-shadow: inset 0 1px 2px color-mix(in srgb, var(--ink) 8%, transparent);
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
		grid-template-columns: minmax(0, 1fr);
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
		/* Narrow: the buttons stay on the account's line, its state goes under the name. */
		.main {
			row-gap: 2px;
		}
		.main .mark {
			grid-row: 1 / span 2;
		}
		.row-actions {
			grid-column: 3;
			grid-row: 1 / span 2;
		}
		.state {
			grid-column: 2;
			grid-row: 2;
			justify-self: start;
		}
		.no-uses {
			margin-left: 64px;
		}
		.detail {
			margin-left: 12px;
		}
	}
</style>
