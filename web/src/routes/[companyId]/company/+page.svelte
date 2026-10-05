<script lang="ts">
	/* Company is an overview: a status line, then one row per area with its current value. Each row
	 * opens the area's own page; a value that could not be read says so rather than guessing. */
	import { page } from '$app/state';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item } from '$lib/ui/page';
	import { formatRelative } from '$lib/ui/time';
	import {
		companiesQuery,
		companyPrincipalQuery,
		companyQuery,
		identityQuery
	} from '$lib/model/queries.svelte';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { fetchConnections } from '$lib/model/connections';
	import { fetchSkillLibrary, monitorSchedules } from '$lib/model/skills';
	import { getCoreMembers } from '$lib/model/members';
	import { companyPageHref, COMPANY_PAGES } from '$lib/model/company-pages';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import Brain from '@lucide/svelte/icons/brain';
	import CalendarClock from '@lucide/svelte/icons/calendar-clock';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Fingerprint from '@lucide/svelte/icons/fingerprint';
	import Gauge from '@lucide/svelte/icons/gauge';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import Monitor from '@lucide/svelte/icons/monitor';
	import Plug from '@lucide/svelte/icons/plug';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Users from '@lucide/svelte/icons/users';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const principal = $derived(companyPrincipalQuery(companyId).view);
	const owner = $derived(principal?.membership_role === 'owner');
	const catalog = companiesQuery();
	const entry = $derived(catalog.view.find((company) => company.id === companyId));
	const source = $derived(companyQuery(companyId));
	$effect(() => {
		if (owner) source.attach();
	});
	const view = $derived(owner ? source.view : null);
	const identity = $derived(identityQuery(companyId));
	const intelligence = $derived(intelligenceQuery(companyId, () => owner));

	/* The small counts that have no shared query: read once per visit, each on its own. */
	type Count = number | 'unavailable' | null;
	let tools = $state<Count>(null);
	let skills = $state<Count>(null);
	let schedules = $state<Count>(null);
	let secrets = $state<Count>(null);
	let members = $state<Count>(null);
	$effect(() => {
		const company = companyId;
		const read = (load: () => Promise<number>, set: (value: Count) => void) =>
			void load().then(set, () => set('unavailable'));
		read(
			async () => (await getCoreMembers(company)).members.length,
			(value) => (members = value)
		);
		if (!owner) return;
		read(
			async () =>
				(await fetchConnections(company)).filter((row) => row.status === 'working').length,
			(value) => (tools = value)
		);
		read(
			async () =>
				(await fetchSkillLibrary(company)).skills.filter((row) => row.disposition === 'accepted')
					.length,
			(value) => (skills = value)
		);
		read(
			async () =>
				(await monitorSchedules(company)).filter(
					(row) => !row.schedule.cancelled_at && !row.schedule.paused_at
				).length,
			(value) => (schedules = value)
		);
		read(
			async () => {
				const response = await fetch(`/api/companies/${encodeURIComponent(company)}/vault`, {
					cache: 'no-store'
				});
				if (!response.ok) throw new Error('vault');
				const body: { references?: unknown[] } = await response.json();
				return body.references?.length ?? 0;
			},
			(value) => (secrets = value)
		);
	});

	const plural = (count: number, one: string, many = `${one}s`) =>
		`${count} ${count === 1 ? one : many}`;
	function counted(value: Count, one: string, none: string, many?: string): string {
		if (value === null) return '…';
		if (value === 'unavailable') return 'Unavailable right now';
		return value ? plural(value, one, many) : none;
	}
	const money = (usd: number) =>
		usd.toLocaleString(undefined, { style: 'currency', currency: 'USD', maximumFractionDigits: 2 });

	const model = $derived.by(() => {
		if (entry?.unstartable_reason) return 'Needs a model connection';
		const value = intelligence.view;
		if (intelligence.error) return 'Unavailable right now';
		if (!value) return '…';
		const exec = value.agents.find((agent) => agent.role === 'exec') ?? value.agents[0];
		const chosen = value.default?.model ?? exec?.effective_model;
		return chosen ? chosen.split('/').pop()! : 'Not chosen yet';
	});
	const charter = $derived.by(() => {
		if (!view) return source.failure ? 'Unavailable right now' : '…';
		if (!view.charter.purpose.trim()) return 'Not written yet';
		return (
			view.charter.current_direction?.title ??
			`Saved ${formatRelative(view.charter.effective_at, 'earlier')}`
		);
	});
	const identityValue = $derived.by(() => {
		const value = identity.view;
		if (identity.failure) return 'Unavailable right now';
		if (!value) return '…';
		if (value.pending_proposals.length)
			return `${plural(value.pending_proposals.length, 'proposal')} waiting`;
		return value.current_release ? 'Published' : 'Not set yet';
	});
	const limits = $derived.by(() => {
		if (!view) return source.failure ? 'Unavailable right now' : '…';
		const asks = view.limits.asks_owner.map((limit) => limit.title.toLowerCase());
		return asks.length
			? `Asks you before ${asks.slice(0, 2).join(', ')}${asks.length > 2 ? '…' : ''}`
			: 'Set what it may do alone';
	});
	const computer = $derived.by(() => {
		if (!view) return source.failure ? 'Unavailable right now' : '…';
		const runtime = view.limits.runtime;
		if (runtime.asleep) return 'Asleep · wakes when work is owed';
		return runtime.sleep_after_minutes
			? `Running · sleeps after ${runtime.sleep_after_minutes} minutes idle`
			: 'Running · never sleeps';
	});

	/* The status line: what the owner would otherwise open four pages to learn. */
	const health = $derived(view?.computer.doctor.status);
	const spend = $derived(view?.limits.spend);
	const working = $derived(entry?.card?.people_working);
	const href = (key: string) => {
		const target = COMPANY_PAGES.find((candidate) => candidate.key === key);
		return target ? companyPageHref(companyId, target) : `/${companyId}/company`;
	};

	const rows = $derived(
		[
			{ key: 'charter', label: 'Charter', value: charter, icon: BookOpen },
			{ key: 'identity', label: 'Identity', value: identityValue, icon: Fingerprint },
			{ key: 'provider', label: 'Intelligence', value: model, icon: Brain },
			{
				key: 'connections',
				label: 'Connections',
				value: counted(tools, 'tool connected', 'No tools connected', 'tools connected'),
				icon: Plug
			},
			{
				key: 'skills',
				label: 'Skills',
				value: counted(skills, 'skill', 'No skills yet'),
				icon: Sparkles
			},
			{
				key: 'vault',
				label: 'Vault',
				value: counted(secrets, 'secret', 'No secrets stored'),
				icon: KeyRound
			},
			{
				key: 'schedules',
				label: 'Schedules',
				value: counted(schedules, 'active schedule', 'Nothing scheduled'),
				icon: CalendarClock
			},
			{ key: 'limits', label: 'Limits', value: limits, icon: Gauge },
			{
				key: 'members',
				label: 'Members',
				value: counted(members, 'person', 'Only you', 'people'),
				icon: Users
			},
			{ key: 'computer', label: 'Computer', value: computer, icon: Monitor }
		].filter((row) => owner || row.key === 'members')
	);
</script>

<CompanyTitle title="Company" {companyId} />

<Page
	title={entry?.name ?? 'Company'}
	info="How this company is set up. Each row shows its current value and opens its own page."
>
	{#if owner}
		<nav class="company-status" aria-label="Company status">
			<a
				href={href('health')}
				class:alert={health && health !== 'healthy'}
				title="Checks on the company computer, model and tools"
				><i aria-hidden="true" class="dot {health ?? 'unknown'}"></i>{health === 'healthy'
					? 'Healthy'
					: health
						? 'Needs attention'
						: 'Checking…'}</a
			>
			<a href={href('limits')} title="Model spend this month against the company's ceiling"
				>{spend
					? spend.status === 'metering_unknown'
						? 'Spend not metered'
						: `${money(spend.accounted_usd)} of ${money(spend.ceiling_usd)} this month`
					: 'Spend …'}</a
			>
			<a href={`/${companyId}/people`} title="People working for this company right now"
				>{working == null
					? 'People …'
					: working
						? `${plural(working, 'person', 'people')} working`
						: 'Nobody working now'}</a
			>
			<a href={href('activity')} title="Decisions, receipts and external actions">Activity</a>
		</nav>
	{/if}

	<Section title="Setup" group>
		{#each rows as row (row.key)}
			{@const Icon = row.icon}
			<Item title={row.label} href={href(row.key)}>
				{#snippet leading()}<Icon size={15} strokeWidth={1.8} aria-hidden="true" />{/snippet}
				{#snippet trailing()}<span class="row-value" title={row.value}>{row.value}</span
					><ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />{/snippet}
			</Item>
		{/each}
	</Section>
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
	.company-status {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-bottom: 4px;
	}
	.company-status a {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		min-height: 30px;
		padding: 0 12px;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: var(--surface-raised);
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-decoration: none;
		transition:
			border-color var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.company-status a:hover {
		border-color: var(--border-strong);
		color: var(--ink);
	}
	.company-status a.alert {
		border-color: color-mix(in srgb, var(--state-danger) 40%, var(--border));
		color: var(--ink);
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
</style>
