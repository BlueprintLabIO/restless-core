<script lang="ts">
	/* A website sign-in, kept as a session (S104 O5): whether it is signed in, and a way to sign in
	 * again on the computer of a company that uses it. The owner signs in there as on their own
	 * browser; "Check sign-in" carries that session to the account's browser inside the plane. No
	 * password is asked for or kept. */
	import { goto } from '$app/navigation';
	import { failureSentence } from '$lib/model/failure';
	import { prepareCompanyBrowser } from '$lib/model/company-browser';
	import { HEALTH_LABEL, type Account } from '$lib/model/owner-accounts';
	import type { AccountsClient } from '$lib/model/owner-accounts-client';

	let {
		client,
		account,
		companyName,
		onsigned
	}: {
		client: AccountsClient;
		account: Account;
		companyName: (id: string) => string;
		onsigned: (account: Account) => void;
	} = $props();

	const site = $derived(account.provider || account.label);
	const companies = $derived([...new Set(account.grants.map((grant) => grant.company))]);
	let company = $state('');
	$effect(() => {
		if (!companies.includes(company)) company = companies[0] ?? '';
	});
	let busy = $state(false);
	let failure = $state('');

	function signIn() {
		if (!company) return;
		prepareCompanyBrowser(company, `https://${site}`);
		void goto(`/${encodeURIComponent(company)}/company/computer?focus=browser`);
	}

	async function check() {
		if (!company || busy) return;
		busy = true;
		failure = '';
		try {
			const saved = await client.signInFromCompany(account.id, company);
			onsigned(saved);
			if (saved.health !== 'signed_in')
				failure = `Not signed in to ${site} on ${companyName(company)}’s computer yet.`;
		} catch (cause) {
			failure = failureSentence(cause, 'The sign-in could not be checked. Try again.');
		} finally {
			busy = false;
		}
	}
</script>

<div class="company-sign-in">
	<p>
		<strong>{HEALTH_LABEL[account.health]}</strong>
		{#if companies.length}
			· You sign in to {site} on the company computer, as on your own browser.
		{:else}
			· Give it to a company, then sign in to {site} on that company’s computer.
		{/if}
	</p>
	{#if companies.length}
		<div class="actions">
			{#if companies.length > 1}<select bind:value={company} aria-label="Which company’s computer">
					{#each companies as id (id)}<option value={id}>{companyName(id)}</option>{/each}
				</select>{/if}
			<button type="button" class="btn small" onclick={signIn}
				>Sign in on {companies.length > 1 ? 'its' : `${companyName(company)}’s`} computer</button
			>
			<button type="button" class="btn small ghost" disabled={busy} onclick={() => void check()}
				>{busy ? 'Checking…' : 'Check sign-in'}</button
			>
		</div>
	{/if}
	{#if failure}<p class="failure" role="alert">{failure}</p>{/if}
</div>

<style>
	.company-sign-in {
		display: grid;
		gap: 8px;
		min-width: 0;
	}
	p {
		margin: 0;
		color: var(--text-secondary);
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
	}
	.failure {
		color: var(--state-danger);
	}
</style>
