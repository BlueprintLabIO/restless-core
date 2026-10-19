<script lang="ts">
	/* A connection that stopped accepting the company's sign-in, signed in again from its Inbox
	 * card (S101). The provider's page opens in a small window, since a provider will not sign in
	 * inside another site; the card watches and clears itself once the tools are back. */
	import { onDestroy } from 'svelte';
	import { signIn } from '$lib/model/connections';
	import { finishAppSignIn, popupFeatures } from '$lib/model/inline-credentials';
	import { failureSentence } from '$lib/model/failure';

	let {
		companyId,
		name,
		owner,
		ondone
	}: { companyId: string; name: string; owner: boolean; ondone: (receipt: string) => void } =
		$props();

	let busy = $state(false);
	let waiting = $state<Window | null | undefined>(undefined);
	let error = $state('');
	let timer: ReturnType<typeof setInterval> | undefined;

	function start() {
		if (busy) return;
		/* Opened inside the click so no popup blocker stops it, then pointed at the provider. */
		const popup = window.open('about:blank', 'restless-sign-in', popupFeatures());
		busy = true;
		error = '';
		void signIn(companyId, name)
			.then((url) => {
				if (popup) popup.location.href = url;
				else window.open(url, 'restless-sign-in', popupFeatures());
				waiting = popup;
				clearInterval(timer);
				timer = setInterval(() => void check(), 2_000);
			})
			.catch((cause) => {
				popup?.close();
				error = failureSentence(cause, `The sign-in to ${name} did not start. Try again.`);
			})
			.finally(() => (busy = false));
	}

	async function check() {
		if (!(await finishAppSignIn(companyId, name).catch(() => false))) return;
		clearInterval(timer);
		waiting?.close();
		waiting = undefined;
		ondone(`${name} is signed in again. Its tools are back and work carries on.`);
	}

	onDestroy(() => clearInterval(timer));
</script>

<section class="inline-reconnect" aria-label={`Sign in to ${name}`}>
	{#if !owner}
		<p class="quiet">Waiting for the owner to sign in to {name}.</p>
	{:else if waiting !== undefined}
		<p class="quiet" role="status">Finish signing in to {name} in the window that opened. This card clears by itself.</p>
		<button class="btn small" type="button" onclick={start}>Open it again</button>
	{:else}
		<button class="btn small primary" type="button" disabled={busy} onclick={start}
			>{busy ? 'Opening…' : `Sign in to ${name}`}</button
		>
	{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
	.inline-reconnect {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		margin-top: 12px;
	}
	.quiet,
	.error {
		margin: 0;
		font-size: var(--t-label);
	}
	.quiet {
		color: var(--text-tertiary);
	}
	.error {
		flex-basis: 100%;
		color: var(--state-danger);
	}
</style>
