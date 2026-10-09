<script lang="ts">
	/* Work: the company's goals and the work under them.
	 *
	 * The side panel lists goals only; choosing one narrows the list and choosing
	 * it again shows everything. Work without a goal is a group in the list, not
	 * a peer of the goals. List is the default view, grouped by what each item
	 * needs; Board is the second view. Both read the same rows. */
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { Item, Empty, Segmented, Dot, Fold } from '$lib/ui/page';
	import { WORK_STATUS_LABEL, runStateLabel } from '$lib/work/status';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import X from '@lucide/svelte/icons/x';
	import Kanban from '@lucide/svelte/icons/kanban';
	import Target from '@lucide/svelte/icons/target';
	import ListIcon from '@lucide/svelte/icons/list';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Plus from '@lucide/svelte/icons/plus';
	import CircleDashed from '@lucide/svelte/icons/circle-dashed';
	import ArchiveIcon from '@lucide/svelte/icons/archive';
	import SidebarShell from '$lib/ui/views/SidebarShell.svelte';
	import SidebarGroup from '$lib/ui/views/SidebarGroup.svelte';
	import SidebarRow from '$lib/ui/views/SidebarRow.svelte';
	import {
		attentionQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		companyPrincipalQuery
	} from '$lib/model/queries.svelte';
	import type { CollaborationWork } from '$lib/model/collaboration';
	import type { WorkRow } from '$lib/model/generated/orgintel';
	import WorkBoard from '$lib/ui/views/WorkBoard.svelte';
	import { askExec } from '$lib/model/ask-exec';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import Check from '@lucide/svelte/icons/check';
	import { STANDARDS, setGoalStandard, standardLabel } from '$lib/model/standards';
	import type { OutcomeStandard } from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const principalProjection = $derived(companyPrincipalQuery(companyId));
	const ownerAccess = $derived(principalProjection.view?.membership_role === 'owner');
	const attentionProjection = $derived(attentionQuery(companyId, () => ownerAccess));
	const cockpitProjection = $derived(cockpitQuery(companyId, () => ownerAccess));
	const collaborationProjection = $derived(
		collaborationBootstrapQuery(companyId, () => (ownerAccess ? null : principalProjection.view))
	);
	const attention = $derived(attentionProjection.view);
	const cockpit = $derived(cockpitProjection.view);
	const collaboration = $derived(collaborationProjection.view);
	const failure = $derived(
		principalProjection.failure ??
			(ownerAccess
				? (attentionProjection.failure ?? cockpitProjection.failure)
				: collaborationProjection.failure) ??
			null
	);
	function retryWork() {
		void principalProjection.refresh();
		if (ownerAccess) {
			void attentionProjection.reload();
			void cockpitProjection.refresh();
		} else void collaborationProjection.refresh();
	}
	const loaded = $derived(
		principalProjection.status !== 'unknown' &&
			(ownerAccess
				? attentionProjection.status !== 'unknown' && cockpitProjection.status !== 'unknown'
				: collaborationProjection.status !== 'unknown')
	);
	type WorkItem = WorkRow | CollaborationWork;

	/* The goal and view live in the address, so a filtered view can be shared,
	 * reloaded and reached again with Back. */
	const selectedGoal = $derived(page.url.searchParams.get('goal') ?? '');
	const view = $derived(page.url.searchParams.get('view') === 'board' ? 'board' : 'list');
	function setQuery(key: string, value: string) {
		const url = new URL(page.url);
		if (value) url.searchParams.set(key, value);
		else url.searchParams.delete(key);
		url.searchParams.delete('lens');
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}
	const chooseGoal = (id: string) => setQuery('goal', selectedGoal === id ? '' : id);

	const graph = $derived(
		ownerAccess ? (attention?.workGraph ?? null) : (collaboration?.work_graph ?? null)
	);
	const goals = $derived(ownerAccess ? (cockpit?.goals ?? []) : (collaboration?.goals ?? []));
	const openGoals = $derived(goals.filter((goal) => !goal.closed_at));
	const closedGoals = $derived(goals.filter((goal) => goal.closed_at));
	const archivedGoalIds = $derived(new Set(closedGoals.map((goal) => goal.id)));
	/* Archived lists its goals beneath it only while one of them, or Archived itself, is open. */
	const archivedOpen = $derived(selectedGoal === 'archived' || archivedGoalIds.has(selectedGoal));
	function viewHref(goal: string): string {
		const url = new URL(page.url);
		if (goal) url.searchParams.set('goal', goal);
		else url.searchParams.delete('goal');
		url.searchParams.delete('lens');
		return `${url.pathname}${url.search}`;
	}
	const proposeGoals = () =>
		askExec(
			'Suggest two or three Goals for the company from what you know so far, each with what done looks like.'
		);
	const people = $derived(ownerAccess ? (cockpit?.people ?? []) : (collaboration?.people ?? []));
	const noWorkYet = $derived(!!graph && graph.work.length === 0 && goals.length === 0);
	/* "other" is the Work that serves no Goal: a peer of the Goals, kept small. */
	const goalWork = $derived(
		(graph?.work ?? []).filter(
			(item) =>
				item.status !== 'abandoned' &&
				(!selectedGoal ||
					(selectedGoal === 'other'
						? !item.goal_id
						: selectedGoal === 'archived'
							? !!item.goal_id && archivedGoalIds.has(item.goal_id)
							: item.goal_id === selectedGoal))
		)
	);
	const otherOpen = $derived(
		(graph?.work ?? []).filter(
			(item) => !item.goal_id && item.status !== 'abandoned' && item.status !== 'completed'
		).length
	);
	const finishLine = (goal: object) =>
		'done_when' in goal && typeof goal.done_when === 'string' ? goal.done_when.trim() : '';
	const dueLabel = (goal: object) =>
		'due_on' in goal && typeof goal.due_on === 'string' && goal.due_on
			? new Date(`${goal.due_on}T00:00:00`).toLocaleDateString(undefined, {
					day: 'numeric',
					month: 'short'
				})
			: '';
	/* Work with a decision or step waiting on the owner. */
	const waitingOnOwner = $derived(
		new Set(
			(graph && 'handoffs' in graph ? graph.handoffs : [])
				.filter((handoff) => handoff.state === 'pending')
				.map((handoff) => handoff.work_id)
		)
	);
	const byRecent = (a: WorkItem, b: WorkItem) =>
		Date.parse(b.updated_at) - Date.parse(a.updated_at);
	const groups = $derived([
		{
			key: 'owner',
			label: 'Needs you',
			tone: 'warning' as const,
			rows: goalWork.filter((item) => item.status !== 'completed' && waitingOnOwner.has(item.id))
		},
		...(['blocked', 'active', 'proposed'] as const).map((status) => ({
			key: status,
			label: WORK_STATUS_LABEL[status],
			tone: (status === 'blocked' ? 'danger' : status === 'active' ? 'progress' : 'muted') as
				'danger' | 'progress' | 'muted',
			rows: goalWork
				.filter((item) => item.status === status && !waitingOnOwner.has(item.id))
				.toSorted(byRecent)
		}))
	]);
	const done = $derived(goalWork.filter((item) => item.status === 'completed').toSorted(byRecent));
	const openCount = $derived(goalWork.length - done.length);
	const title = $derived(
		selectedGoal === 'other'
			? 'Other work'
			: selectedGoal === 'archived'
				? 'Archived'
				: (goals.find((goal) => goal.id === selectedGoal)?.title ?? 'All work')
	);
	/* The quality bar lives on the Goal; a piece of Work shows it only when it
	 * holds itself to a different one. */
	const goalStandard = (goal: object | undefined): OutcomeStandard | undefined =>
		goal && 'outcome_standard' in goal
			? (goal as { outcome_standard: OutcomeStandard }).outcome_standard
			: undefined;
	const selectedGoalRow = $derived(goals.find((goal) => goal.id === selectedGoal));
	const overrides = $derived(
		new Map(
			((graph && 'standards' in graph ? graph.standards : undefined) ?? []).map((row) => [
				row.work_id,
				row.outcome_standard
			])
		)
	);
	let standardBusy = $state(false);
	let standardFailure = $state('');
	async function chooseGoalStandard(standard: OutcomeStandard) {
		if (!selectedGoal || standardBusy) return;
		standardBusy = true;
		standardFailure = '';
		try {
			await setGoalStandard(companyId, selectedGoal, standard);
			await cockpitProjection.refresh();
		} catch (cause) {
			standardFailure = failureSentence(cause, 'The quality bar did not change.');
		} finally {
			standardBusy = false;
		}
	}

	function attemptOf(work: WorkItem) {
		return (
			graph?.attempts
				.filter((attempt) => attempt.work_id === work.id)
				.toSorted(
					(a, b) =>
						a.revision - b.revision ||
						a.attempt_no - b.attempt_no ||
						Date.parse(a.started_at) - Date.parse(b.started_at)
				)
				.at(-1) ?? null
		);
	}
	function outputs(work: WorkItem): number {
		return graph?.artifacts.filter((artifact) => artifact.work_id === work.id).length ?? 0;
	}
	function goalProgress(goalId: string): { done: number; total: number } {
		const rows = (graph?.work ?? []).filter(
			(item) => item.goal_id === goalId && item.status !== 'abandoned'
		);
		return { done: rows.filter((item) => item.status === 'completed').length, total: rows.length };
	}
	function ownerName(actorId: string): string {
		return (
			people.find((person) => person.actor_id === actorId)?.display ??
			actorId.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
		);
	}
	function workHref(workId: string): string {
		const query = new URLSearchParams();
		if (selectedGoal) query.set('goal', selectedGoal);
		if (view === 'board') query.set('view', 'board');
		const search = query.toString();
		return `/${encodeURIComponent(companyId)}/work/${encodeURIComponent(workId)}${search ? `?${search}` : ''}`;
	}
	function meta(work: WorkItem, withOwner = true): string {
		const run = runStateLabel(attemptOf(work)?.state);
		const count = outputs(work);
		const own = overrides.get(work.id);
		return [
			withOwner ? ownerName(work.owner_id) : '',
			own ? `${standardLabel(own)} bar` : '',
			run && run !== 'running' ? run : '',
			count ? `${count} ${count === 1 ? 'output' : 'outputs'}` : ''
		]
			.filter(Boolean)
			.join(' · ');
	}
	const boardColumns = $derived(
		(['proposed', 'active', 'blocked', 'completed'] as const).map((status) => ({
			key: status,
			label: WORK_STATUS_LABEL[status],
			items: goalWork
				.filter((item) => item.status === status)
				.toSorted(byRecent)
				.map((row) => ({
					id: row.id,
					title: row.title,
					signal: meta(row, false),
					status: row.status,
					ownerName: ownerName(row.owner_id),
					revision: row.revision,
					href: workHref(row.id)
				}))
		}))
	);
</script>

<CompanyTitle title="Work" {companyId} />

{#snippet goalRow(
	goal: {
		id: string;
		title: string;
		closed_at?: string | null;
	},
	indent = false
)}
	{@const progress = goalProgress(goal.id)}
	<SidebarRow
		onclick={() => chooseGoal(goal.id)}
		label={goal.title}
		active={selectedGoal === goal.id}
		{indent}
		title={[
			finishLine(goal)
				? `Done when ${finishLine(goal)}`
				: 'No finish line yet: Exec will agree one with you',
			`${progress.done} of ${progress.total} done`,
			dueLabel(goal) ? `due ${dueLabel(goal)}` : '',
			`${standardLabel(goalStandard(goal))} quality bar`
		]
			.filter(Boolean)
			.join(' · ')}
	>
		{#snippet leading()}<span
				class="ring"
				style:--done={`${progress.total ? Math.round((progress.done / progress.total) * 100) : 0}%`}
				aria-label={`${progress.done} of ${progress.total} done`}
				role="img"
			></span>{/snippet}
	</SidebarRow>
{/snippet}

{#snippet row(work: WorkItem, tone: 'warning' | 'danger' | 'progress' | 'muted' | 'success')}
	<Item
		title={work.title}
		meta={meta(work)}
		href={workHref(work.id)}
		dim={work.status === 'completed'}
	>
		{#snippet leading()}<Dot
				{tone}
				label={waitingOnOwner.has(work.id) ? 'Needs you' : WORK_STATUS_LABEL[work.status]}
			/>{/snippet}
		{#snippet trailing()}<RelativeTime value={work.updated_at} />{/snippet}
	</Item>
{/snippet}

<SidebarShell label="Goals">
	{#snippet nav()}
		<SidebarRow
			href={viewHref('')}
			label="All work"
			icon={ListIcon}
			active={!selectedGoal}
			count={loaded
				? (graph?.work ?? []).filter(
						(item) => item.status !== 'abandoned' && item.status !== 'completed'
					).length
				: null}
		/>
		{#if loaded && graph}
			<SidebarGroup
				label="Goals"
				help="Goals are the outcomes you're working toward, like “3 paying clients by November”. Exec proposes them as you talk and keeps the work under them."
				action={ownerAccess && openGoals.length
					? { label: 'Ask Exec to propose goals', icon: Plus, onclick: proposeGoals }
					: undefined}
			>
				{#each openGoals as goal (goal.id)}{@render goalRow(goal)}{:else}
					{#if ownerAccess}<SidebarRow
							onclick={proposeGoals}
							label="Propose goals"
							icon={Sparkles}
							title="Ask Exec to suggest two or three goals, each with what done looks like"
						/>{/if}
				{/each}
				{#if otherOpen}<SidebarRow
						onclick={() => chooseGoal('other')}
						label="Other work"
						icon={CircleDashed}
						active={selectedGoal === 'other'}
						count={otherOpen}
						title="Work that serves no Goal. Exec keeps this small and proposes a Goal when a theme repeats."
					/>{/if}
			</SidebarGroup>
		{:else if !loaded && !failure}
			<Skeleton label="Loading goals" variant="list" count={4} />
		{/if}
	{/snippet}
	{#snippet foot()}
		<SidebarRow
			href={viewHref(selectedGoal === 'archived' ? '' : 'archived')}
			label="Archived"
			icon={ArchiveIcon}
			active={selectedGoal === 'archived'}
			count={closedGoals.length || null}
			title="Goals that are closed, and their work"
		/>
		{#if archivedOpen}{#each closedGoals as goal (goal.id)}{@render goalRow(goal, true)}{/each}{/if}
	{/snippet}

	<div class="page">
		<header class="head">
			{#if selectedGoal}<Target size={15} strokeWidth={1.8} aria-hidden="true" />{/if}
			<h1>{title}</h1>
			<span class="count" title="Open work">{loaded ? openCount : ''}</span>
			{#if selectedGoal}<button
					class="clear"
					type="button"
					aria-label="Show all work"
					title="Show all work"
					onclick={() => setQuery('goal', '')}><X size={14} strokeWidth={2} /></button
				>{/if}
			{#if selectedGoalRow && ownerAccess}
				<div
					class="goal-standard-picker"
					title="How far this Goal's Work should go before it is called done"
				>
					<ActionMenu label="Quality bar">
						{#snippet trigger()}<span>{standardLabel(goalStandard(selectedGoalRow))}</span
							>{/snippet}
						{#each STANDARDS as option (option.value)}
							<button
								type="button"
								title={option.title}
								disabled={standardBusy}
								onclick={() => chooseGoalStandard(option.value)}
								><span>{option.label}</span
								>{#if goalStandard(selectedGoalRow) === option.value}<Check
										size={14}
										aria-label="Current"
									/>{/if}</button
							>
						{/each}
					</ActionMenu>
				</div>
				{#if standardFailure}<span class="standard-failure" role="alert">{standardFailure}</span
					>{/if}
			{/if}
			<span class="spacer"></span>
			<!-- With no goals there is nothing to pick, and "All work" is already the title. -->
			{#if goals.length}<select
					class="goal-picker"
					aria-label="Goal"
					value={selectedGoal}
					onchange={(event) => setQuery('goal', event.currentTarget.value)}
				>
					<option value="">All work</option>
					{#each goals as goal (goal.id)}<option value={goal.id}>{goal.title}</option>{/each}
				</select>{/if}
			<Segmented
				label="View"
				value={view}
				options={[
					{ value: 'list', label: 'List' },
					{ value: 'board', label: 'Board' }
				]}
				onchange={(value) => setQuery('view', value === 'board' ? 'board' : '')}
			/>
		</header>

		<!-- Board padding only for the board itself: empty, loading and failure sit where List puts them. -->
		<div class="body" class:board={view === 'board' && loaded && !!graph && !noWorkYet}>
			{#if failure && graph}<FailureNotice
					error={failure}
					subject="Work"
					stale
					onretry={retryWork}
				/>{/if}
			{#if !graph && failure}
				<FailureNotice error={failure} subject="Work" variant="page" onretry={retryWork} />
			{:else if !loaded}
				<Skeleton label="Loading work" variant="list" count={6} />
			{:else if noWorkYet}
				<Empty
					page
					icon={Kanban}
					title="No work yet"
					info="Tell Exec what you want. It turns that into work for the team, and it shows up here as it moves."
				>
					{#snippet action()}{#if ownerAccess}<button
								class="btn small primary"
								type="button"
								onclick={askExec}>Tell Exec what you want</button
							>{/if}{/snippet}
				</Empty>
			{:else if view === 'board'}
				<WorkBoard columns={boardColumns} />
			{:else}
				{#each groups.filter((group) => group.rows.length) as group (group.key)}
					<section class="group" aria-label={group.label}>
						<h3>{group.label}<span>{group.rows.length}</span></h3>
						<div class="rows">
							{#each group.rows as work (work.id)}{@render row(work, group.tone)}{/each}
						</div>
					</section>
				{/each}
				{#if done.length}
					<section class="group" aria-label="Done">
						<div class="rows">
							<Fold label="Done" count={done.length}>
								{#each done as work (work.id)}{@render row(work, 'success')}{/each}
							</Fold>
						</div>
					</section>
				{/if}
				{#if !goalWork.length}
					<Empty compact title={selectedGoal ? 'No work under this goal yet' : 'No work yet'} />
				{/if}
			{/if}
		</div>
	</div>
</SidebarShell>

<style>
	.goal-standard-picker :global(summary) {
		width: auto;
		height: 26px;
		padding: 0 8px;
		border: 1px solid var(--border);
		border-radius: 999px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.goal-standard-picker :global(.action-menu-panel button) {
		justify-content: space-between;
	}
	.standard-failure {
		color: var(--state-danger);
		font-size: var(--t-label);
	}
	.page {
		container: page / inline-size;
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		width: 100%;
		min-height: 0;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 52px;
		padding: 8px 12px 8px 20px;
		border-bottom: 1px solid var(--border);
	}
	.head :global(svg) {
		flex: none;
		color: var(--text-tertiary);
	}
	h1 {
		min-width: 0;
		margin: 0;
		overflow: hidden;
		font-size: var(--t-head);
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.count {
		color: var(--text-tertiary);
		font-variant-numeric: tabular-nums;
	}
	.clear {
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.clear:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.spacer {
		flex: 1;
	}
	.head .goal-picker {
		display: none;
		max-width: 160px;
	}
	.body {
		min-height: 0;
		overflow: auto;
		padding: 12px 12px 24px;
	}
	.body.board {
		padding: 0;
	}
	.group + .group {
		margin-top: 18px;
	}
	h3 {
		display: flex;
		align-items: baseline;
		gap: 8px;
		margin: 0 4px 8px;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 600;
	}
	h3 span {
		color: var(--text-tertiary);
		font-weight: 400;
		font-variant-numeric: tabular-nums;
	}
	.rows {
		overflow: hidden;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.rows > :global(* + *) {
		border-top: 1px solid var(--border);
	}
	@container sidebar (max-width: 760px) {
		.head .goal-picker {
			display: block;
		}
	}
	/* A goal's progress: a small ring, filled as its work completes. */
	.ring {
		flex: none;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: conic-gradient(var(--state-success) var(--done), var(--border-strong) 0);
		mask: radial-gradient(circle, transparent 3.5px, #000 4px);
	}
</style>
