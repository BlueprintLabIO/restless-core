<script lang="ts">
	/* An agent asked for an app (S101), answered in its card: choose an account you already hold,
	 * or a new one, by token or by the provider's own sign-in in a small window the card watches.
	 * The footer gives it. When it works the request resolves and the Work resumes. Only the owner
	 * adds apps; anyone else sees who it is waiting on. */
	import { onDestroy } from 'svelte';
	import { CATALOGUE } from '$lib/model/apps';
	import type { AppRequest } from '$lib/model/app-requests';
	import { failureSentence } from '$lib/model/failure';
	import {
		addTokenApp,
		finishAppSignIn,
		grantHeldAccount,
		matchingAccounts,
		popupFeatures,
		startAppSignIn,
		tokenHelp
	} from '$lib/model/inline-credentials';
	import type { Account } from '$lib/model/owner-accounts';
	import { ownerAccounts } from '$lib/model/owner-accounts-client';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import { cardActions, type CardAction } from './card-actions.svelte';

	let {
		companyId,
		request,
		owner,
		ondone
	}: {
		companyId: string;
		request: AppRequest;
		/** Only the owner adds apps and gives accounts. */
		owner: boolean;
		/** The request is answered: the card shows the receipt and moves on. */
		ondone: (receipt: string) => void;
	} = $props();

	const NEW = '__new';
	const entry = $derived(CATALOGUE.find((candidate) => candidate.key === request.catalogueKey));
	const usesToken = $derived(entry?.auth === 'token');
	const help = $derived(tokenHelp(request.catalogueKey ?? ''));

	let held = $state<Account[]>([]);
	let choice = $state<string | null>(NEW);
	let token = $state('');
	let showToken = $state(false);
	let error = $state('');
	let waiting = $state<{ name: string; window: Window | null } | null>(null);
	let timer: ReturnType<typeof setInterval> | undefined;
	const given = (account: Account) => account.grants.some((grant) => grant.company === companyId);
	const chosen = $derived(held.find((account) => account.id === choice) ?? null);

	$effect(() => {
		if (!owner) return;
		void ownerAccounts
			.list()
			.then((accounts) => {
				held = matchingAccounts(accounts, request.catalogueKey ?? request.app, companyId);
				const first = held.find((account) => !given(account));
				if (first) choice = first.id;
			})
			.catch(() => (held = []));
	});

	async function attempt(work: () => Promise<string | null>) {
		error = '';
		try {
			const receipt = await work();
			if (receipt) ondone(receipt);
		} catch (cause) {
			error = failureSentence(cause, `${request.name} was not added. Try again.`);
		}
	}

	function signInWithProvider() {
		/* Opened now, inside the click, so no popup blocker stops it; pointed at the provider once
		 * its address is known. */
		const popup = window.open('about:blank', 'restless-sign-in', popupFeatures());
		return attempt(async () => {
			const { connection, url } = await startAppSignIn(companyId, request);
			if (!url) {
				popup?.close();
				return (await finishAppSignIn(companyId, connection.name))
					? `${request.name} is connected. ${request.asker} carries on.`
					: null;
			}
			if (popup) popup.location.href = url;
			else window.open(url, 'restless-sign-in', popupFeatures());
			waiting = { name: connection.name, window: popup };
			clearInterval(timer);
			timer = setInterval(() => void check(), 2_000);
			return null;
		});
	}

	async function check() {
		if (!waiting) return;
		if (!(await finishAppSignIn(companyId, waiting.name).catch(() => false))) return;
		clearInterval(timer);
		waiting.window?.close();
		waiting = null;
		ondone(`${request.name} is connected. ${request.asker} carries on.`);
	}

	onDestroy(() => clearInterval(timer));

	cardActions(() => {
		if (!owner) return {};
		let primary: CardAction;
		if (chosen) {
			const account = chosen;
			primary = {
				label: `Give ${account.label}`,
				busyLabel: 'Giving…',
				run: () =>
					attempt(async () => {
						await grantHeldAccount(companyId, account);
						return `Gave the company ${account.label}${account.identity.display ? ` (${account.identity.display})` : ''}. ${request.asker} carries on.`;
					})
			};
		} else if (usesToken) {
			primary = {
				label: 'Save token and give it',
				busyLabel: 'Saving…',
				disabled: !token.trim(),
				run: () =>
					attempt(async () => {
						const { label } = await addTokenApp(companyId, request, token);
						token = '';
						return `${label} added with your token and given to the company. ${request.asker} carries on.`;
					})
			};
		} else {
			primary = {
				label: waiting ? 'Open the sign-in again' : `Sign in with ${request.name}`,
				busyLabel: 'Opening…',
				run: signInWithProvider
			};
		}
		return { primary };
	});
</script>

<InlinePanel
	label={`Add ${request.name}`}
	{error}
	note={!owner
		? `Waiting for the owner to add ${request.name}.`
		: chosen
			? ''
			: usesToken
				? 'It goes straight to your vault and is never shown again.'
				: waiting
					? `Finish signing in to ${request.name} in the window that opened. This card completes by itself.`
					: `${request.name}'s own sign-in opens in a small window. Anything that acts asks you first.`}
>
	{#if owner}
		{#if held.length}
			<InlineChoice
				label={`Which ${request.name} account`}
				options={[
					...held.map((account) => ({
						value: account.id,
						title: account.label,
						detail: account.identity.display ?? undefined,
						mark: given(account) ? 'Already given' : undefined,
						disabled: given(account)
					})),
					{
						value: NEW,
						title: usesToken ? 'A new token' : `Another ${request.name} account`,
						detail: usesToken ? 'Paste it below' : 'Sign in with the provider'
					}
				]}
				bind:value={choice}
			/>
		{/if}
		{#if usesToken && !chosen}
			<div class="token-row">
				<input
					type={showToken ? 'text' : 'password'}
					autocomplete="off"
					spellcheck="false"
					aria-label={`${request.name} token`}
					placeholder={request.catalogueKey === 'github' ? 'github_pat_…' : `${request.name} token`}
					bind:value={token}
				/>
				<button
					class="btn small ghost"
					type="button"
					aria-pressed={showToken}
					aria-label={showToken ? 'Hide the token' : 'Show the token'}
					onclick={() => (showToken = !showToken)}>{showToken ? 'Hide' : 'Show'}</button
				>
			</div>
			{#if help}<p class="help">{help.text} <a href={help.href} target="_blank" rel="noreferrer">Make one ↗</a></p>{/if}
		{/if}
	{/if}
</InlinePanel>

<style>
	.token-row {
		display: flex;
		gap: 6px;
		min-width: 0;
	}
	.token-row input {
		flex: 1;
		min-width: 0;
		font-family: var(--font-mono, monospace);
	}
	.help {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
</style>
