<script lang="ts">
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { companyQuery } from '$lib/model/queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);

	function when(value: string): string {
		return new Date(value).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function words(value: string): string {
		return value.replaceAll('_', ' ');
	}

	function evidenceExplanation(value: string): string {
		return (
			{
				provider_confirmed: 'An authenticated provider observation names the external result.',
				self_attested:
					'A governed local process recorded this result, but no independent provider confirmation is claimed.',
				reconciled: 'A later governed external status check closed an earlier unknown outcome.',
				legacy_unverified: 'This preserved record predates the current governed receipt contract.',
				unknown: 'Durable intent exists but no authoritative result receipt does.',
				authority_recorded:
					'Authority has reserved or submitted this consequence; no provider result is claimed yet.'
			}[value] ?? 'Evidence state reported by the owning source.'
		);
	}
</script>

<svelte:head><title>External actions — {view?.company.name ?? companyId}</title></svelte:head>

<div class="company-page actions-page">
	<header class="company-page-head">
		<h1>External actions</h1>
		<InfoTip
			text="Only consequential effects and provider outcomes belong here. Shell commands, builds and Git activity remain with the Work that produced them."
		/>
	</header>
	{#if view}
		{#if view.external_actions.status !== 'available'}
			<p class="source-unavailable">
				Authority is unavailable. External history is not being presented as empty.
			</p>
		{:else}
			<div class="action-ledger">
				{#each view.external_actions.items as item (item.id)}
					<article>
						<div class="action-outcome">
							<span class="state-chip state-{item.state}">{words(item.state)}</span><span
								class="evidence-chip evidence-{item.evidence}">{words(item.evidence)}</span
							><InfoTip text={evidenceExplanation(item.evidence)} />
						</div>
						<div class="action-copy">
							<strong>{item.title}</strong><span
								>{words(item.effect_class)}{item.party ? ` · ${item.party}` : ''}</span
							>
						</div>
						<div class="action-provenance">
							<time>{when(item.observed_at)}</time><span>{item.actor ?? 'source record'}</span
							>{#if item.detail}<InfoTip text={item.detail} />{/if}
						</div>
					</article>
				{:else}
					<p class="quiet-empty">No governed external action has been recorded.</p>
				{/each}
			</div>
		{/if}
	{:else if source.failure}<div class="company-source-error" role="alert">
			{source.failure.message}
		</div>{:else}<div class="company-page-wait" aria-label="Reading external actions"></div>{/if}
</div>
