<script lang="ts">
	/* Signing in to a website for a company (S104 O5): the owner signs in on the company computer,
	 * in the card, as they would on their own (codes, captchas, a new-device check, whatever the
	 * site asks). "I'm signed in" carries that session to the account's own browser inside the
	 * plane and checks it took. No password is typed into Restless or kept; signing in to the
	 * computer directly is the one way, so the two never clash. Render it before the card's
	 * InlineDesktop: the computer's "Open … here" leads until the site is open, then this does. */
	import { failureSentence } from '$lib/model/failure';
	import type { Account } from '$lib/model/owner-accounts';
	import { ownerAccounts } from '$lib/model/owner-accounts-client';
	import InlinePanel from './InlinePanel.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		account,
		company,
		onsigned
	}: {
		account: Account;
		company: string;
		onsigned: (account: Account) => void;
	} = $props();

	const site = $derived(account.provider || account.label);
	let failure = $state('');

	async function check() {
		failure = '';
		try {
			const saved = await ownerAccounts.signInFromCompany(account.id, company);
			if (saved.health === 'signed_in') onsigned(saved);
			else
				failure = `It doesn’t look signed in to ${site} on the company computer yet. Sign in there, then try again.`;
		} catch (cause) {
			failure = failureSentence(cause, 'The sign-in could not be checked. Try again.');
		}
	}

	cardActions(() => ({
		primary: { label: 'I’m signed in', busyLabel: 'Checking…', run: check }
	}));
</script>

<InlinePanel
	label={`Sign in to ${site}`}
	note={`Sign in to ${site} on the company computer below, as you would on your own. The company keeps that session; Restless never sees or stores your password.`}
	error={failure}
/>
