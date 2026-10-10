<script lang="ts">
	/* Company's overview: a line of facts, then what waits on the owner. The section list beside it
	 * carries every area's value; where it steps out (a narrow pane, a phone) the overview lists the
	 * sections itself. A value that could not be read says so rather than guessing. */
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item } from '$lib/ui/page';
	import { useCompanySetup } from '$lib/model/company-setup-rows.svelte';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';

	const setup = useCompanySetup();
	const companyId = $derived(setup.companyId);

	const plural = (count: number, one: string, many = `${one}s`) =>
		`${count} ${count === 1 ? one : many}`;
	const money = (usd: number) =>
		usd.toLocaleString(undefined, { style: 'currency', currency: 'USD', maximumFractionDigits: 2 });
	const spendLine = $derived.by(() => {
		const spend = setup.spend;
		if (!spend) return '';
		return spend.status === 'metering_unknown'
			? 'Spend not metered'
			: `${money(spend.accounted_usd)} of ${money(spend.ceiling_usd)} this month`;
	});
	const working = $derived(setup.working);
</script>

<CompanyTitle title="Company" {companyId} />

<Page
	title={setup.entry?.name ?? 'Company'}
	info="How this company is set up. The sections beside it show each area's current value."
>
	{#if setup.owner}
		<p class="company-facts">
			<a
				href={setup.href('health')}
				class:alert={setup.health && setup.health !== 'healthy'}
				title="Checks on the company computer, model and tools"
				><i aria-hidden="true" class="dot {setup.health ?? 'unknown'}"></i>{setup.health ===
				'healthy'
					? 'Healthy'
					: setup.health
						? 'Needs attention'
						: 'Checking…'}</a
			>
			{#if spendLine}<span aria-hidden="true">·</span><a
					href={setup.href('limits')}
					title="Model spend this month against the company's ceiling">{spendLine}</a
				>{/if}
			{#if working != null}<span aria-hidden="true">·</span><a
					href={`/${companyId}/people`}
					title="People working for this company right now"
					>{working ? `${plural(working, 'person', 'people')} working` : 'Nobody working now'}</a
				>{/if}
		</p>

		{#if setup.todos.length}
			<Section title={`${setup.todos.length} to finish setting up`} group>
				{#each setup.todos as row (row.key)}
					{@const Icon = row.icon}
					<!-- The task says what to do; the section beside it carries the current value. -->
					<Item title={row.task ?? row.label} href={row.href}>
						{#snippet leading()}<Icon size={15} strokeWidth={1.8} aria-hidden="true" />{/snippet}
						{#snippet trailing()}<ChevronRight
								size={14}
								strokeWidth={1.8}
								aria-label={row.value}
							/>{/snippet}
					</Item>
				{/each}
			</Section>
		{:else}
			<p class="company-done">Everything is set up.</p>
		{/if}
	{/if}

	<div class="setup-all">
		{#each setup.sections as group (group.label)}
			<Section title={group.label} group>
				{#each group.rows as row (row.key)}
					{@const Icon = row.icon}
					<Item title={row.label} href={row.href}>
						{#snippet leading()}<Icon size={15} strokeWidth={1.8} aria-hidden="true" />{/snippet}
						{#snippet trailing()}{#if row.reading}<span class="row-value" title={row.tooltip}
									>{row.reading}</span
								>{/if}<ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />{/snippet}
					</Item>
				{/each}
			</Section>
		{/each}
	</div>
</Page>

<style>
	/* The current value sits beside the chevron, so it survives on phones where row meta hides. */
	.row-value {
		max-width: min(46ch, 44vw);
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* Facts as a line of plain text, each a link with its explanation on hover. */
	.company-facts {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px 8px;
		margin: 0 0 4px;
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
	.company-facts a {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--text-secondary);
		text-decoration: none;
	}
	.company-facts a:hover {
		color: var(--ink);
		text-decoration: underline;
		text-underline-offset: 3px;
	}
	.company-facts a.alert {
		color: var(--ink);
	}
	.company-done {
		margin: 8px 0 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--text-tertiary);
	}
	.dot.healthy {
		background: var(--state-success, var(--intent-presence));
	}
	.dot.degraded,
	.dot.unavailable {
		background: var(--state-danger);
	}
	/* Beside the section list, the overview need not repeat it. */
	@container sidebar (min-width: 761px) {
		.setup-all {
			display: none;
		}
	}
</style>
