<script lang="ts">
	/* Company's overview: a line of facts, then what waits on the owner. The section list beside it
	 * carries every area's value; where it steps out (a narrow pane, a phone) the overview lists the
	 * sections itself. A value that could not be read says so rather than guessing. */
	import { formatMoney } from '$lib/ui/time';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item } from '$lib/ui/page';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import SidebarSearch from '$lib/ui/views/SidebarSearch.svelte';
	import { useCompanySetup } from '$lib/model/company-setup-rows.svelte';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Download from '@lucide/svelte/icons/download';
	import Archive from '@lucide/svelte/icons/archive';
	import { goto } from '$app/navigation';
	import { archiveCompany, companyExportHref } from '$lib/model/cockpit';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import CompanyMark from '$lib/ui/glyph/CompanyMark.svelte';

	const setup = useCompanySetup();
	const companyId = $derived(setup.companyId);
	// A bounded excerpt of the first body paragraph; the full authored charter stays one click away.
	const purpose = $derived(
		(setup.entry?.mission ?? '')
			.split('\n')
			.filter((line) => !/^\s*#{1,6}\s/.test(line))
			.join('\n')
			.trim()
			.split(/\n\s*\n/)[0] ?? ''
	);

	const plural = (count: number, one: string, many = `${one}s`) =>
		`${count} ${count === 1 ? one : many}`;
	const money = formatMoney;
	const spendLine = $derived.by(() => {
		const spend = setup.spend;
		if (!spend) return '';
		/* A running total, or this month's spend against a monthly budget (Limits sets which). Spend
		 * some calls could not price is said in the tip, not as "At least" before the figure. */
		const budget = spend.period === 'monthly' ? 'budget this month' : 'model budget';
		return `${money(spend.accounted_usd)} spent of ${money(spend.ceiling_usd)} ${budget}`;
	});
	const spendWhy = $derived.by(() => {
		const spend = setup.spend;
		const against =
			spend?.period === 'monthly'
				? "Model spend this month (UTC) against the company's monthly budget."
				: "Model spend so far against the company's total budget, a running total.";
		return spend?.status === 'metering_unknown'
			? `${against} A few model calls reported no price, so the real figure may be a little higher.`
			: against;
	});
	const working = $derived(setup.working);

	let archiving = $state(false);
	async function archive() {
		if (archiving) return;
		const name = setup.entry?.name ?? 'this company';
		if (
			!(await confirmAction({
				title: `Archive ${name}?`,
				body: 'Exec, Staff and schedules stop, and its computer is shut down. Files, history and settings are kept, and you can restore it at any time from your companies.',
				action: 'Archive company'
			}))
		)
			return;
		archiving = true;
		try {
			await archiveCompany(companyId);
			toast(`${name} archived`);
			await goto('/');
		} catch (cause) {
			toast(failureSentence(cause, 'Company could not be archived.'), { tone: 'failure' });
		} finally {
			archiving = false;
		}
	}
</script>

<CompanyTitle title="Company" {companyId} />

<Page
	title={setup.entry?.name ?? 'Company'}
	info="How this company is set up. The sections beside it show each area's current value."
>
	{#snippet leading()}<span
			class="company-mark"
			class:logo={!!setup.entry?.logo_url}
			aria-hidden="true"
			><CompanyMark id={companyId} logoUrl={setup.entry?.logo_url} size={12} /></span
		>{/snippet}
	{#snippet actions()}{#if setup.owner}<a class="btn small" href={`${setup.href('charter')}#name`}
				>Name & logo</a
			>{/if}{/snippet}
	<!-- The overview appears once its sources have answered: the facts line and the setup tasks
	     arriving a read at a time pushed everything below them down (S109 C7). -->
	{#if !setup.settled}
		<Skeleton label="Loading the company overview" variant="page" />
	{:else}
		{#if purpose}<div class="company-purpose">
				<div class="company-purpose-copy"><Markdown text={purpose} /></div>
				<a href={setup.href('charter')}>Charter <ChevronRight size={14} aria-hidden="true" /></a>
			</div>{/if}
		{#if setup.owner}
			<p class="company-facts">
				<a
					href={setup.href('health')}
					class:alert={setup.health && setup.health !== 'healthy'}
					title="Checks on the company computer, model and tools"
					><i aria-hidden="true" class="dot {setup.health ?? 'unknown'}"></i>{setup.health ===
					'healthy'
						? 'Healthy'
						: setup.health === 'unavailable'
							? 'Health unavailable'
							: setup.health
								? 'Needs attention'
								: 'Checking…'}</a
				>
				{#if spendLine}<span aria-hidden="true">·</span><a href={setup.href('limits')}
						>{spendLine}</a
					><InfoTip text={spendWhy} />{/if}
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
				<div class="company-current">
					{#if setup.health === 'unavailable' || setup.rows.some((row) => row.unavailable)}
						<a class="current-action" href={setup.href('health')}
							><span
								><strong>Company details unavailable</strong><span
									>Open Health to check the connection and try again.</span
								></span
							><ChevronRight size={16} aria-hidden="true" /></a
						>
					{:else if setup.health && setup.health !== 'healthy'}
						<a class="current-action" href={setup.href('health')}
							><span
								><strong>Review company health</strong><span>A current check needs attention.</span
								></span
							><ChevronRight size={16} aria-hidden="true" /></a
						>
					{:else if setup.entry?.card?.decisions_waiting}
						<a class="current-action" href={`/${encodeURIComponent(companyId)}`}
							><span
								><strong>{plural(setup.entry.card.decisions_waiting, 'decision')} waiting</strong
								><span>Review what the company has prepared for you.</span></span
							><ChevronRight size={16} aria-hidden="true" /></a
						>
					{:else if setup.health === 'healthy'}
						<a class="current-action" href={`/${encodeURIComponent(companyId)}/work`}
							><span
								><strong>No setup tasks outstanding</strong><span
									>{working
										? `${plural(working, 'person', 'people')} working. View current outcomes.`
										: 'View the company’s current work and outcomes.'}</span
								></span
							><ChevronRight size={16} aria-hidden="true" /></a
						>
					{:else}<p class="company-done">Checking the company's current state…</p>{/if}
				</div>
			{/if}
		{/if}

		<div class="setup-all">
			<!-- Where the section list steps out, its search comes here. -->
			<SidebarSearch bind:value={setup.query} placeholder="Search settings" />
			{#each setup.matching as group (group.label)}
				<Section title={group.label} group>
					{#each group.rows as row (row.key)}
						{@const Icon = row.icon}
						<!-- The value takes a line of its own under the name, reserved from the first frame:
						     beside the chevron it arrived late and widened the row, cutting the name
						     short on a phone (S109 C7). Here neither moves and neither is cut. -->
						<Item
							title={row.label}
							href={row.href}
							note={row.valued ? (row.reading ?? '') : undefined}
						>
							{#snippet leading()}<Icon size={15} strokeWidth={1.8} aria-hidden="true" />{/snippet}
							{#snippet trailing()}<ChevronRight
									size={14}
									strokeWidth={1.8}
									aria-hidden="true"
								/>{/snippet}
						</Item>
					{/each}
				</Section>
			{:else}
				<p class="company-done">No settings match.</p>
			{/each}
		</div>

		{#if setup.owner}
			<Section
				title="Export and archive"
				info="Take everything the company holds with you, or stop it for now. Archived companies can be restored, opened read-only or deleted from your companies."
				group
			>
				<Item title="Download export" href={companyExportHref(companyId)} download>
					{#snippet leading()}<Download size={15} strokeWidth={1.8} aria-hidden="true" />{/snippet}
				</Item>
				<Item title={archiving ? 'Archiving…' : 'Archive company'} onclick={archive}>
					{#snippet leading()}<Archive size={15} strokeWidth={1.8} aria-hidden="true" />{/snippet}
				</Item>
			</Section>
		{/if}
	{/if}
</Page>

<style>
	/* The company's own mark beside its name, as on its account card and in the top bar. */
	.company-mark {
		display: grid;
		flex: none;
		place-items: center;
		width: 28px;
		height: 28px;
		overflow: hidden;
		border-radius: 7px;
		background: var(--accent-soft);
		color: var(--accent-strong);
	}
	.company-mark.logo {
		background: var(--surface);
	}
	.company-purpose {
		display: grid;
		gap: 8px;
		padding-block: 4px 8px;
	}
	.company-purpose-copy {
		max-width: 64ch;
		margin: 0;
		font-size: var(--t-body);
		line-height: 1.65;
		color: var(--text-secondary);
		white-space: pre-line;
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.company-purpose > a {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		width: fit-content;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.company-current {
		border-block: 1px solid var(--border);
		padding-block: 4px;
	}
	.current-action {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 16px 0;
		color: var(--ink);
		text-decoration: none;
	}
	.current-action > span {
		display: grid;
		gap: 5px;
	}
	.current-action strong {
		font-size: var(--t-body);
		font-weight: 600;
	}
	.current-action span span {
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.current-action:hover strong {
		text-decoration: underline;
		text-underline-offset: 3px;
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
		background: var(--state-success);
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
