<script lang="ts">
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { companySource } from '$lib/model/companySource.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companySource(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);

	function money(value: number): string {
		return new Intl.NumberFormat(undefined, { style: 'currency', currency: 'USD' }).format(value);
	}

	function minor(value: number, currency: string): string {
		return new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(value / 100);
	}
</script>

<svelte:head><title>Authority & limits — {view?.company.name ?? companyId}</title></svelte:head>

<div class="company-page authority-page">
	<header class="company-page-head">
		<h1>Authority & limits</h1>
		<a class="attention-crosslink" href={view?.attention_href ?? `/${companyId}`}
			>Open owner decisions in Attention →</a
		>
	</header>
	{#if view}
		{#if view.limits.status !== 'available'}
			<p class="source-unavailable">
				Authority is unavailable. Limits and grants below are not being presented as an empty
				mandate.
			</p>
		{:else}
			<div class="authority-boundaries">
				<section>
					<div class="section-heading">
						<h2>May do independently</h2>
						<InfoTip
							text="These are the outer classes of work allowed without a new owner decision. Each real effect still passes its source-owned checks."
						/>
					</div>
					{#each view.limits.independently as item (item.title)}<article>
							<strong>{item.title}</strong>
							<p>{item.explanation}</p>
						</article>{/each}
				</section>
				<section>
					<div class="section-heading">
						<h2>Asks you</h2>
						<InfoTip
							text="Company shows the boundary; Attention remains the only place that resolves a pending owner decision."
						/>
					</div>
					{#each view.limits.asks_owner as item (item.title)}<article>
							<strong>{item.title}</strong>
							<p>{item.explanation}</p>
						</article>{/each}
				</section>
				<section>
					<div class="section-heading">
						<h2>Cannot do</h2>
						<InfoTip
							text="These are authority and custody boundaries, not a speculative catalogue of every harmful act."
						/>
					</div>
					{#each view.limits.cannot as item (item.title)}<article>
							<strong>{item.title}</strong>
							<p>{item.explanation}</p>
						</article>{/each}
				</section>
			</div>

			<section class="limits-ledger">
				<div class="section-heading">
					<h2>Model spend</h2>
					<InfoTip
						text="The ceiling is a hard company fuse. Spend detail remains supporting evidence, not the product pitch."
					/>
				</div>
				<div class="spend-line">
					<div><strong>{money(view.limits.spend.accounted_usd)}</strong><span>accounted</span></div>
					<div><strong>{money(view.limits.spend.ceiling_usd)}</strong><span>ceiling</span></div>
					<div>
						<strong
							>{view.limits.spend.remaining_usd == null
								? 'Unknown'
								: money(view.limits.spend.remaining_usd)}</strong
						><span>remaining</span>
					</div>
				</div>
				<div
					class="spend-track"
					aria-label={`${money(view.limits.spend.accounted_usd)} of ${money(view.limits.spend.ceiling_usd)} accounted`}
				>
					<i
						style={`width: ${Math.min(100, (view.limits.spend.accounted_usd / Math.max(view.limits.spend.ceiling_usd, 0.01)) * 100)}%`}
					></i>
				</div>
			</section>

			<div class="limits-lower">
				<section>
					<h2>Approved external parties</h2>
					{#if view.limits.approved_parties.length}<div class="party-list">
							{#each view.limits.approved_parties as party}<span>{party}</span>{/each}
						</div>{:else}<p class="quiet-empty">
							No external party currently has a standing first-contact grant.
						</p>{/if}
				</section>
				<section>
					<h2>Money envelopes</h2>
					{#if view.limits.money_envelopes.length}
						{#each view.limits.money_envelopes as envelope (envelope.currency)}
							<div class="money-envelope">
								<strong>{envelope.currency}</strong><span
									>{minor(envelope.per_payment_limit_minor, envelope.currency)} each · {minor(
										envelope.aggregate_limit_minor,
										envelope.currency
									)} total</span
								><em class:frozen={envelope.frozen}>{envelope.frozen ? 'frozen' : 'active'}</em>
							</div>
						{/each}
					{:else}<p class="quiet-empty">No operating-money envelope is recorded.</p>{/if}
				</section>
			</div>
		{/if}
	{:else if source.failure}<div class="company-source-error" role="alert">
			{source.failure.message}
		</div>{:else}<div class="company-page-wait" aria-label="Reading Authority"></div>{/if}
</div>
