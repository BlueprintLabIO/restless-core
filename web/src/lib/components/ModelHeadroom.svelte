<script lang="ts">
	/* Company › Model: how much headroom the account behind this company's model has (S100), so a
	 * limit never arrives as a surprise. Shows nothing for a model whose account reports no limits. */
	import { onDestroy, onMount } from 'svelte';
	import {
		freshness,
		meterTone,
		readCompanyHeadroom,
		resetsIn,
		timeOfDay,
		type CompanyHeadroom
	} from '$lib/model/account-usage';

	let { companyId }: { companyId: string } = $props();

	let headroom = $state<CompanyHeadroom | null>(null);
	let now = $state(Date.now());

	async function refresh() {
		try {
			headroom = await readCompanyHeadroom(companyId);
		} catch {
			headroom = null;
		}
	}

	let timer: ReturnType<typeof setInterval> | undefined;
	onMount(() => {
		void refresh();
		timer = setInterval(() => {
			now = Date.now();
			if (document.visibilityState === 'visible' && Math.floor(now / 1000) % 60 === 0) void refresh();
		}, 1_000);
	});
	onDestroy(() => clearInterval(timer));
</script>

{#if headroom?.windows.length}
	<section class="headroom" aria-label="Account headroom">
		<h3>Account headroom</h3>
		<div class="windows">
			{#each headroom.windows as window (window.name)}
				{@const tone = meterTone(window.used_percent)}
				{@const reset = resetsIn(window.resets_at, now)}
				<div class="window">
					<div class="line">
						<span>{window.label}</span>
						<span class="figure {tone}">{Math.round(window.used_percent)}% used</span>
					</div>
					<div
						class="meter {tone}"
						role="meter"
						aria-label={`${window.label} limit used`}
						aria-valuemin="0"
						aria-valuemax="100"
						aria-valuenow={Math.round(window.used_percent)}
					>
						<span style:width={`${Math.min(100, window.used_percent)}%`}></span>
					</div>
					<div class="facts">
						{#if reset}<span>{reset}{window.resets_at ? ` (${timeOfDay(window.resets_at, now)})` : ''}</span>{/if}
						{#if window.runs_out_at && window.used_percent < 100}<span class="forecast"
								>Runs out around {timeOfDay(window.runs_out_at, now)} at this pace</span
							>{/if}
						<span class="stamp">{freshness(window.observed_at, now)}</span>
					</div>
				</div>
			{/each}
		</div>
	</section>
{/if}

<style>
	.headroom {
		display: grid;
		gap: 8px;
		margin: 0 0 20px;
	}
	h3 {
		margin: 0;
		font-size: var(--t-label);
		font-weight: 600;
		color: var(--text-secondary);
	}
	.windows {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
		gap: 12px 20px;
	}
	.window {
		display: grid;
		gap: 5px;
	}
	.line {
		display: flex;
		justify-content: space-between;
		font-size: var(--t-label);
		color: var(--text-secondary);
	}
	.figure {
		font-variant-numeric: tabular-nums;
	}
	.figure.warn {
		color: var(--state-warning, var(--text-primary));
	}
	.figure.full {
		color: var(--state-danger);
	}
	.meter {
		height: 6px;
		border-radius: 99px;
		background: var(--surface-alt);
		overflow: hidden;
	}
	.meter span {
		display: block;
		height: 100%;
		border-radius: inherit;
		background: var(--state-success);
	}
	.meter.warn span {
		background: var(--state-warning, var(--text-secondary));
	}
	.meter.full span {
		background: var(--state-danger);
	}
	.facts {
		display: flex;
		flex-wrap: wrap;
		gap: 2px 10px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.forecast {
		color: var(--state-warning, var(--text-secondary));
	}
</style>
