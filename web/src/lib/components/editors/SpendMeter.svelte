<script lang="ts">
	/* Where the model budget stands (S101), the same on Company › Limits and in an Inbox card: what
	 * is spent against the limit, what is left, and the share, amber from the warning share on. */
	import { formatMoney as money } from '$lib/ui/time';
	import type { CompanyView } from '$lib/model/company';

	let { spend }: { spend: CompanyView['limits']['spend'] } = $props();

	const share = $derived(
		Math.min(100, (spend.accounted_usd / Math.max(spend.ceiling_usd, 0.01)) * 100)
	);
	const monthly = $derived(spend.period === 'monthly');
</script>

<div class="spend-meter">
	<div class="copy">
		<strong
			>{#if spend.status === 'metering_unknown'}<span
					class="lower-bound"
					title="Some model calls reported no price, so actual spend may be higher. Charged work continues until known spend reaches the limit."
					>At least</span
				>{' '}{/if}{money(spend.accounted_usd)}</strong
		>
		<span
			>of {money(spend.ceiling_usd)}
			{monthly ? 'this month' : 'model budget'}{spend.remaining_usd != null
				? ` · ${money(spend.remaining_usd)} left`
				: ''}</span
		>
		<span class="share">{Math.round(share)}%</span>
	</div>
	<div
		class="meter"
		class:high={share >= spend.warning_percent}
		role="meter"
		aria-label="Model spend"
		aria-valuemin={0}
		aria-valuemax={spend.ceiling_usd}
		aria-valuenow={Math.min(spend.accounted_usd, spend.ceiling_usd)}
		aria-valuetext={`${money(spend.accounted_usd)} of ${money(spend.ceiling_usd)} used`}
	>
		<i style={`width: ${share}%`}></i>
	</div>
</div>

<style>
	.spend-meter {
		display: grid;
		gap: 10px;
	}
	.copy {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 6px;
		font-size: var(--t-body);
	}
	.copy strong {
		color: var(--ink);
		font-size: var(--t-head);
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}
	.copy span {
		color: var(--text-tertiary);
	}
	.share {
		margin-left: auto;
		font-variant-numeric: tabular-nums;
	}
	.lower-bound {
		text-decoration: underline dotted;
		text-underline-offset: 3px;
		cursor: help;
	}
	.meter {
		height: 6px;
		overflow: hidden;
		border-radius: 999px;
		background: var(--surface-alt);
	}
	.meter i {
		display: block;
		height: 100%;
		border-radius: inherit;
		background: var(--intent-conversation);
		transition: width var(--motion-disclosure) var(--ease-out);
	}
	.meter.high i {
		background: var(--intent-authority);
	}
</style>
