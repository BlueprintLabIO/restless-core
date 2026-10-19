<script lang="ts">
	/* An agent asked for an app (S101): answered inside its Inbox card. Give the company an account
	 * you already hold in one click, paste a token, or sign in with the provider in a small window
	 * the card watches. When it works the request resolves and the Work resumes; nothing here sends
	 * you to another page. Only the owner adds apps; anyone else sees who it is waiting on. */
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

	const entry = $derived(CATALOGUE.find((candidate) => candidate.key === request.catalogueKey));
	const usesToken = $derived(entry?.auth === 'token');
	const help = $derived(tokenHelp(request.catalogueKey ?? ''));

	let held = $state<Account[]>([]);
	let token = $state('');
	let showToken = $state(false);
	let busy = $state('');
	let error = $state('');
	let waiting = $state<{ name: string; window: Window | null } | null>(null);
	let timer: ReturnType<typeof setInterval> | undefined;

	$effect(() => {
		if (!owner) return;
		void ownerAccounts
			.list()
			.then((accounts) => (held = matchingAccounts(accounts, request.catalogueKey ?? request.app, companyId)))
			.catch(() => (held = []));
	});

	async function act(label: string, work: () => Promise<string | null>) {
		if (busy) return;
		busy = label;
		error = '';
		try {
			const receipt = await work();
			if (receipt) ondone(receipt);
		} catch (cause) {
			error = failureSentence(cause, `${request.name} was not added. Try again.`);
		} finally {
			busy = '';
		}
	}

	const useHeld = (account: Account) =>
		act(account.id, async () => {
			await grantHeldAccount(companyId, account);
			return `Gave the company ${account.label}${account.identity.display ? ` (${account.identity.display})` : ''}. ${request.asker} carries on.`;
		});

	const saveToken = (event: SubmitEvent) => {
		event.preventDefault();
		void act('token', async () => {
			const { label } = await addTokenApp(companyId, request, token);
			token = '';
			return `${label} added with your token and given to the company. ${request.asker} carries on.`;
		});
	};

	function signInWithProvider() {
		/* Opened now, inside the click, so no popup blocker stops it; pointed at the provider once
		 * its address is known. */
		const popup = window.open('about:blank', 'restless-sign-in', popupFeatures());
		void act('sign-in', async () => {
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
		const done = await finishAppSignIn(companyId, waiting.name).catch(() => false);
		if (done) {
			clearInterval(timer);
			waiting.window?.close();
			waiting = null;
			ondone(`${request.name} is connected. ${request.asker} carries on.`);
		}
	}

	onDestroy(() => clearInterval(timer));
</script>

<section class="inline-credential" aria-label={`Add ${request.name}`}>
	{#if !owner}
		<p class="waiting-owner">Waiting for the owner to add {request.name}.</p>
	{:else}
		{#if held.length}
			<div class="held" role="list" aria-label="Accounts you already hold">
				{#each held as account (account.id)}
					{@const given = account.grants.some((grant) => grant.company === companyId)}
					<div class="held-row" role="listitem">
						<span class="mark" aria-hidden="true">{account.label.slice(0, 2).toUpperCase()}</span>
						<span class="who">
							<strong>{account.label}</strong>
							{#if account.identity.display}<small>{account.identity.display}</small>{/if}
						</span>
						<button
							class="btn small"
							class:primary={!given}
							type="button"
							disabled={!!busy || given}
							onclick={() => void useHeld(account)}
							>{given ? 'Already given' : busy === account.id ? 'Giving…' : 'Give access'}</button
						>
					</div>
				{/each}
			</div>
		{/if}

		{#if usesToken}
			<form class="token" onsubmit={saveToken}>
				<label for={`token-${request.handoff_id}`}
					>{held.length ? 'Or paste a new token' : `Paste a ${request.name} token`}</label
				>
				<div class="token-row">
					<input
						id={`token-${request.handoff_id}`}
						type={showToken ? 'text' : 'password'}
						autocomplete="off"
						spellcheck="false"
						placeholder={request.catalogueKey === 'github' ? 'github_pat_…' : 'Token'}
						bind:value={token}
						disabled={!!busy}
					/>
					<button
						class="reveal"
						type="button"
						aria-pressed={showToken}
						aria-label={showToken ? 'Hide the token' : 'Show the token'}
						onclick={() => (showToken = !showToken)}>{showToken ? 'Hide' : 'Show'}</button
					>
					<button class="btn small primary" disabled={!!busy || !token.trim()}
						>{busy === 'token' ? 'Saving…' : 'Save and continue'}</button
					>
				</div>
				<p class="hint">
					{#if help}{help.text}
						<a href={help.href} target="_blank" rel="noreferrer">Make one ↗</a>{/if}
					It goes straight to your vault and is never shown again.
				</p>
			</form>
		{:else}
			<div class="sign-in">
				{#if waiting}
					<p class="pending" role="status">
						<span class="pulse" aria-hidden="true"></span>Finish signing in to {request.name} in the
						window that opened. This card completes by itself.
					</p>
					<button class="btn small" type="button" onclick={signInWithProvider}>Open it again</button>
				{:else}
					<button class="btn small primary" type="button" disabled={!!busy} onclick={signInWithProvider}
						>{busy === 'sign-in' ? 'Opening…' : `Sign in with ${request.name}`}</button
					>
					<p class="hint">
						{request.name}'s own sign-in opens in a small window. Anything that acts asks you first.
					</p>
				{/if}
			</div>
		{/if}
		{#if error}<p class="error" role="alert">{error}</p>{/if}
	{/if}
</section>

<style>
	.inline-credential {
		display: grid;
		gap: 12px;
		margin-top: 12px;
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
	}
	.held {
		display: grid;
		gap: 6px;
	}
	.held-row {
		display: grid;
		grid-template-columns: 28px minmax(0, 1fr) auto;
		align-items: center;
		gap: 10px;
	}
	.mark {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		border-radius: 7px;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.who {
		display: grid;
		min-width: 0;
	}
	.who strong {
		font-weight: 500;
	}
	.who small,
	.hint,
	.waiting-owner {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.hint,
	.waiting-owner,
	.pending,
	.error {
		margin: 0;
	}
	.token {
		display: grid;
		gap: 6px;
	}
	.token label {
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.token-row {
		display: flex;
		gap: 6px;
		min-width: 0;
	}
	.token-row input {
		flex: 1;
		min-width: 0;
		font-family: var(--font-mono, ui-monospace, monospace);
	}
	.reveal {
		padding: 0 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: none;
		color: var(--text-secondary);
		font-size: var(--t-label);
		cursor: pointer;
	}
	.sign-in {
		display: grid;
		justify-items: start;
		gap: 6px;
	}
	.pending {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--text-secondary);
	}
	.pulse {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: var(--state-success);
		animation: pulse 1.2s ease-in-out infinite;
	}
	@keyframes pulse {
		50% {
			opacity: 0.3;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.pulse {
			animation: none;
		}
	}
	.error {
		color: var(--state-danger);
		font-size: var(--t-label);
	}
	@media (max-width: 520px) {
		.token-row {
			flex-wrap: wrap;
		}
		.token-row input {
			flex-basis: 100%;
		}
	}
</style>
