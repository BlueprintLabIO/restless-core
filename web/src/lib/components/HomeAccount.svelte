<script lang="ts">
	/* Home's account band: what each company's AI has cost against the limit the owner set it, and
	 * when this computer last backed everything up. Only what an owner API returns: a company whose
	 * spend this viewer may not read, or a plane that keeps no backups, simply has no row. */
	import Meter from '$lib/ui/data/Meter.svelte';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import { getCompanySpend, type CompanySpendView } from '$lib/model/company';
	import {
		getApplianceOperations,
		type ApplianceOperations,
		type ApplianceStatus
	} from '$lib/model/appliance';

	let {
		companies,
		appliance
	}: {
		/** The active companies, in the order Home lists them. */
		companies: { id: string; name: string }[];
		appliance: ApplianceStatus | null;
	} = $props();

	let spend = $state<Record<string, CompanySpendView>>({});
	let operations = $state<ApplianceOperations | null>(null);

	const ids = $derived(companies.map((company) => company.id).join('\n'));
	$effect(() => {
		const wanted = ids ? ids.split('\n') : [];
		let stale = false;
		void Promise.allSettled(wanted.map((id) => getCompanySpend(id))).then((results) => {
			if (stale) return;
			const next: Record<string, CompanySpendView> = {};
			results.forEach((result, index) => {
				if (result.status === 'fulfilled') next[wanted[index]] = result.value;
			});
			spend = next;
		});
		return () => (stale = true);
	});
	/* A hosted plane's backups are its operator's; only a local appliance reports its own. */
	const local = $derived(appliance?.profile === 'stable' && !appliance.hosted);
	$effect(() => {
		if (!local) return;
		const controller = new AbortController();
		void getApplianceOperations(controller.signal)
			.then((value) => (operations = value))
			.catch(() => {});
		return () => controller.abort();
	});

	const rows = $derived(
		companies.flatMap((company) => {
			const view = spend[company.id];
			return view ? [{ ...company, spend: view.spend, native: view.native }] : [];
		})
	);
	const monthly = $derived(rows.length > 0 && rows.every((row) => row.spend.period === 'monthly'));
	const total = $derived(rows.reduce((sum, row) => sum + row.spend.accounted_usd, 0));

	const usd = (value: number) =>
		value.toLocaleString('en-US', { style: 'currency', currency: 'USD' });
	const day = (value: string | number) =>
		new Date(value).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	const size = (bytes: number) =>
		bytes >= 1e9 ? `${(bytes / 1e9).toFixed(1)} GB` : `${Math.max(1, Math.round(bytes / 1e6))} MB`;
	/* The first day of the next UTC month after the period began. */
	function renews(periodStart: string | null | undefined): string {
		if (!periodStart) return '';
		const start = new Date(periodStart);
		if (Number.isNaN(start.getTime())) return '';
		const next = new Date(Date.UTC(start.getUTCFullYear(), start.getUTCMonth() + 1, 1));
		return next.toLocaleDateString(undefined, { month: 'short', day: 'numeric', timeZone: 'UTC' });
	}
	function used(row: (typeof rows)[number]): number {
		return row.spend.ceiling_usd > 0 ? row.spend.accounted_usd / row.spend.ceiling_usd : 0;
	}
	function tone(row: (typeof rows)[number]): 'conversation' | 'warning' | 'danger' {
		if (row.spend.status === 'exhausted') return 'danger';
		return used(row) * 100 >= row.spend.warning_percent ? 'warning' : 'conversation';
	}
	function reading(row: (typeof rows)[number]): string {
		const { spend } = row;
		if (spend.status === 'metering_unknown')
			return `${usd(spend.accounted_usd)} so far · not fully metered`;
		const period =
			spend.period === 'monthly'
				? `this month${renews(spend.period_start) ? ` · renews ${renews(spend.period_start)}` : ''}`
				: 'in total';
		return `${usd(spend.accounted_usd)} of ${usd(spend.ceiling_usd)} ${period}`;
	}
	const backup = $derived(operations?.last_backup ?? null);
</script>

{#if rows.length || operations}
	<div class="home-account">
		{#if rows.length}
			<section aria-labelledby="home-ai-use">
				<header class="home-account-head">
					<h2 id="home-ai-use">AI use</h2>
					<InfoTip
						text="What each company's models have cost against the spend limit you gave it. At the limit that company's AI pauses until its period renews or you raise the limit. Use on a company's own Codex or Claude sign-in is counted in turns, not dollars."
					/>
					{#if monthly}<span class="home-account-total">{usd(total)} this month</span>{/if}
				</header>
				<ul class="home-account-list">
					{#each rows as row (row.id)}
						<li>
							<a href={`/${encodeURIComponent(row.id)}/company/resources`}>
								<strong>{row.name}</strong>
								<span class="home-account-meter"
									><Meter
										fraction={used(row)}
										tone={tone(row)}
										label={`${row.name} AI spend against its limit`}
									/></span
								>
								<small
									>{reading(row)}{#if row.native.turns > 0}
										· {row.native.turns.toLocaleString()} own sign-in
										{row.native.turns === 1 ? 'turn' : 'turns'}{/if}</small
								>
							</a>
						</li>
					{/each}
				</ul>
			</section>
		{/if}
		{#if operations}
			<section aria-labelledby="home-computer">
				<header class="home-account-head">
					<h2 id="home-computer">This computer</h2>
					<InfoTip
						text="One backup holds every company on this computer, with its files, databases, passwords and keys."
					/>
				</header>
				<ul class="home-account-list">
					<li>
						<a href="/account/settings/this-computer">
							<strong>Last backup</strong>
							<small class:warning={!backup}
								>{backup
									? `${day(backup.created_at_unix_seconds * 1000)} · ${size(backup.bytes)}`
									: 'Never backed up'}</small
							>
						</a>
					</li>
				</ul>
			</section>
		{/if}
	</div>
{/if}

<style>
	.home-account {
		display: grid;
		gap: 28px;
		margin-top: 32px;
	}
	.home-account-head {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 10px;
	}
	.home-account-head h2 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.home-account-total {
		margin-left: auto;
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
	}
	/* The same quiet list as the companies above: rows on hairlines, a wash under the pointer. */
	.home-account-list {
		display: grid;
		margin: 0;
		padding: 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		list-style: none;
	}
	.home-account-list li + li {
		border-top: 1px solid var(--border);
	}
	.home-account-list a {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(80px, 160px) auto;
		align-items: center;
		gap: 16px;
		min-height: 44px;
		padding: 8px 10px;
		border-radius: var(--radius-control);
		color: inherit;
		text-decoration: none;
		transition: background var(--motion-state) var(--ease-standard);
	}
	.home-account-list a:hover {
		background: var(--wash-hover, var(--surface-alt));
	}
	.home-account-list a:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.home-account-list strong {
		overflow: hidden;
		font-size: var(--t-body);
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.home-account-list small {
		grid-column: -2;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
		text-align: right;
	}
	.home-account-list small.warning {
		color: var(--intent-authority);
	}
	.home-account-meter {
		display: block;
		min-width: 0;
	}
	@media (prefers-reduced-motion: reduce) {
		.home-account-list a {
			transition: none;
		}
	}
	@media (max-width: 560px) {
		.home-account-list a {
			grid-template-columns: minmax(0, 1fr) auto;
			gap: 6px 12px;
		}
		.home-account-meter {
			grid-column: 1 / -1;
			grid-row: 2;
		}
		.home-account-list small {
			grid-column: 2;
			grid-row: 1;
		}
	}
</style>
