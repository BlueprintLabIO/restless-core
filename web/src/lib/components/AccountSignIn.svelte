<script lang="ts">
	/* Signing in to a browser account by hand (S98-11): the account's own managed browser opens
	 * here at its sign-in page, the owner signs in (passwords, codes, a captcha, whatever the site
	 * asks), then says "I'm signed in" and the profile is kept for the companies granted it. The
	 * owner holds the only keyboard while this is open. Says so plainly if the owner plane cannot
	 * do it yet. */
	import { onDestroy } from 'svelte';
	import DesktopViewport from '$lib/components/DesktopViewport.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import type { Account } from '$lib/model/owner-accounts';
	import type { AccountsClient } from '$lib/model/owner-accounts-client';

	let {
		client,
		account,
		onsigned
	}: {
		client: AccountsClient;
		account: Account;
		onsigned: (account: Account) => void;
	} = $props();

	let session = $state<{ session: string; desktop_url: string } | null>(null);
	let busy = $state(false);
	let unavailable = $state(false);
	let failure = $state('');
	const signedIn = $derived(account.health === 'signed_in');

	async function start() {
		if (busy) return;
		busy = true;
		failure = '';
		try {
			session = await client.signIn(account.id);
		} catch (cause) {
			if ((cause as { status?: number }).status === 404) unavailable = true;
			else failure = failureSentence(cause, 'The browser could not be opened.');
		} finally {
			busy = false;
		}
	}
	async function done() {
		if (!session || busy) return;
		busy = true;
		failure = '';
		try {
			const saved = await client.signInDone(account.id, session.session);
			session = null;
			onsigned(saved);
			toast(
				saved.health === 'signed_in'
					? `Signed in to ${saved.label}. Companies granted it use this session.`
					: `${saved.label} was kept, but it does not look signed in yet.`,
				saved.health === 'signed_in' ? {} : { tone: 'failure' }
			);
		} catch (cause) {
			failure = failureSentence(cause, 'The sign-in was not kept. Try again.');
		} finally {
			busy = false;
		}
	}
	async function cancel() {
		const open = session;
		session = null;
		failure = '';
		if (open) await client.signInCancel(account.id, open.session).catch(() => {});
	}
	/* Leaving the page abandons an unfinished sign-in rather than holding the browser. */
	onDestroy(() => {
		if (session) void client.signInCancel(account.id, session.session).catch(() => {});
	});
</script>

{#if session}
	<div class="sign-in" aria-label={`Sign in to ${account.label}`} role="group">
		<p class="sign-in-copy">
			Sign in to {account.label} below, including any code it asks for. Only you can type here.
		</p>
		<div class="screen">
			<DesktopViewport
				src={session.desktop_url}
				title={`${account.label} sign-in`}
				interactive
				canResize
				onclaim={async () => true}
			/>
		</div>
		{#if failure}<p class="failure">{failure}</p>{/if}
		<div class="sign-in-actions">
			<button type="button" class="btn small primary" disabled={busy} onclick={() => void done()}
				>{busy ? 'Saving…' : 'I’m signed in'}</button
			>
			<button type="button" class="btn small ghost" disabled={busy} onclick={() => void cancel()}
				>Cancel</button
			>
		</div>
	</div>
{:else if unavailable}
	<p class="note">Signing in here arrives with the next update.</p>
{:else}
	<div class="sign-in-start">
		<button type="button" class="btn small" disabled={busy} onclick={() => void start()}
			>{busy ? 'Opening…' : signedIn ? 'Sign in again' : 'Sign in'}</button
		>
		<small
			>{signedIn
				? 'Companies use the session you signed in with.'
				: 'Opens a private browser for this account. Companies never see the password.'}</small
		>
		{#if failure}<p class="failure">{failure}</p>{/if}
	</div>
{/if}

<style>
	.sign-in {
		display: grid;
		gap: 8px;
	}
	.sign-in-copy,
	.note {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.screen {
		height: min(520px, 60vh);
		min-height: 320px;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--surface-alt);
	}
	.sign-in-actions,
	.sign-in-start {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}
	.sign-in-start small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.failure {
		flex-basis: 100%;
		margin: 0;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
