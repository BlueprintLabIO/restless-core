<script lang="ts">
	import Dialog from '$lib/ui/controls/Dialog.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import ResizableSidebar from '$lib/components/ResizableSidebar.svelte';
	/* Work: the company's goals and the work under them.
	 *
	 * The side panel lists goals as links; choosing one narrows the list and
	 * "All work" shows everything. Work without a goal is "No goal" here and on
	 * every Work page. List is the default view, grouped by what each item
	 * needs; Board is the second view. Both read the same rows. */
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { Item, Empty, Segmented, Dot, Fold } from '$lib/ui/page';
	import { WORK_STATUS_LABEL, runStateLabel } from '$lib/work/status';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { untrack } from 'svelte';
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
	import SidebarSearch from '$lib/ui/views/SidebarSearch.svelte';
	import SidebarEmpty from '$lib/ui/views/SidebarEmpty.svelte';
	import type { SidebarTone } from '$lib/ui/views/SidebarRow.svelte';
	import InboxIcon from '@lucide/svelte/icons/inbox';
	import Activity from '@lucide/svelte/icons/activity';
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
	import CopyButton from '$lib/ui/controls/CopyButton.svelte';
	import Check from '@lucide/svelte/icons/check';
	import { STANDARDS, setGoalStandard, standardLabel } from '$lib/model/standards';
	import type { OutcomeStandard } from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';
	import {
		coordinationFailure,
		judgementHolders,
		workWaitingOnOwner
	} from '$lib/model/attention-presentation';
	import { emptyWorkTitle, matchesSearch, goalsWithFinishedWork } from '$lib/model/work-views';
	import { dueLabel, finishLine, type Goal } from '$lib/work/goal';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import Ban from '@lucide/svelte/icons/ban';
	import {
		blockerSentence,
		steerWork,
		closeGoal,
		fetchWorkSpend,
		priorityLabel,
		reopenGoal,
		setGoalDetails,
		setGoalTeam
	} from '$lib/model/work-actions';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { celebrate } from '$lib/ui/celebrate';

	const companyId = $derived(page.params.companyId ?? '');
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
	/* Coordination down is not an empty company: its Work may be missing. */
	const coordination = $derived(
		coordinationFailure(
			ownerAccess ? attention : collaboration ? { sourceHealth: collaboration.source_health } : null
		)
	);
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
	/* Who does it, what state it is in, and whether priority leads: filters kept in the address. */
	const ownerFilter = $derived(page.url.searchParams.get('owner') ?? '');
	const statusFilter = $derived(page.url.searchParams.get('status') ?? '');
	const sort = $derived(page.url.searchParams.get('sort') === 'priority' ? 'priority' : 'recent');
	const stoppedView = $derived(selectedGoal === 'stopped');
	function setQuery(key: string, value: string) {
		const url = new URL(page.url);
		if (value) url.searchParams.set(key, value);
		else url.searchParams.delete(key);
		url.searchParams.delete('lens');
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}
	/* The search is in the address too, written as the owner pauses typing. */
	const urlSearch = $derived(page.url.searchParams.get('q') ?? '');
	let search = $state('');
	let searchTimer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const next = urlSearch;
		if (next !== untrack(() => search.trim())) search = next;
	});
	function typeSearch(value: string) {
		search = value;
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => {
			if (search.trim() !== urlSearch) setQuery('q', search.trim());
		}, 250);
	}
	const filtered = $derived(!!ownerFilter || !!statusFilter);
	function clearSearch() {
		clearTimeout(searchTimer);
		search = '';
		const url = new URL(page.url);
		for (const key of ['q', 'owner', 'status']) url.searchParams.delete(key);
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}

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
	/* Asking Exec for work: in an open goal the composer starts with that goal, and each Work that
	 * appears afterwards is confirmed with a way to open it. */
	let watchedWork = $state<Set<string> | null>(null);
	function addWork() {
		watchedWork = new Set((graph?.work ?? []).map((item) => item.id));
		askExec(
			selectedGoalRow && !selectedGoalRow.closed_at
				? `Add work toward “${selectedGoalRow.title}”: `
				: undefined
		);
	}
	$effect(() => {
		const rows = graph?.work ?? [];
		const known = untrack(() => watchedWork);
		if (!known) return;
		const added = rows.filter((item) => !known.has(item.id));
		if (!added.length) return;
		watchedWork = new Set(rows.map((item) => item.id));
		for (const item of added)
			toast(`Work added: ${item.title}`, {
				action: { label: 'Open', run: () => goto(workHref(item.id)) }
			});
	});
	const noWorkYet = $derived(!!graph && graph.work.length === 0 && goals.length === 0);
	/* Done stays out of the way (S85-14): Work finished more than a week ago, or before this
	 * person last cleared Done, lives under Archived. Clearing is kept per person in this
	 * browser. */
	const DONE_KEEP_MS = 7 * 24 * 60 * 60 * 1000;
	const clearedKey = $derived(`restless:work-done-cleared:${companyId}`);
	let doneClearedAt = $state(0);
	$effect(() => {
		try {
			doneClearedAt = Number(localStorage.getItem(clearedKey)) || 0;
		} catch {
			doneClearedAt = 0;
		}
	});
	function retired(item: { status: string; updated_at: string }): boolean {
		if (item.status !== 'completed') return false;
		const finished = Date.parse(item.updated_at);
		return finished < Date.now() - DONE_KEEP_MS || finished <= doneClearedAt;
	}
	/* "other" is the Work that serves no Goal: a peer of the Goals, kept small. "waiting" and
	 * "active" are views across every Goal: what waits on the owner, and what is moving. */
	const goalWork = $derived(
		(graph?.work ?? []).filter(
			(item) =>
				/* A search also finds stopped Work, so nothing the owner looks for is hidden. */
				(stoppedView
					? item.status === 'abandoned'
					: item.status !== 'abandoned' || !!search.trim()) &&
				(!ownerFilter || item.owner_id === ownerFilter) &&
				(!statusFilter || item.status === statusFilter) &&
				matchesSearch(search, [
					item.title,
					item.outcome,
					ownerName(item.owner_id),
					goals.find((goal) => goal.id === item.goal_id)?.title,
					...(graph?.artifacts ?? [])
						.filter((artifact) => artifact.work_id === item.id)
						.map((artifact) => artifact.label)
				]) &&
				(!selectedGoal ||
					stoppedView ||
					(selectedGoal === 'other'
						? !item.goal_id
						: selectedGoal === 'waiting'
							? item.status !== 'completed' && waitingOnOwner.has(item.id)
							: selectedGoal === 'active'
								? item.status === 'active'
								: selectedGoal === 'archived'
									? (!!item.goal_id && archivedGoalIds.has(item.goal_id)) || retired(item)
									: item.goal_id === selectedGoal)) &&
				/* Retired Done shows only under Archived, or when a search looks for it. */
				(selectedGoal === 'archived' || !!search.trim() || !retired(item))
		)
	);
	const waitingCount = $derived(
		(graph?.work ?? []).filter(
			(item) =>
				item.status !== 'completed' && item.status !== 'abandoned' && waitingOnOwner.has(item.id)
		).length
	);
	const stoppedCount = $derived(
		(graph?.work ?? []).filter((item) => item.status === 'abandoned').length
	);
	const activeCount = $derived(
		(graph?.work ?? []).filter((item) => item.status === 'active').length
	);
	/* A goal's second line: how far its Work has gone, and what is stuck, from the Work itself. */
	function goalReading(goalId: string): { text: string; tone: SidebarTone } {
		const rows = (graph?.work ?? []).filter(
			(item) => item.goal_id === goalId && item.status !== 'abandoned'
		);
		if (!rows.length) return { text: 'No work yet', tone: '' };
		const done = rows.filter((item) => item.status === 'completed').length;
		const blocked = rows.filter((item) => item.status === 'blocked').length;
		const waiting = rows.filter(
			(item) => item.status !== 'completed' && waitingOnOwner.has(item.id)
		).length;
		const parts = [`${done} of ${rows.length} done`];
		if (waiting) parts.push(`${waiting} needs you`);
		if (blocked) parts.push(`${blocked} blocked`);
		return {
			text: parts.join(' · '),
			tone: waiting
				? 'wait'
				: blocked
					? 'block'
					: rows.some((item) => item.status === 'active')
						? 'work'
						: ''
		};
	}
	const otherOpen = $derived(
		(graph?.work ?? []).filter(
			(item) => !item.goal_id && item.status !== 'abandoned' && item.status !== 'completed'
		).length
	);
	/* Too narrow for the sidebar: the same places as one row, in the same order. */
	const narrowViews = $derived([
		{ goal: '', label: 'All work' },
		...(graph?.work.length
			? [
					{ goal: 'waiting', label: 'Needs you' },
					{ goal: 'active', label: 'In progress' }
				]
			: []),
		...openGoals.map((goal) => ({ goal: goal.id, label: goal.title })),
		...(otherOpen ? [{ goal: 'other', label: 'No goal' }] : []),
		{ goal: 'archived', label: 'Archived' },
		...(stoppedCount ? [{ goal: 'stopped', label: 'Stopped' }] : []),
		...(archivedOpen ? closedGoals.map((goal) => ({ goal: goal.id, label: goal.title })) : [])
	]);
	/* Work with a decision or step waiting on the viewer: what their Inbox shows for it. For the
	 * owner that is the company queue; for a team's owner, the handoffs passed to them. */
	const waitingOnOwner = $derived(
		ownerAccess
			? workWaitingOnOwner(attention?.items ?? [])
			: new Set((collaboration?.attention.handoffs ?? []).map((handoff) => handoff.work_id))
	);
	/* Judgement a lead or Exec still holds before it reaches the owner. */
	const heldBy = $derived(judgementHolders(graph && 'handoffs' in graph ? graph.handoffs : []));
	const holder = (work: WorkItem) =>
		work.status !== 'completed' && heldBy.has(work.id)
			? `With ${ownerName(heldBy.get(work.id) ?? '')}`
			: '';
	const byRecent = (a: WorkItem, b: WorkItem) =>
		Date.parse(b.updated_at) - Date.parse(a.updated_at);
	const bySort = $derived((a: WorkItem, b: WorkItem) =>
		sort === 'priority' ? b.priority - a.priority || byRecent(a, b) : byRecent(a, b)
	);
	const groups = $derived([
		{
			key: 'owner',
			label: 'Needs you',
			tone: 'warning' as const,
			rows: goalWork
				.filter((item) => item.status !== 'completed' && waitingOnOwner.has(item.id))
				.toSorted(bySort)
		},
		...(['blocked', 'active', 'proposed', 'abandoned'] as const).map((status) => ({
			key: status,
			label: WORK_STATUS_LABEL[status],
			tone: (status === 'blocked' ? 'danger' : status === 'active' ? 'progress' : 'muted') as
				'danger' | 'progress' | 'muted',
			rows: goalWork
				.filter((item) => item.status === status && !waitingOnOwner.has(item.id))
				.toSorted(bySort)
		}))
	]);
	const done = $derived(goalWork.filter((item) => item.status === 'completed').toSorted(bySort));
	const openCount = $derived(goalWork.length - done.length);
	const title = $derived(
		selectedGoal === 'other'
			? 'No goal'
			: selectedGoal === 'waiting'
				? 'Needs you'
				: selectedGoal === 'active'
					? 'In progress'
					: selectedGoal === 'archived'
						? 'Archived'
						: stoppedView
							? 'Stopped'
							: (goals.find((goal) => goal.id === selectedGoal)?.title ?? 'All work')
	);
	/* The quality bar lives on the Goal; a piece of Work shows it only when it
	 * holds itself to a different one. */
	const goalStandard = (goal: object | undefined): OutcomeStandard | undefined =>
		goal && 'outcome_standard' in goal
			? (goal as { outcome_standard: OutcomeStandard }).outcome_standard
			: undefined;
	const selectedGoalRow = $derived(goals.find((goal) => goal.id === selectedGoal));
	/* Teams own goals: which team delivers each one, and who owns that team. */
	const teamRows = $derived((ownerAccess ? cockpit?.teams : collaboration?.teams) ?? []);
	const peopleRows = $derived((ownerAccess ? cockpit?.people : collaboration?.people) ?? []);
	function goalTeam(goal: Goal): string {
		const team = goal.team_id ? teamRows.find((row) => row.id === goal.team_id) : undefined;
		if (!team) return '';
		const ownerId =
			('owner_person' in team ? team.owner_person : team.responsible_actor_id) ?? null;
		const owner = ownerId
			? ownerId === principalProjection.view?.actor_id
				? 'you'
				: (peopleRows.find((person) => person.actor_id === ownerId)?.display ?? '')
			: '';
		return owner ? `${team.name} team, owned by ${owner}` : `${team.name} team`;
	}
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
	function workHref(workId: string, talk = ''): string {
		const query = new URLSearchParams();
		if (selectedGoal) query.set('goal', selectedGoal);
		if (view === 'board') query.set('view', 'board');
		if (urlSearch) query.set('q', urlSearch);
		if (talk) query.set('talk', talk);
		const search = query.toString();
		return `/${encodeURIComponent(companyId)}/work/${encodeURIComponent(workId)}${search ? `?${search}` : ''}`;
	}
	/* Resume from the row: no reason is asked for here; the Work page takes one when there is one. */
	let resuming = $state(new Set<string>());
	async function quickResume(work: WorkItem) {
		resuming = new Set([...resuming, work.id]);
		try {
			await steerWork(companyId, work.id, 'resume', 'Resumed by the owner from the Work list');
			toast(`Resumed: ${work.title}`);
			retryWork();
		} catch (cause) {
			toast(failureSentence(cause, 'That Work did not resume.'), { tone: 'failure' });
		} finally {
			resuming = new Set([...resuming].filter((id) => id !== work.id));
		}
	}
	function meta(work: WorkItem, withOwner = true): string {
		const run = runStateLabel(attemptOf(work)?.state);
		const count = outputs(work);
		const own = overrides.get(work.id);
		return [
			withOwner ? ownerName(work.owner_id) : '',
			work.status === 'blocked' && !waitingOnOwner.has(work.id)
				? blockerSentence(work.resolution, attemptOf(work)?.summary ?? '')
				: '',
			priorityLabel(work.priority),
			waitingOnOwner.has(work.id) ? '' : holder(work),
			own ? `${standardLabel(own)} bar` : '',
			/* A run's state only when it adds to the Work's own: it failed or was sent back. */
			['failed', 'changes_requested'].includes(attemptOf(work)?.state ?? '') ? run : '',
			count ? `${count} ${count === 1 ? 'output' : 'outputs'}` : ''
		]
			.filter(Boolean)
			.join(' · ');
	}
	/* The board shows recent Done, as the list folds it; the column's footer shows the rest. */
	const RECENT_DONE = 8;
	let showAllDone = $state(false);
	const boardColumns = $derived(
		(stoppedView
			? (['abandoned'] as const)
			: goalWork.some((item) => item.status === 'abandoned')
				? (['proposed', 'active', 'blocked', 'completed', 'abandoned'] as const)
				: (['proposed', 'active', 'blocked', 'completed'] as const)
		).map((status) => ({
			key: status,
			label: WORK_STATUS_LABEL[status],
			/* Stacked on a narrow pane, blocked Work comes first rather than below every card. */
			lead: status === 'blocked',
			items: goalWork
				.filter((item) => item.status === status)
				.toSorted(bySort)
				.slice(0, status === 'completed' && !showAllDone ? RECENT_DONE : undefined)
				.map((row) => ({
					id: row.id,
					title: row.title,
					signal: meta(row, false),
					status: row.status,
					ownerName: ownerName(row.owner_id),
					revision: row.revision,
					href: workHref(row.id),
					needsYou: row.status !== 'completed' && waitingOnOwner.has(row.id)
				}))
		}))
	);
	/* Filters: the people who own Work here, and the states it can be in. */
	const ownerOptions = $derived(
		[...new Set((graph?.work ?? []).map((item) => item.owner_id))]
			.map((id) => ({ id, name: ownerName(id) }))
			.toSorted((a, b) => a.name.localeCompare(b.name))
	);
	const STATUS_OPTIONS = ['blocked', 'active', 'proposed', 'completed'] as const;

	/* Metered spend per Work, summed for a Goal. Owner only; unavailable says so. */
	let workSpend = $state<Map<string, number> | 'unavailable' | null>(null);
	$effect(() => {
		if (!ownerAccess || !companyId) return;
		let current = true;
		fetchWorkSpend(companyId)
			.then((rows) => {
				if (current) workSpend = rows;
			})
			.catch(() => {
				if (current) workSpend = 'unavailable';
			});
		return () => (current = false);
	});
	function goalCost(goalId: string): string {
		if (workSpend === null) return '';
		if (workSpend === 'unavailable') return 'Cost unavailable right now';
		const spend = workSpend;
		const total = (graph?.work ?? [])
			.filter((item) => item.goal_id === goalId)
			.reduce((sum, item) => sum + (spend.get(item.id) ?? 0), 0);
		return total > 0 ? `$${total.toFixed(2)} spent` : 'Nothing metered yet';
	}

	/* The selected Goal's own menu: edit its words, who delivers it, close or reopen it. */
	let goalEditing = $state(false);
	let goalTitle = $state('');
	let goalDoneWhen = $state('');
	let goalDue = $state('');
	let goalBusy = $state(false);
	let goalFailure = $state('');
	const goalDueOn = (goal: object) => (goal as { due_on?: string | null }).due_on ?? '';
	function openGoalEdit() {
		if (!selectedGoalRow) return;
		goalTitle = selectedGoalRow.title;
		goalDoneWhen = finishLine(selectedGoalRow);
		goalDue = goalDueOn(selectedGoalRow);
		goalFailure = '';
		goalEditing = true;
	}
	async function goalAction(run: () => Promise<unknown>, done: string) {
		if (goalBusy) return;
		goalBusy = true;
		goalFailure = '';
		try {
			await run();
			toast(done);
			goalEditing = false;
			await cockpitProjection.refresh();
		} catch (cause) {
			goalFailure = failureSentence(cause, 'The Goal did not change.');
		} finally {
			goalBusy = false;
		}
	}
	async function archiveSelectedGoal() {
		const goal = selectedGoalRow;
		if (!goal || goalBusy) return;
		if (
			await confirmAction({
				title: 'Archive this goal?',
				body:
					'Archive “' +
					goal.title +
					'”? Existing work keeps running. You can reopen the goal from Archived.',
				action: 'Archive goal'
			})
		)
			await goalAction(() => closeGoal(companyId, goal.id), 'Goal archived');
	}

	let clearingDone = $state(false);
	const archiveCandidates = $derived.by(() => {
		if (
			!ownerAccess ||
			ownerFilter ||
			statusFilter ||
			search.trim() ||
			(selectedGoal && !selectedGoalRow) ||
			selectedGoalRow?.closed_at
		)
			return [];
		const ready = goalsWithFinishedWork(goals, graph?.work ?? []);
		return goals.filter(
			(goal) => ready.has(goal.id) && (!selectedGoal || selectedGoal === goal.id)
		);
	});
	async function clearCompleted(from: Element) {
		if (clearingDone) return;
		const candidates = archiveCandidates;
		if (
			candidates.length &&
			!(await confirmAction({
				title:
					candidates.length === 1
						? 'Clear done and archive the goal?'
						: 'Clear done and archive ' + candidates.length + ' goals?',
				body:
					candidates.map((goal) => '“' + goal.title + '”').join(', ') +
					' has no unfinished work. Archive ' +
					(candidates.length === 1 ? 'this goal' : 'these goals') +
					' while clearing completed work? Each can be reopened from Archived.',
				action: 'Clear and archive'
			}))
		)
			return;
		clearingDone = true;
		const previous = doneClearedAt;
		const count = done.length;
		const archived: Goal[] = [];
		try {
			for (const goal of candidates) {
				await closeGoal(companyId, goal.id);
				archived.push(goal);
			}
			doneClearedAt = Date.now();
			try {
				localStorage.setItem(clearedKey, String(doneClearedAt));
			} catch {
				/* This visit only. */
			}
			celebrate(from);
			toast(
				(count ? count + ' completed tasks cleared' : 'Completed work cleared') +
					(archived.length
						? ' · ' +
							archived.length +
							(archived.length === 1 ? ' goal archived' : ' goals archived')
						: ''),
				{
					undo: () => {
						doneClearedAt = previous;
						try {
							localStorage.setItem(clearedKey, String(previous));
						} catch {
							/* This visit only. */
						}
						void Promise.all(archived.map((goal) => reopenGoal(companyId, goal.id)))
							.then(() => cockpitProjection.refresh())
							.catch((cause) =>
								toast(failureSentence(cause, 'Some goals could not be reopened.'), {
									tone: 'failure'
								})
							);
					}
				}
			);
		} catch (cause) {
			toast(failureSentence(cause, 'Completed work was not cleared.'), { tone: 'failure' });
		} finally {
			if (archived.length) await cockpitProjection.refresh();
			clearingDone = false;
		}
	}
	function saveGoal() {
		const row = selectedGoalRow;
		if (!row || !goalTitle.trim()) return;
		const details: Parameters<typeof setGoalDetails>[2] = {};
		if (goalTitle.trim() !== row.title) details.title = goalTitle.trim();
		if (goalDoneWhen.trim() !== finishLine(row)) details.done_when = goalDoneWhen.trim();
		const due = goalDueOn(row);
		if (goalDue !== due) details.due_on = goalDue || null;
		if (!Object.keys(details).length) {
			goalEditing = false;
			return;
		}
		void goalAction(() => setGoalDetails(companyId, row.id, details), 'Goal saved');
	}
	$effect(() => {
		void selectedGoal;
		goalEditing = false;
		goalFailure = '';
	});

	holdQueries(() => [
		principalProjection,
		attentionProjection,
		cockpitProjection,
		collaborationProjection
	]);
</script>

<CompanyTitle title="Work" {companyId} />

{#snippet clearDone()}
	<button
		class="btn small ghost"
		type="button"
		disabled={clearingDone}
		title="Clear completed work from the current view. You can also archive goals whose tasks are all complete."
		onclick={(event) => void clearCompleted(event.currentTarget)}
	>
		{clearingDone
			? 'Clearing…'
			: done.length
				? 'Clear done'
				: archiveCandidates.length === 1
					? 'Archive goal'
					: 'Archive finished goals'}
	</button>
{/snippet}

{#snippet goalRow(goal: Goal, indent = false)}
	{@const progress = goalProgress(goal.id)}
	{@const reading = goalReading(goal.id)}
	<SidebarRow
		href={viewHref(goal.id)}
		label={goal.title}
		active={selectedGoal === goal.id}
		sub={indent ? undefined : reading.text}
		subTone={reading.tone}
		{indent}
		title={[
			goalTeam(goal),
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
				label={waitingOnOwner.has(work.id)
					? 'Needs you'
					: holder(work) || WORK_STATUS_LABEL[work.status]}
			/>{/snippet}
		{#snippet trailing()}<RelativeTime value={work.updated_at} />{/snippet}
		{#snippet actions()}
			{#if ownerAccess && work.status === 'blocked' && !waitingOnOwner.has(work.id)}<button
					class="btn small"
					type="button"
					disabled={resuming.has(work.id)}
					title="Let this Work continue"
					onclick={() => quickResume(work)}>Resume</button
				>{/if}
			{#if ownerAccess}<a
					class="btn small ghost"
					href={workHref(work.id, 'exec')}
					title="Open this Work and talk about it with the Exec">Discuss</a
				>{/if}
			<CopyButton
				value={new URL(workHref(work.id), page.url).href}
				label="Copy link to this Work"
			/>
		{/snippet}
	</Item>
{/snippet}

<ResizableSidebar area="work" label="Resize the Work sidebar">
	<SidebarShell label="Goals" title="Work">
		{#snippet top()}
			<SidebarSearch bind:value={() => search, typeSearch} placeholder="Search work" />
		{/snippet}
		{#snippet nav()}
			<SidebarRow
				href={viewHref('')}
				label="All work"
				icon={ListIcon}
				active={!selectedGoal}
				count={loaded
					? (graph?.work ?? []).filter(
							(item) => item.status !== 'abandoned' && item.status !== 'completed'
						).length || null
					: null}
			/>
			{#if loaded && graph && graph.work.length}
				<SidebarRow
					href={viewHref('waiting')}
					label="Needs you"
					icon={InboxIcon}
					active={selectedGoal === 'waiting'}
					count={waitingCount}
					todo
					title="Work with a decision or step only you can take"
				/>
				<SidebarRow
					href={viewHref('active')}
					label="In progress"
					icon={Activity}
					active={selectedGoal === 'active'}
					count={activeCount}
					title="Work someone is doing now"
				/>
			{/if}
			{#if loaded && graph}
				<SidebarGroup
					label="Goals"
					count={openGoals.length || null}
					help="Goals are the outcomes you're working toward, like “3 paying clients by November”. Exec proposes them as you talk and keeps the work under them."
					action={ownerAccess && openGoals.length
						? { label: 'Ask Exec to propose goals', icon: Plus, onclick: proposeGoals }
						: undefined}
				>
					{#each openGoals as goal (goal.id)}{@render goalRow(goal)}{:else}
						<!-- What a goal is lives in the heading's tooltip; the empty state only says there is none. -->
						<SidebarEmpty text="No goals yet">
							{#if ownerAccess}<button
									type="button"
									title="Ask Exec to propose goals"
									onclick={proposeGoals}
									><Sparkles size={15} strokeWidth={1.8} aria-hidden="true" />Propose goals</button
								>{/if}
						</SidebarEmpty>
					{/each}
					{#if otherOpen}<SidebarRow
							href={viewHref('other')}
							label="No goal"
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
				href={viewHref('archived')}
				label="Archived"
				icon={ArchiveIcon}
				active={selectedGoal === 'archived'}
				count={closedGoals.length || null}
				title="Goals that are closed, and their work"
			/>
			{#if archivedOpen}{#each closedGoals as goal (goal.id)}{@render goalRow(
						goal,
						true
					)}{/each}{/if}
			{#if stoppedCount}<SidebarRow
					href={viewHref('stopped')}
					label="Stopped"
					icon={Ban}
					active={stoppedView}
					count={stoppedCount}
					title="Work that was stopped for good; open one to restore it"
				/>{/if}
		{/snippet}
		{#snippet narrow()}
			<div class="narrow-search">
				<SidebarSearch bind:value={() => search, typeSearch} placeholder="Search work" />
			</div>
			{#if loaded && graph && !noWorkYet}
				<label class="goal-select">
					<span>Show</span>
					<select
						aria-label="Work view"
						value={selectedGoal}
						onchange={(event) => setQuery('goal', event.currentTarget.value)}
					>
						{#each narrowViews as item (item.goal)}<option value={item.goal}>{item.label}</option
							>{/each}
					</select>
				</label>
			{/if}
		{/snippet}

		<div class="page">
			<header class="head">
				<div class="title">
					{#if selectedGoal}<Target size={15} strokeWidth={1.8} aria-hidden="true" />{/if}
					<h1>{title}</h1>
					<span class="count" title="Open work">{loaded && openCount ? openCount : ''}</span>
					{#if selectedGoal}<button
							class="clear"
							type="button"
							aria-label="Show all work"
							title="Show all work"
							onclick={() => setQuery('goal', '')}><X size={14} strokeWidth={2} /></button
						>{/if}
				</div>
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
				{#if selectedGoalRow && ownerAccess}
					<div class="goal-standard-picker">
						<ActionMenu label="Goal actions">
							{#snippet trigger()}<span>Goal actions</span>{/snippet}
							{#if !selectedGoalRow.closed_at}
								<button type="button" onclick={addWork}><span>Add work…</span></button>
								<button type="button" onclick={openGoalEdit}><span>Edit goal…</span></button>
								{#each teamRows as team (team.id)}
									<button
										type="button"
										disabled={goalBusy}
										onclick={() =>
											goalAction(
												() => setGoalTeam(companyId, selectedGoalRow.id, team.id),
												`${team.name} team delivers it`
											)}
										><span>Delivered by {team.name}</span
										>{#if selectedGoalRow.team_id === team.id}<Check
												size={14}
												aria-label="Current"
											/>{/if}</button
									>
								{/each}
								{#if selectedGoalRow.team_id}
									<button
										type="button"
										disabled={goalBusy}
										onclick={() =>
											goalAction(
												() => setGoalTeam(companyId, selectedGoalRow.id, null),
												'You hold this goal'
											)}><span>Hold it yourself</span></button
									>
								{/if}
								<button type="button" onclick={() => void archiveSelectedGoal()}
									><span>Archive goal…</span></button
								>
							{:else}
								<button
									type="button"
									disabled={goalBusy}
									onclick={() =>
										goalAction(() => reopenGoal(companyId, selectedGoalRow.id), 'Goal reopened')}
									><span>Reopen goal</span></button
								>
							{/if}
						</ActionMenu>
					</div>
				{/if}
				<div class="filters">
					<div class="goal-standard-picker" title="Show the work one person owns">
						<ActionMenu label="Owner">
							{#snippet trigger()}<span>{ownerFilter ? ownerName(ownerFilter) : 'Anyone'}</span
								>{/snippet}
							<button type="button" onclick={() => setQuery('owner', '')}
								><span>Anyone</span>{#if !ownerFilter}<Check
										size={14}
										aria-label="Current"
									/>{/if}</button
							>
							{#each ownerOptions as option (option.id)}
								<button type="button" onclick={() => setQuery('owner', option.id)}
									><span>{option.name}</span>{#if ownerFilter === option.id}<Check
											size={14}
											aria-label="Current"
										/>{/if}</button
								>
							{/each}
						</ActionMenu>
					</div>
					{#if !stoppedView}
						<div class="goal-standard-picker" title="Show work in one state">
							<ActionMenu label="Status">
								{#snippet trigger()}<span
										>{statusFilter
											? WORK_STATUS_LABEL[statusFilter as WorkRow['status']]
											: 'Any status'}</span
									>{/snippet}
								<button type="button" onclick={() => setQuery('status', '')}
									><span>Any status</span>{#if !statusFilter}<Check
											size={14}
											aria-label="Current"
										/>{/if}</button
								>
								{#each STATUS_OPTIONS as option (option)}
									<button type="button" onclick={() => setQuery('status', option)}
										><span>{WORK_STATUS_LABEL[option]}</span>{#if statusFilter === option}<Check
												size={14}
												aria-label="Current"
											/>{/if}</button
									>
								{/each}
							</ActionMenu>
						</div>
					{/if}
					<div class="goal-standard-picker" title="Order the work">
						<ActionMenu label="Sort">
							{#snippet trigger()}<span
									>{sort === 'priority' ? 'Priority first' : 'Recent first'}</span
								>{/snippet}
							<button type="button" onclick={() => setQuery('sort', '')}
								><span>Recent first</span>{#if sort === 'recent'}<Check
										size={14}
										aria-label="Current"
									/>{/if}</button
							>
							<button type="button" onclick={() => setQuery('sort', 'priority')}
								><span>Priority first</span>{#if sort === 'priority'}<Check
										size={14}
										aria-label="Current"
									/>{/if}</button
							>
						</ActionMenu>
					</div>
				</div>
				{#if done.length || archiveCandidates.length}
					<div class="completed-actions" aria-label="Completed work actions">
						{#if view === 'board' && done.length > RECENT_DONE}
							<button
								class="btn small ghost"
								type="button"
								onclick={() => (showAllDone = !showAllDone)}
							>
								{showAllDone ? 'Show recent only' : 'Show all ' + done.length}
							</button>
						{/if}
						{#if selectedGoal !== 'archived'}{@render clearDone()}{/if}
					</div>
				{/if}
				<div class="view-switch">
					<Segmented
						label="View"
						value={view}
						options={[
							{ value: 'list', label: 'List' },
							{ value: 'board', label: 'Board' }
						]}
						onchange={(value) => setQuery('view', value === 'board' ? 'board' : '')}
					/>
				</div>
			</header>

			<!-- Board padding only for the board itself: empty, loading and failure sit where List puts them. -->
			<div class="body" class:board={view === 'board' && loaded && !!graph && !noWorkYet}>
				{#if failure && graph}<FailureNotice
						error={failure}
						subject="Work"
						stale
						onretry={retryWork}
					/>{:else if coordination && graph}<FailureNotice
						error={coordination}
						onretry={retryWork}
					/>{/if}
				{#if selectedGoalRow && loaded && graph}
					{@const progress = goalProgress(selectedGoalRow.id)}
					<section class="goal-head" aria-label="About this goal">
						{#if selectedGoalRow.body.trim() || finishLine(selectedGoalRow)}
							<details class="goal-description">
								<summary>About this goal</summary>
								{#if selectedGoalRow.body.trim() && selectedGoalRow.body.trim() !== selectedGoalRow.title}<p
										class="goal-body"
									>
										{selectedGoalRow.body}
									</p>{/if}
								{#if finishLine(selectedGoalRow)}<p class="goal-body">
										Done when {finishLine(selectedGoalRow)}
									</p>{/if}
							</details>
						{/if}
						<p class="goal-facts">
							<span>{progress.done} of {progress.total} done</span>
							{#if dueLabel(selectedGoalRow)}<span>Due {dueLabel(selectedGoalRow)}</span>{/if}
							{#if goalTeam(selectedGoalRow)}<span>{goalTeam(selectedGoalRow)}</span>{/if}
							<!-- The facts a goal row's hover title carries, readable on touch once it is chosen. -->
							{#if !ownerAccess}<span
									>{standardLabel(goalStandard(selectedGoalRow))} quality bar</span
								>{/if}
							{#if ownerAccess && goalCost(selectedGoalRow.id)}<span
									title="Metered model spend on this goal's work"
									>{goalCost(selectedGoalRow.id)}</span
								>{/if}
							{#if selectedGoalRow.closed_at}<span>Closed</span>{/if}
						</p>

						{#if goalFailure}<p class="standard-failure" role="alert">{goalFailure}</p>{/if}
					</section>
				{/if}
				{#if !graph && failure}
					<FailureNotice error={failure} subject="Work" variant="page" onretry={retryWork} />
				{:else if !graph && coordination && loaded}
					<FailureNotice error={coordination} subject="Work" variant="page" onretry={retryWork} />
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
									onclick={addWork}>Tell Exec what you want</button
								>{/if}{/snippet}
					</Empty>
				{:else if !goalWork.length && (search.trim() || filtered)}
					<Empty compact title={emptyWorkTitle(selectedGoal, search, filtered)}>
						{#snippet action()}<button class="btn small" type="button" onclick={clearSearch}
								>{search.trim() ? 'Clear search' : 'Clear filters'}</button
							>{/snippet}
					</Empty>
				{:else if view === 'board'}
					<WorkBoard columns={boardColumns}></WorkBoard>
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
						<Empty compact title={emptyWorkTitle(selectedGoal)}>
							{#snippet action()}{#if ownerAccess && selectedGoalRow && !selectedGoalRow.closed_at}<button
										class="btn small"
										type="button"
										onclick={addWork}>Add work</button
									>{/if}{/snippet}
						</Empty>
					{/if}
				{/if}
			</div>
		</div>
	</SidebarShell>
</ResizableSidebar>

{#if goalEditing && selectedGoalRow}
	<Dialog title="Edit goal" busy={goalBusy} onclose={() => (goalEditing = false)}>
		<form
			class="goal-form"
			onsubmit={(event) => {
				event.preventDefault();
				saveGoal();
			}}
		>
			<label><span>Goal</span><input bind:value={goalTitle} required /></label>
			<label
				><span>Done when</span><input
					bind:value={goalDoneWhen}
					maxlength="400"
					placeholder="What you will see when it is done"
				/></label
			>
			<label><span>Due</span><input type="date" bind:value={goalDue} /></label>
			<div class="goal-form-actions">
				<button class="btn small primary" disabled={goalBusy || !goalTitle.trim()}
					>{goalBusy ? 'Saving…' : 'Save'}</button
				>
				<button class="btn small ghost" type="button" onclick={() => (goalEditing = false)}
					>Cancel</button
				>
			</div>
			{#if goalFailure}<p class="standard-failure" role="alert">{goalFailure}</p>{/if}
		</form>
	</Dialog>
{/if}

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
	/* Too narrow for the sidebar: its search, then its places as one row that scrolls sideways. */
	.narrow-search {
		padding: 10px 12px 4px;
	}
	.goal-select {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 6px 12px;
		min-width: 0;
	}
	.goal-select select {
		flex: 1;
		width: 100%;
		min-width: 0;
	}
	/* One column the width of the pane: a long goal title is cut, never widening every row. */
	.page {
		container: page / inline-size;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		grid-template-rows: auto minmax(0, 1fr);
		width: 100%;
		min-height: 0;
	}
	/* On a narrow pane the controls wrap under the title rather than off the edge. */
	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		min-height: 52px;
		padding: 8px 12px 8px 20px;
		border-bottom: 1px solid var(--border);
	}
	.title {
		flex: 1 1 240px;
		max-width: 100%;
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}
	.title > :global(svg) {
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
		flex: none;
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
	.view-switch {
		display: flex;
	}
	.filters {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-left: auto;
	}
	.goal-head {
		display: grid;
		gap: 6px;
		margin: 0 0 14px;
		padding: 2px 4px;
	}
	.body.board .goal-head {
		margin: 0;
		padding: 12px 16px;
		border-bottom: 1px solid var(--border);
	}
	.goal-body {
		margin: 0;
		color: var(--ink);
		white-space: pre-line;
	}
	.goal-facts {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 14px;
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.goal-form {
		display: grid;
		gap: 8px;
		max-width: 520px;
	}
	.goal-form label {
		display: grid;
		gap: 4px;
	}
	.goal-form label span {
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.goal-form input {
		padding: 6px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
	}
	.goal-form-actions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
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
	/* A goal's progress: a small ring, filled as its work completes. */
	.ring {
		flex: none;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: conic-gradient(var(--state-success) var(--done), var(--border-strong) 0);
		mask: radial-gradient(circle, transparent 3.5px, #000 4px);
	}
	.completed-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.goal-description summary {
		color: var(--text-secondary);
		font-size: var(--t-label);
		cursor: pointer;
	}
	.goal-description p {
		margin-block: 8px;
	}
</style>
