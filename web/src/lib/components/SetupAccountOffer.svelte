<script lang="ts">
	/* M6: a new company can start on the plan's included AI while the owner is already signed in to
	 * ChatGPT or Claude. Before Exec's first question, setup names those accounts and lets the owner
	 * choose one, or keep the included AI. Nothing changes until the owner picks. */
	import { untrack } from 'svelte';
	import { failureSentence } from '$lib/model/failure';
	import { agentRouteState, intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { PROVIDER_LABELS } from '$lib/model/provider-labels';
	import {
		readReusableConnections,
		useAccountConnection,
		type ReusableConnection
	} from '$lib/model/provider';
	import {
		accountName,
		includedRoute,
		offerSentence,
		setupAccountOffer,
		startingModel
	} from '$lib/model/setup-model-offer';

	let { companyId, interviewStarted }: { companyId: string; interviewStarted: boolean } = $props();

	const catalog = modelCatalog();
	const intelligence = untrack(() => intelligenceQuery(companyId));
	const keptKey = $derived(`restless.setup-included-kept.${companyId}`);

	let connections = $state<ReusableConnection[]>([]);
	let includedProviders = $state<string[]>([]);
	let kept = $state(false);
	let busy = $state(false);
	let failure = $state('');
	let using = $state('');

	$effect(() => {
		try {
			kept = localStorage.getItem(keptKey) === '1';
		} catch {
			kept = false;
		}
	});
	$effect(() => {
		let live = true;
		void readReusableConnections()
			.then((view) => {
				if (!live) return;
				connections = view.connections;
				includedProviders = view.includedProviders;
			})
			.catch(() => {});
		return () => {
			live = false;
		};
	});

	const offered = $derived(
		setupAccountOffer({
			view: intelligence.view,
			connections,
			includedProviders,
			chosen: kept || !!using,
			interviewStarted
		})
	);
	const names = $derived(offered.map((row) => accountName(row, PROVIDER_LABELS)));

	function keep() {
		kept = true;
		try {
			localStorage.setItem(keptKey, '1');
		} catch {
			/* The card returns on reload; nothing else depends on it. */
		}
	}

	async function choose(row: ReusableConnection) {
		if (busy) return;
		busy = true;
		failure = '';
		const kind = row.kind ?? 'api_key';
		try {
			const model = startingModel(row.provider, catalog.models(row.provider, kind));
			if (!model) throw new Error('No model is listed for that account yet. Use Intelligence instead.');
			await useAccountConnection(companyId, row.id, model, false);
			announceIntelligenceChange(companyId);
			using = accountName(row, PROVIDER_LABELS);
			/* Exec's new route loads on the company's computer in a few seconds. */
			const until = Date.now() + 60_000;
			while (Date.now() < until) {
				await intelligence.refresh().catch(() => {});
				if (!includedRoute(intelligence.view) && agentRouteState(intelligence.view, 'exec') === 'ready')
					break;
				await new Promise((resolve) => setTimeout(resolve, 2000));
			}
		} catch (cause) {
			using = '';
			failure = failureSentence(cause, 'That account could not be used here. Try again.');
		} finally {
			busy = false;
		}
	}

	const pays = (row: ReusableConnection) =>
		row.kind === 'oauth' ? `Your ${accountName(row)} plan pays.` : 'Billed to that key.';
</script>

{#if using}
	<p class="offer-done" role="status">
		{busy ? `Switching to ${using}…` : `I’ll think with ${using} from here.`}
	</p>
{:else if offered.length}
	<div class="setup-offer" role="group" aria-label="Choose what Exec thinks with">
		<p>
			You’re signed in to {offerSentence(names)}. I’m on the included AI right now. Want me to use
			yours instead?
		</p>
		<ul>
			{#each offered as row (row.id)}
				<li>
					<button
						type="button"
						class="btn small primary"
						disabled={busy}
						title={pays(row)}
						onclick={() => void choose(row)}>Use {accountName(row, PROVIDER_LABELS)}</button
					>
				</li>
			{/each}
			<li>
				<button type="button" class="btn small" disabled={busy} onclick={keep}
					>Keep the included AI</button
				>
			</li>
		</ul>
		{#if failure}<p class="offer-failure" role="alert">{failure}</p>{/if}
	</div>
{/if}

<style>
	.setup-offer {
		display: grid;
		gap: 10px;
		margin: 6px 0 4px 38px;
		padding: 12px 14px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
	}
	.setup-offer > p {
		margin: 0;
		color: var(--ink);
		font-weight: 500;
		line-height: 1.4;
	}
	ul {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.offer-done {
		margin: 4px 0 0 38px;
		color: var(--text-secondary);
	}
	.offer-failure {
		margin: 0;
		color: var(--danger, var(--ink));
		font-size: var(--t-label);
	}
</style>
