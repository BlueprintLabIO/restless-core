<script lang="ts">
	/* A company asks for an account (S98-08), answered inside its Inbox card (S101). Give one you
	 * already hold, or add it right here: a website login, a browser sign-in on the live desktop,
	 * an API key or a GitHub token. Choose what the company may do with it, and the request is
	 * answered. Only the owner gives accounts; anyone else sees it waits on the owner. */
	import AccountSignIn from '$lib/components/AccountSignIn.svelte';
	import { failureSentence } from '$lib/model/failure';
	import {
		createRequestedAccount,
		domainOf,
		grantForRequest,
		matchingAccounts,
		missingFor,
		tokenHelp,
		type AccountAsk,
		type NewAccountFields
	} from '$lib/model/inline-credentials';
	import { PRESETS, grantSummary, kindPhrase, type Abilities, type Account, type AccountKind } from '$lib/model/owner-accounts';
	import { ownerAccounts } from '$lib/model/owner-accounts-client';

	let {
		ask,
		owner,
		ondone
	}: {
		ask: AccountAsk;
		owner: boolean;
		ondone: (receipt: string) => void;
	} = $props();

	const kind = $derived(ask.kind ?? '');
	const inlineKinds = ['website_login', 'browser_profile', 'github', 'api'];
	let held = $state<Account[]>([]);
	let choice = $state<'asked' | 'read' | 'full'>('asked');
	let fields = $state<NewAccountFields>({ site: '', label: '', username: '', password: '', totp: '', secret: '', baseUrl: '' });
	let pendingSignIn = $state<Account | null>(null);
	let busy = $state('');
	let error = $state('');
	let tried = $state(false);

	$effect(() => {
		fields.site = ask.site ?? '';
	});
	$effect(() => {
		if (!owner) return;
		void ownerAccounts
			.list()
			.then((accounts) => {
				held = ask.account_id
					? accounts.filter((account) => account.id === ask.account_id)
					: matchingAccounts(accounts, kind === 'github' ? 'github' : (domainOf(ask.site ?? '') || kind), ask.company).filter(
							(account) => !ask.kind || account.kind === ask.kind
						);
			})
			.catch(() => (held = []));
	});

	const abilities = $derived<Abilities>(
		choice === 'asked'
			? ask.abilities
			: (PRESETS.find((preset) => preset.key === (choice === 'read' ? 'read' : 'full'))?.abilities ?? ask.abilities)
	);
	const missing = $derived(missingFor(kind, fields));
	const help = $derived(kind === 'github' ? tokenHelp('github') : null);

	async function act(label: string, work: () => Promise<string | null>) {
		if (busy) return;
		busy = label;
		error = '';
		try {
			const receipt = await work();
			if (receipt) ondone(receipt);
		} catch (cause) {
			error = failureSentence(cause, 'That did not save. Try again.');
		} finally {
			busy = '';
		}
	}

	const give = (account: Account) =>
		act(account.id, async () => {
			await grantForRequest(ask, account.id, abilities);
			return `Gave the company ${account.label} (${grantSummary({ abilities, spend_cap: null })}). ${ask.requested_by_display} carries on.`;
		});

	function add(event: SubmitEvent) {
		event.preventDefault();
		tried = true;
		if (missing) return;
		void act('add', async () => {
			const account = await createRequestedAccount(kind, fields);
			fields.password = '';
			fields.secret = '';
			fields.totp = '';
			if (kind === 'browser_profile') {
				pendingSignIn = account;
				return null;
			}
			await grantForRequest(ask, account.id, abilities);
			return `${account.label} added and given to the company. ${ask.requested_by_display} carries on.`;
		});
	}

	const signedIn = (account: Account) =>
		act('signed', async () => {
			await grantForRequest(ask, account.id, abilities);
			pendingSignIn = null;
			return `Signed in to ${account.label} and gave it to the company. ${ask.requested_by_display} carries on.`;
		});
</script>

<section class="inline-account" aria-label="Answer the account request">
	{#if !owner}
		<p class="quiet">Waiting for the owner to give {kindPhrase((kind || 'api') as AccountKind)}.</p>
	{:else if pendingSignIn}
		<p class="step">Sign in to {pendingSignIn.label} below, then press "I'm signed in". Nobody else can type while it is open.</p>
		<AccountSignIn client={ownerAccounts} account={pendingSignIn} onsigned={(account) => void signedIn(account)} />
	{:else}
		<div class="abilities" role="radiogroup" aria-label="What the company may do with it">
			<span class="label">May</span>
			{#each [['asked', 'As asked'], ['read', 'Only read'], ['full', 'Full access']] as [value, text] (value)}
				<button
					type="button"
					role="radio"
					aria-checked={choice === value}
					class:on={choice === value}
					onclick={() => (choice = value as typeof choice)}>{text}</button
				>
			{/each}
			<span class="summary">{grantSummary({ abilities, spend_cap: null })}</span>
		</div>

		{#if held.length}
			<div class="held" role="list" aria-label="Accounts you hold">
				{#each held as account (account.id)}
					<div class="held-row" role="listitem">
						<span class="who"
							><strong>{account.label}</strong>{#if account.identity.display}<small>{account.identity.display}</small>{/if}</span
						>
						<button class="btn small primary" type="button" disabled={!!busy} onclick={() => void give(account)}
							>{busy === account.id ? 'Giving…' : 'Give access'}</button
						>
					</div>
				{/each}
			</div>
		{/if}

		{#if !ask.account_id && inlineKinds.includes(kind)}
			<form class="add" onsubmit={add} novalidate>
				<p class="label">{held.length ? 'Or add a new one' : `Add ${kindPhrase(kind as AccountKind)}`}</p>
				{#if kind === 'website_login' || kind === 'browser_profile'}
					<label
						>Site<input
							bind:value={fields.site}
							placeholder="ebay.com.au/signin"
							autocomplete="off"
							aria-invalid={tried && !domainOf(fields.site)}
						/></label
					>
				{/if}
				{#if kind === 'website_login'}
					<div class="pair">
						<label>Username or email<input bind:value={fields.username} autocomplete="off" /></label>
						<label>Password<input type="password" bind:value={fields.password} autocomplete="new-password" /></label>
					</div>
					<label
						>Authenticator secret <small>(optional)</small><input
							bind:value={fields.totp}
							autocomplete="off"
							placeholder="Lets Restless answer two-step codes itself"
						/></label
					>
				{:else if kind === 'api'}
					<div class="pair">
						<label>API address<input bind:value={fields.baseUrl} placeholder="api.example.com" autocomplete="off" /></label>
						<label>Key<input type="password" bind:value={fields.secret} autocomplete="off" /></label>
					</div>
				{:else if kind === 'github'}
					<label>Token<input type="password" bind:value={fields.secret} autocomplete="off" placeholder="github_pat_…" /></label>
					{#if help}<p class="quiet">
							{help.text} <a href={help.href} target="_blank" rel="noreferrer">Make one ↗</a>
						</p>{/if}
				{/if}
				<div class="foot">
					<span class="quiet"
						>{tried && missing
							? missing
							: kind === 'browser_profile'
								? 'Next, you sign in on the live browser here.'
								: 'Secrets go straight to your vault and are never shown again.'}</span
					>
					<button class="btn small primary" disabled={!!busy}
						>{busy === 'add' ? 'Adding…' : kind === 'browser_profile' ? 'Open the browser' : 'Add and give access'}</button
					>
				</div>
			</form>
		{:else if !ask.account_id && !held.length}
			<p class="quiet">This kind of account is added in Settings › Accounts, where it is set up with its provider.</p>
		{/if}
		{#if error}<p class="error" role="alert">{error}</p>{/if}
	{/if}
</section>

<style>
	.inline-account {
		display: grid;
		gap: 12px;
		margin-top: 12px;
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
	}
	.abilities {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
	}
	.abilities button {
		padding: 3px 10px;
		border: 1px solid var(--border);
		border-radius: 99px;
		background: none;
		color: var(--text-secondary);
		font-size: var(--t-label);
		cursor: pointer;
	}
	.abilities button.on {
		border-color: var(--text-secondary);
		color: var(--ink);
	}
	.summary,
	.quiet,
	.label,
	small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.quiet,
	.label,
	.step,
	.error {
		margin: 0;
	}
	.held {
		display: grid;
		gap: 6px;
	}
	.held-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}
	.who {
		display: grid;
		min-width: 0;
	}
	.who strong {
		font-weight: 500;
	}
	.add {
		display: grid;
		gap: 8px;
	}
	.add label {
		display: grid;
		gap: 3px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.add input {
		min-width: 0;
	}
	.pair {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}
	.foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}
	.error {
		color: var(--state-danger);
		font-size: var(--t-label);
	}
	@media (max-width: 520px) {
		.pair {
			grid-template-columns: 1fr;
		}
		.foot {
			flex-wrap: wrap;
		}
	}
</style>
