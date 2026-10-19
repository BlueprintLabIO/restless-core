<script lang="ts">
	/* A company asks for an account (S98-08), answered in its card (S101): which account (one you
	 * hold, or a new one filled in here) and what the company may do with it. The footer gives it,
	 * which settles the request. A browser sign-in continues on the live desktop in the card. Only
	 * the owner gives accounts; anyone else sees it waits on the owner. */
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
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		ask,
		owner,
		ondone
	}: {
		ask: AccountAsk;
		owner: boolean;
		ondone: (receipt: string) => void;
	} = $props();

	const NEW = '__new';
	const kind = $derived(ask.kind ?? '');
	const addable = $derived(!ask.account_id && ['website_login', 'browser_profile', 'github', 'api'].includes(kind));
	let held = $state<Account[]>([]);
	let account = $state<string | null>(null);
	let scope = $state<'asked' | 'read' | 'full'>('asked');
	let fields = $state<NewAccountFields>({ site: '', label: '', username: '', password: '', totp: '', secret: '', baseUrl: '' });
	let pendingSignIn = $state<Account | null>(null);
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
					? accounts.filter((row) => row.id === ask.account_id)
					: matchingAccounts(accounts, kind === 'github' ? 'github' : (domainOf(ask.site ?? '') || kind), ask.company).filter(
							(row) => !ask.kind || row.kind === ask.kind
						);
				account = held[0]?.id ?? (addable ? NEW : null);
			})
			.catch(() => {
				held = [];
				account = addable ? NEW : null;
			});
	});

	const abilities = $derived<Abilities>(
		scope === 'asked'
			? ask.abilities
			: (PRESETS.find((preset) => preset.key === (scope === 'read' ? 'read' : 'full'))?.abilities ?? ask.abilities)
	);
	const summary = $derived(grantSummary({ abilities, spend_cap: null }));
	const missing = $derived(missingFor(kind, fields));
	const help = $derived(kind === 'github' ? tokenHelp('github') : null);
	const chosen = $derived(held.find((row) => row.id === account) ?? null);

	async function attempt(work: () => Promise<string | null>) {
		error = '';
		try {
			const receipt = await work();
			if (receipt) ondone(receipt);
		} catch (cause) {
			error = failureSentence(cause, 'That did not save. Try again.');
		}
	}

	const signedIn = (row: Account) =>
		attempt(async () => {
			await grantForRequest(ask, row.id, abilities);
			pendingSignIn = null;
			return `Signed in to ${row.label} and gave it to the company. ${ask.requested_by_display} carries on.`;
		});

	cardActions(() => {
		if (!owner || pendingSignIn) return {};
		if (chosen) {
			const row = chosen;
			return {
				primary: {
					label: `Give ${row.label}`,
					busyLabel: 'Giving…',
					run: () =>
						attempt(async () => {
							await grantForRequest(ask, row.id, abilities);
							return `Gave the company ${row.label} (${summary}). ${ask.requested_by_display} carries on.`;
						})
				}
			};
		}
		if (account === NEW && addable)
			return {
				primary: {
					label: kind === 'browser_profile' ? 'Continue to sign in' : 'Add and give it',
					busyLabel: 'Adding…',
					run: () => {
						tried = true;
						if (missing) return;
						return attempt(async () => {
							const created = await createRequestedAccount(kind, fields);
							fields.password = '';
							fields.secret = '';
							fields.totp = '';
							if (kind === 'browser_profile') {
								pendingSignIn = created;
								return null;
							}
							await grantForRequest(ask, created.id, abilities);
							return `${created.label} added and given to the company. ${ask.requested_by_display} carries on.`;
						});
					}
				}
			};
		return {};
	});
</script>

{#if !owner}
	<InlinePanel label="Account request" note={`Waiting for the owner to give ${kindPhrase((kind || 'api') as AccountKind)}.`} />
{:else if pendingSignIn}
	<InlinePanel label={`Sign in to ${pendingSignIn.label}`} {error} note="Sign in on the browser here. Nobody else can type while it is open; the card finishes when you confirm you are signed in.">
		<AccountSignIn client={ownerAccounts} account={pendingSignIn} onsigned={(row) => void signedIn(row)} />
	</InlinePanel>
{:else}
	<InlinePanel
		label="Answer the account request"
		{error}
		note={account === NEW
			? tried && missing
				? missing
				: kind === 'browser_profile'
					? 'Next, you sign in on the live browser in this card.'
					: 'Secrets go straight to your vault and are never shown again.'
			: !held.length && !addable
				? 'This kind of account is set up with its provider in Settings › Accounts.'
				: ''}
	>
		{#if held.length > 1 || (held.length && addable)}
			<InlineChoice
				label="Which account"
				options={[
					...held.map((row) => ({ value: row.id, title: row.label, detail: row.identity.display ?? undefined })),
					...(addable ? [{ value: NEW, title: 'A new one', detail: 'Fill it in below' }] : [])
				]}
				bind:value={account}
			/>
		{/if}
		{#if account === NEW && addable}
			{#if kind === 'website_login' || kind === 'browser_profile'}
				<label class="field"
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
					<label class="field">Username or email<input bind:value={fields.username} autocomplete="off" /></label>
					<label class="field">Password<input type="password" bind:value={fields.password} autocomplete="new-password" /></label>
				</div>
				<label class="field"
					><span>Authenticator secret <small>optional</small></span><input
						bind:value={fields.totp}
						autocomplete="off"
						placeholder="Lets Restless answer two-step codes itself"
					/></label
				>
			{:else if kind === 'api'}
				<div class="pair">
					<label class="field">API address<input bind:value={fields.baseUrl} placeholder="api.example.com" autocomplete="off" /></label>
					<label class="field">Key<input type="password" bind:value={fields.secret} autocomplete="off" /></label>
				</div>
			{:else if kind === 'github'}
				<label class="field"
					><span>Token{#if help}<small> · {help.text} <a href={help.href} target="_blank" rel="noreferrer">Make one ↗</a></small>{/if}</span
					><input type="password" bind:value={fields.secret} autocomplete="off" placeholder="github_pat_…" /></label
				>
			{/if}
		{/if}
		{#if held.length || addable}
			<label class="field"
				><span>It may <small>{summary}</small></span>
				<select bind:value={scope}>
					<option value="asked">What {ask.requested_by_display} asked for</option>
					<option value="read">Only read</option>
					<option value="full">Full access</option>
				</select></label
			>
		{/if}
	</InlinePanel>
{/if}
