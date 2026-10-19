<script lang="ts">
	/* Settings › Accounts › Usage (S100): how much headroom each AI account has. One block per
	 * account: a meter per limit window, when it resets in your own time, when it runs out at the
	 * pace of the last hour, which companies used it, and how fresh the numbers are. Numbers come
	 * only from what the account reported on calls companies were already making; an account that
	 * has reported nothing says so instead of guessing. */
	import { onDestroy, onMount } from 'svelte';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import { readAccountConnections, type AccountConnection } from '$lib/model/account-connections';
	import {
		connectionOf,
		freshness,
		meterTone,
		readAccountUsage,
		resetsIn,
		timeOfDay,
		type AccountUsage
	} from '$lib/model/account-usage';

	let { companies }: { companies: { id: string; name: string }[] } = $props();

	let usage = $state<AccountUsage[]>([]);
	let connections = $state<AccountConnection[]>([]);
	let loaded = $state(false);
	let failed = $state(false);
	let now = $state(Date.now());

	const companyName = (id: string) => companies.find((company) => company.id === id)?.name ?? id;
	/* Sign-ins report limits; keys bill per use. Every sign-in is listed, reported or not. */
	const rows = $derived(
		connections
			.filter((connection) => connection.kind === 'oauth')
			.map((connection) => ({
				connection,
				usage: usage.find((entry) => connectionOf(entry.account) === connection.id) ?? null
			}))
	);

	async function refresh() {
		try {
			const [read, view] = await Promise.all([readAccountUsage(), readAccountConnections()]);
			usage = read;
			connections = view.connections;
			failed = false;
		} catch {
			failed = true;
		} finally {
			loaded = true;
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

{#if loaded && (rows.length || failed)}
	<section class="usage" aria-label="Usage">
		<header class="head">
			<h3>Usage</h3>
			<InfoTip
				text="How much of each sign-in's limits is used, as the account itself reported it on your companies' last calls. Restless never spends a call to check. When one runs out, a company with a fallback model carries on with it."
			/>
		</header>
		{#if failed}
			<p class="quiet" role="alert">Usage could not be read just now.</p>
		{:else}
			<div class="list">
				{#each rows as row (row.connection.id)}
					<article class="account">
						<div class="who">
							<strong>{row.connection.label}</strong>
							{#if row.connection.account_identity}<small>{row.connection.account_identity}</small>{/if}
						</div>
						{#if row.usage?.windows.length}
							<div class="windows">
								{#each row.usage.windows as window (window.name)}
									{@const tone = meterTone(window.used_percent)}
									{@const reset = resetsIn(window.resets_at, now)}
									<div class="window">
										<div class="line">
											<span class="label">{window.label}</span>
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
											{#if reset}<span title={window.resets_at ? new Date(window.resets_at).toLocaleString() : undefined}
													>{reset}{window.resets_at ? ` (${timeOfDay(window.resets_at, now)})` : ''}</span
												>{/if}
											{#if window.runs_out_at && window.used_percent < 100}<span class="forecast"
													>At this pace it runs out around {timeOfDay(window.runs_out_at, now)}</span
												>{/if}
											{#if window.by_company.length}<span
													>{window.by_company
														.slice(0, 3)
														.map(([company, points]) => `${companyName(company)} ${Math.round(points)}%`)
														.join(' · ')}</span
												>{/if}
											<span class="stamp">{freshness(window.observed_at, now)}</span>
										</div>
									</div>
								{/each}
							</div>
						{:else}
							<p class="unknown">
								Unknown. It reports its limits once a company uses it, so nothing is shown until then.
							</p>
						{/if}
					</article>
				{/each}
			</div>
		{/if}
	</section>
{/if}

<style>
	.usage {
		display: grid;
		gap: 12px;
		margin-top: 28px;
		min-width: 0;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.list {
		display: grid;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
		overflow: hidden;
	}
	.account {
		display: grid;
		grid-template-columns: minmax(140px, 0.6fr) minmax(0, 2fr);
		gap: 16px;
		padding: 12px 14px;
	}
	.account + .account {
		border-top: 1px solid var(--border);
	}
	.who {
		display: grid;
		align-content: start;
		gap: 1px;
		min-width: 0;
	}
	.who strong {
		font-weight: 600;
	}
	.who small,
	.quiet,
	.unknown {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.unknown,
	.quiet {
		margin: 0;
	}
	.windows {
		display: grid;
		gap: 12px;
		min-width: 0;
	}
	.window {
		display: grid;
		gap: 5px;
	}
	.line {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		font-size: var(--t-label);
	}
	.label {
		color: var(--text-secondary);
	}
	.figure {
		font-variant-numeric: tabular-nums;
		color: var(--text-secondary);
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
		transition: width 300ms ease;
	}
	.meter.warn span {
		background: var(--state-warning, var(--text-secondary));
	}
	.meter.full span {
		background: var(--state-danger);
	}
	@media (prefers-reduced-motion: reduce) {
		.meter span {
			transition: none;
		}
	}
	.facts {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 12px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.forecast {
		color: var(--state-warning, var(--text-secondary));
	}
	.stamp {
		margin-left: auto;
	}
	@media (max-width: 640px) {
		.account {
			grid-template-columns: minmax(0, 1fr);
			gap: 8px;
		}
		.stamp {
			margin-left: 0;
		}
	}
</style>
