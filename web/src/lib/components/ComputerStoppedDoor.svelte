<script lang="ts">
	/* The door beside "the owner stopped the company computer", wherever that stops someone: a
	 * file, a review, the Library's file list. The owner starts it right here. Anyone else cannot,
	 * so they ask the owner, who is told like any mention; this then waits for the computer and
	 * carries on by itself once the owner starts it. */
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { askOwnerToStartComputer, computerPower, recoverCompany } from '$lib/model/company';
	import { whileSeen } from '$lib/model/activity';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';

	let {
		companyId,
		onstarting,
		onstarted,
		onfailed
	}: {
		companyId: string;
		/** The owner pressed Start computer: show it starting. */
		onstarting?: () => void;
		/** The computer is on again (the owner's start, or one a teammate waited for). */
		onstarted: () => void;
		/** The start failed, in a sentence for the owner. */
		onfailed?: (sentence: string) => void;
	} = $props();

	const principal = $derived(companyPrincipalQuery(companyId));
	const known = $derived(!!principal.view);
	const owner = $derived(principal.view?.membership_role === 'owner');
	let busy = $state(false);
	let asked = $state(false);

	async function start() {
		if (busy) return;
		busy = true;
		onstarting?.();
		try {
			await recoverCompany(companyId, 'start');
			onstarted();
		} catch (cause) {
			onfailed?.(failureSentence(cause, 'The computer could not start. Try again.'));
		} finally {
			busy = false;
		}
	}

	async function ask() {
		if (busy || asked) return;
		busy = true;
		try {
			const answer = await askOwnerToStartComputer(companyId);
			if (!answer.sent) {
				onstarted();
				return;
			}
			asked = true;
			toast('Asked the owner to start the company computer. This opens by itself once they do.');
		} catch (cause) {
			toast(failureSentence(cause, 'The owner could not be asked. Try again.'), {
				tone: 'failure'
			});
		} finally {
			busy = false;
		}
	}

	/* A teammate cannot start it, so watch for the owner doing so, gently: twice a minute while
	 * the page is in use, never while it is hidden. */
	$effect(() => {
		if (!known || owner) return;
		const company = companyId;
		const controller = new AbortController();
		const check = () =>
			void computerPower(company, controller.signal)
				.then((answer) => {
					if (!controller.signal.aborted && answer.state !== 'stopped') onstarted();
				})
				.catch(() => {});
		const stop = whileSeen(check, 30_000);
		return () => {
			controller.abort();
			stop();
		};
	});
</script>

{#if owner}
	<button
		class="btn small primary"
		type="button"
		disabled={busy}
		aria-busy={busy}
		title="Starts it on the current release. Files and the browser profile are kept."
		onclick={() => void start()}>Start computer</button
	>
{:else if known}
	<button
		class="btn small primary"
		type="button"
		disabled={busy || asked}
		aria-busy={busy}
		title={asked
			? 'Asked. This opens by itself once the owner starts the computer.'
			: 'Only the owner can start it. They are told at once, like a mention.'}
		onclick={() => void ask()}>Ask the owner</button
	>
{/if}
