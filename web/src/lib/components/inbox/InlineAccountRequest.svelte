<script lang="ts">
	/* A company asks for an account (S98-08), answered in its card (S101) the way Settings › Accounts
	 * adds and gives one: which account (one you hold, or a new one added with Add account's own
	 * editor), then what the company may do with it, in the same access editor. The footer gives it,
	 * which settles the request. A website login is a sign-in the owner makes on the company
	 * computer, in the card (S104 O5): never a password form. Only the owner gives accounts; anyone
	 * else sees it waits on the owner. */
	import Field from '$lib/ui/forms/Field.svelte';
	import { domainOf, matchingAccounts, type AccountAsk } from '$lib/model/inline-credentials';
	import { ADDABLE_KINDS } from '$lib/model/account-forms';
	import {
		kindPhrase,
		type Account,
		type AccountKind,
		type GrantInput
	} from '$lib/model/owner-accounts';
	import { ownerAccounts } from '$lib/model/owner-accounts-client';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import InlineDesktop from './InlineDesktop.svelte';
	import InlineSiteSignIn from './InlineSiteSignIn.svelte';
	import AccountFieldsEditor from '../editors/AccountFieldsEditor.svelte';
	import GrantEditor from '../editors/GrantEditor.svelte';

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
	const BROWSER: AccountKind[] = ['browser_profile', 'website_login'];
	/* An older ask for a website login (a password) is answered as a sign-in like any other. */
	const kind = $derived(
		(ask.kind === 'website_login' ? 'browser_profile' : (ask.kind ?? '')) as AccountKind
	);
	const addable = $derived(!ask.account_id && ADDABLE_KINDS.includes(kind));
	let held = $state<Account[]>([]);
	let account = $state<string | null>(null);
	let pendingSignIn = $state<Account | null>(null);
	let loaded = $state(false);

	$effect(() => {
		if (!owner) return;
		void ownerAccounts
			.list()
			.then((accounts) => {
				held = ask.account_id
					? accounts.filter((row) => row.id === ask.account_id)
					: matchingAccounts(
							accounts,
							kind === 'github' ? 'github' : domainOf(ask.site ?? '') || kind,
							ask.company
						).filter(
							(row) =>
								!ask.kind ||
								row.kind === kind ||
								(BROWSER.includes(kind) && BROWSER.includes(row.kind))
						);
				account = held[0]?.id ?? (addable ? NEW : null);
			})
			.catch(() => {
				held = [];
				account = addable ? NEW : null;
			})
			.finally(() => (loaded = true));
	});

	const chosen = $derived(held.find((row) => row.id === account) ?? null);

	/* A new account joins the list and is chosen, so the next step is giving it. */
	function added(created: Account) {
		held = [...held, created];
		account = created.id;
		if (created.kind === 'browser_profile') pendingSignIn = created;
	}

	async function give(row: Account, input: GrantInput) {
		const grant = await ownerAccounts.grant(row.id, { company: ask.company, ...input });
		await ownerAccounts.settleRequest(ask.id, { decision: 'approved', grant_id: grant.id });
		ondone(`Gave the company ${row.label}. ${ask.requested_by_display} carries on.`);
	}
</script>

{#if !owner}
	<InlinePanel
		label="Account request"
		note={`Waiting for the owner to give ${kindPhrase((kind || 'api') as AccountKind)}.`}
	/>
{:else if pendingSignIn}
	{@const signing = pendingSignIn}
	{#key signing.id}<InlineSiteSignIn
			account={signing}
			company={ask.company}
			onsigned={(saved) => {
				held = held.map((row) => (row.id === saved.id ? saved : row));
				pendingSignIn = null;
			}}
		/><InlineDesktop
			companyId={ask.company}
			itemId="runtime-rescue"
			attached={false}
			url={`https://${signing.provider || ask.site || ''}`}
		/>{/key}
{:else if !loaded}
	<InlinePanel label="Answer the account request" note="Looking for the accounts you hold…" />
{:else if !held.length && !addable}
	<InlinePanel
		label="Answer the account request"
		note="This kind of account is set up with its provider in Settings › Accounts."
	/>
{:else}
	<InlinePanel label="Answer the account request" form>
		{#if held.length > 1 || (held.length && addable)}
			<Field label="Which account">
				<InlineChoice
					label="Which account"
					options={[
						...held.map((row) => ({
							value: row.id,
							title: row.label,
							detail: row.identity.display ?? undefined
						})),
						...(addable ? [{ value: NEW, title: 'A new one', detail: 'Add it here' }] : [])
					]}
					bind:value={account}
				/>
			</Field>
		{/if}
		{#if account === NEW && addable}
			<AccountFieldsEditor
				client={ownerAccounts}
				{kind}
				site={ask.site ?? ''}
				action={kind === 'browser_profile' ? 'Continue to sign in' : 'Add account'}
				onadded={added}
			/>
		{:else if chosen}
			{@const row = chosen}
			{#key row.id}<GrantEditor
					company={ask.company}
					abilities={ask.abilities}
					action={`Give ${row.label}`}
					onsave={(input) => give(row, input)}
				/>{/key}
		{/if}
	</InlinePanel>
{/if}
