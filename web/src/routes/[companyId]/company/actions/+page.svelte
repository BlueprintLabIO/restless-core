<script lang="ts">
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import EmptyState from '$lib/ui/views/EmptyState.svelte';
	import SettingsHeader from '$lib/ui/views/SettingsHeader.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { companyQuery } from '$lib/model/queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);

	const when = (value?: Date | string) => formatRelative(value, 'Not yet');

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

<svelte:head><title>External activity — {view?.company.name ?? companyId}</title></svelte:head>

<div class="company-page actions-page">
	<SettingsHeader
		title="External activity"
		explanation="Only consequential effects and provider outcomes belong here. Shell commands, builds and Git activity remain with the Work that produced them."
	></SettingsHeader>
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
							<time title={formatMoment(item.observed_at)}>{when(item.observed_at)}</time><span
								>{item.actor ?? 'source record'}</span
							>{#if item.detail}<InfoTip text={item.detail} />{/if}
						</div>
					</article>
				{:else}
					<EmptyState
						title="No external activity yet"
						explanation="Messages sent, forms submitted and payments made on the company's behalf appear here."
					/>
				{/each}
			</div>
		{/if}
	{:else if source.failure}
		<FailureNotice
			error={source.failure}
			subject="external actions"
			variant="block"
			onretry={source.refresh}
		/>
	{:else}<Skeleton label="Reading external actions…" variant="page" count={4} />{/if}
</div>
