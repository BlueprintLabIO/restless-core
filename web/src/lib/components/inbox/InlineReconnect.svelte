<script lang="ts">
	/* A connection that stopped accepting the company's sign-in, signed in again from its card
	 * (S101). The footer opens the provider in a small window, since a provider will not sign in
	 * inside another site; the card watches and clears itself once the tools are back. */
	import { onDestroy } from 'svelte';
	import { signIn } from '$lib/model/connections';
	import { finishAppSignIn, popupFeatures } from '$lib/model/inline-credentials';
	import { failureSentence } from '$lib/model/failure';
	import InlinePanel from './InlinePanel.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		name,
		owner,
		ondone
	}: { companyId: string; name: string; owner: boolean; ondone: (receipt: string) => void } =
		$props();

	let waiting = $state<Window | null | undefined>(undefined);
	let error = $state('');
	let timer: ReturnType<typeof setInterval> | undefined;

	async function start() {
		/* Opened inside the click so no popup blocker stops it, then pointed at the provider. */
		const popup = window.open('about:blank', 'restless-sign-in', popupFeatures());
		error = '';
		try {
			const url = await signIn(companyId, name);
			if (popup) popup.location.href = url;
			else window.open(url, 'restless-sign-in', popupFeatures());
			waiting = popup;
			clearInterval(timer);
			timer = setInterval(() => void check(), 2_000);
		} catch (cause) {
			popup?.close();
			error = failureSentence(cause, `The sign-in to ${name} did not start. Try again.`);
		}
	}

	async function check() {
		if (!(await finishAppSignIn(companyId, name).catch(() => false))) return;
		clearInterval(timer);
		waiting?.close();
		waiting = undefined;
		ondone(`${name} is signed in again. Its tools are back and work carries on.`);
	}

	onDestroy(() => clearInterval(timer));

	cardActions(() =>
		owner
			? {
					primary: {
						label: waiting !== undefined ? 'Open the sign-in again' : `Sign in to ${name}`,
						busyLabel: 'Opening…',
						run: start
					}
				}
			: {}
	);
</script>

<InlinePanel
	label={`Sign in to ${name}`}
	{error}
	note={!owner
		? `Waiting for the owner to sign in to ${name}.`
		: waiting !== undefined
			? `Finish signing in to ${name} in the window that opened. This card clears by itself.`
			: `${name} opens in a small window. What the company may do with it stays as you set it.`}
/>
