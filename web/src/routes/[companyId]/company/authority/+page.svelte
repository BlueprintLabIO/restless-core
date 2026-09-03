<script lang="ts">
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { setCompanyOutcomeStandard, type OutcomeStandard } from '$lib/model/company';
	import { companyQuery } from '$lib/model/queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);
	let standardSaving = $state(false);
	let standardNotice = $state('');
	let standardFailure = $state('');

	async function changeOutcomeStandard(event: Event) {
		const standard = (event.currentTarget as HTMLSelectElement).value as OutcomeStandard;
		if (!view || standard === view.company.outcome_standard || standardSaving) return;
		standardSaving = true;
		standardNotice = '';
		standardFailure = '';
		try {
			const next = await setCompanyOutcomeStandard(companyId, standard);
			source.accept(next);
			standardNotice = `New outcomes now inherit ${standard}. Existing outcomes are unchanged.`;
		} catch (cause) {
			standardFailure =
				cause instanceof Error ? cause.message : 'The outcome standard was not changed.';
		} finally {
			standardSaving = false;
		}
	}

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
			<section class="outcome-standard-setting">
				<div class="section-heading">
					<h2>Outcome standard</h2>
					<InfoTip
						text="The ambition new outcomes inherit. It changes how deeply Restless explores and evaluates; it never lowers safety or raises the company spend ceiling."
					/>
				</div>
				<div class="standard-setting-line">
					<div>
						<strong
							>{view.company.outcome_standard[0].toUpperCase() +
								view.company.outcome_standard.slice(1)}</strong
						>
						<span>Default for newly commissioned outcomes</span>
					</div>
					<select
						value={view.company.outcome_standard}
						disabled={standardSaving}
						onchange={changeOutcomeStandard}
						aria-label="Company outcome standard"
					>
						<option value="fast">Fast — smallest correct result</option>
						<option value="thorough">Thorough — production ready</option>
						<option value="exceptional">Exceptional — clearly superior</option>
						<option value="frontier">Frontier — seek a new ceiling</option>
					</select>
				</div>
				{#if standardNotice}<p class="standard-setting-message" role="status">
						{standardNotice}
					</p>{/if}
				{#if standardFailure}<p class="standard-setting-message failure" role="alert">
						{standardFailure}
					</p>{/if}
			</section>

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

<style>
	.outcome-standard-setting {
		display: grid;
		gap: 12px;
		padding: 18px;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: rgba(255, 255, 255, 0.56);
		box-shadow: var(--bevel-subtle);
	}

	.standard-setting-line {
		display: flex;
		align-items: end;
		justify-content: space-between;
		gap: 18px;
	}

	.standard-setting-line > div,
	.standard-setting-line strong,
	.standard-setting-line span {
		display: block;
	}

	.standard-setting-line strong {
		font-size: var(--t-title);
		color: var(--text-primary);
	}

	.standard-setting-line span,
	.standard-setting-message {
		margin-top: 4px;
		color: var(--text-secondary);
		font-size: var(--t-body-small);
	}

	.standard-setting-line select {
		max-width: min(100%, 300px);
		padding: 8px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--text-primary);
		font: 600 var(--t-body-small) var(--font-sans);
	}

	.standard-setting-message.failure {
		color: var(--danger);
	}

	@media (max-width: 720px) {
		.standard-setting-line {
			align-items: stretch;
			flex-direction: column;
		}

		.standard-setting-line select {
			max-width: none;
		}
	}
</style>
