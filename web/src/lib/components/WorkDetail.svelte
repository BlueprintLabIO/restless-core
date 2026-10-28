<script lang="ts">
	import Field from '$lib/ui/forms/Field.svelte';
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
	import { formatMoment } from '$lib/ui/time';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { runStateLabel, workStatusLabel } from '$lib/work/status';
	import { resizePane } from '$lib/actions/resize-pane';
	import { page } from '$app/state';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/ui/glyph/MatrixGlyph.svelte';
	import type { CollaborationArtifact, CollaborationWork } from '$lib/model/collaboration';
	import {
		attentionQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		companyPrincipalQuery,
		workActivityStream
	} from '$lib/model/queries.svelte';
	import type { ArtifactRefRow, WorkGateRow, WorkRow } from '$lib/model/generated/orgintel';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import Check from '@lucide/svelte/icons/check';
	import X from '@lucide/svelte/icons/x';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { celebrate } from '$lib/ui/celebrate';
	import { coordinationFailure } from '$lib/model/attention-presentation';
	import { fileLabel, LIBRARY_KIND_LABEL, libraryKind } from '$lib/model/library-files';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import { goalAbout } from '$lib/work/goal';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { attentionTitle, isOutcomeReview } from '$lib/model/attention-presentation';
	import type { AttentionItem } from '$lib/model/view';
	import {
		PRIORITIES,
		blockerSentence,
		steerResolution,
		fetchWorkFeedback,
		fetchWorkSpend,
		priorityLabel,
		reassignWork,
		reopenWork,
		setWorkDetails,
		steerWork,
		type WorkFeedback,
		type WorkSteer
	} from '$lib/model/work-actions';

	/* The authoring contract deliberately separates a human opening from the
	 * exact actor contract with one blank line. Respect that declared boundary;
	 * never infer a summary from arbitrary prose or rewrite either side. */
	const OUTCOME_CLAMP_CHARS = 460;
	let outcomeExpanded = $state(false);

	/* A piece of Work, shown in a side sheet beside its agent's chat in People (S85-15). */
	let {
		companyId,
		workId,
		onclose,
		onowner
	}: {
		companyId: string;
		workId: string;
		onclose?: () => void;
		/** The agent doing this Work, once it is known: People opens that agent's chat. */
		onowner?: (actorId: string) => void;
	} = $props();
	const principalProjection = $derived(companyPrincipalQuery(companyId));
	const ownerAccess = $derived(principalProjection.view?.membership_role === 'owner');
	/* Everyone's attention: the owner's is the company queue, anyone else's what is assigned to
	 * them. The decision on this Work is shown here, not only in the Inbox. */
	const attentionProjection = $derived(
		attentionQuery(
			companyId,
			() => !!principalProjection.view,
			() => ownerAccess
		)
	);
	const cockpitProjection = $derived(cockpitQuery(companyId, () => ownerAccess));
	const collaborationProjection = $derived(
		collaborationBootstrapQuery(companyId, () => (ownerAccess ? null : principalProjection.view))
	);
	const attention = $derived(attentionProjection.view);
	const cockpit = $derived(cockpitProjection.view);
	const collaboration = $derived(collaborationProjection.view);
	const loaded = $derived(
		principalProjection.status !== 'unknown' &&
			(ownerAccess
				? attentionProjection.status !== 'unknown' && cockpitProjection.status !== 'unknown'
				: collaborationProjection.status !== 'unknown')
	);
	/* The first projection that failed, and whether this page still holds data
	 * from before the failure. FailureNotice turns that into owner copy. */
	const failure = $derived(
		principalProjection.failure ??
			(ownerAccess
				? (attentionProjection.failure ?? cockpitProjection.failure)
				: collaborationProjection.failure) ??
			null
	);
	const error = $derived(Boolean(failure));
	/* Coordination down is not missing Work: it may simply not be readable right now. */
	const coordination = $derived(
		coordinationFailure(
			ownerAccess ? attention : collaboration ? { sourceHealth: collaboration.source_health } : null
		)
	);
	function retryWork() {
		void principalProjection.refresh();
		if (ownerAccess) {
			void attentionProjection.reload();
			void cockpitProjection.refresh();
		} else void collaborationProjection.refresh();
	}
	type WorkItem = WorkRow | CollaborationWork;
	type Artifact = ArtifactRefRow | CollaborationArtifact;

	const graph = $derived(
		ownerAccess ? (attention?.workGraph ?? null) : (collaboration?.work_graph ?? null)
	);
	const work = $derived(graph?.work.find((item) => item.id === workId) ?? null);
	$effect(() => {
		if (work?.owner_id) onowner?.(work.owner_id);
	});
	const goals = $derived(ownerAccess ? (cockpit?.goals ?? []) : (collaboration?.goals ?? []));
	const people = $derived(ownerAccess ? (cockpit?.people ?? []) : (collaboration?.people ?? []));
	const teams = $derived(ownerAccess ? (cockpit?.teams ?? []) : (collaboration?.teams ?? []));
	const companyName = $derived(
		ownerAccess ? (cockpit?.company.name ?? companyId) : (collaboration?.company.name ?? companyId)
	);
	const goal = $derived(goals.find((item) => item.id === work?.goal_id) ?? null);
	/* The owner, and the person who owns this Work's team, may resume it or stop it; the server
	 * checks the team. It used to live only in a team owner's Inbox. */
	const workTeam = $derived(
		teams.find(
			(team) => team.id === people.find((person) => person.actor_id === work?.owner_id)?.team_id
		) ?? null
	);
	const maySteer = $derived(
		ownerAccess ||
			(!!workTeam &&
				'owner_person' in workTeam &&
				workTeam.owner_person === principalProjection.view?.actor_id)
	);
	/* One open form at a time: steer the Work, send it back, move it, or edit what it is. */
	type Steering = '' | WorkSteer | 'reopen' | 'reassign';
	let steering = $state<Steering>('');
	let steerReason = $state('');
	let steerBusy = $state(false);
	let steerFailure = $state('');
	let moveTo = $state('');
	const STEER: Record<
		Exclude<Steering, ''>,
		/* `fallback`: the note is optional and this is recorded when none is given. */
		{ placeholder: string; submit: string; done: string; warning?: string; fallback?: string }
	> = {
		resume: {
			placeholder: 'What changed, if you want to say (optional)',
			submit: 'Resume this Work',
			done: 'Resumed',
			fallback: 'Resumed from the Work page without a note'
		},
		pause: {
			placeholder: 'Why it should wait',
			submit: 'Pause this Work',
			done: 'Paused. Resume continues it.'
		},
		abandon: {
			placeholder: 'Why it should stop for good',
			submit: 'Stop for good',
			done: 'Stopped for good',
			warning: 'Nobody works on it again. It moves to Stopped, where it can be restored.'
		},
		restore: {
			placeholder: 'Why it should continue (optional)',
			submit: 'Restore this Work',
			done: 'Restored',
			fallback: 'Restored from the Work page without a note'
		},
		reopen: {
			placeholder: 'What should change',
			submit: 'Send back for changes',
			done: 'Sent back for changes'
		},
		reassign: {
			placeholder: 'Why they should take it',
			submit: 'Move this Work',
			done: 'Moved'
		}
	};
	function openSteer(action: Exclude<Steering, ''>, person = '') {
		moveTo = person;
		steerFailure = '';
		steering = steering === action && !person ? '' : action;
	}
	async function steer() {
		const action = steering;
		if (!work || !action || steerBusy || !(steerReason.trim() || STEER[action].fallback)) return;
		steerBusy = true;
		steerFailure = '';
		try {
			const reason = steerReason.trim() || STEER[action].fallback || '';
			if (action === 'reopen') await reopenWork(companyId, work.id, reason);
			else if (action === 'reassign') await reassignWork(companyId, work.id, moveTo, reason);
			else await steerWork(companyId, work.id, action, reason);
			if (action === 'resume' || action === 'restore') celebrate(document.activeElement);
			toast(action === 'reassign' ? `Moved to ${ownerName(moveTo)}` : STEER[action].done);
			steering = '';
			steerReason = '';
			feedbackVersion += 1;
			retryWork();
		} catch (cause) {
			steerFailure = failureSentence(cause, 'That was not recorded.');
		} finally {
			steerBusy = false;
		}
	}

	/* Edit what the Work is: its title, brief and expected output. */
	let editing = $state(false);
	let editTitle = $state('');
	let editOutcome = $state('');
	let editExpected = $state('');
	let editBusy = $state(false);
	let editFailure = $state('');
	function openEdit() {
		if (!work) return;
		editTitle = work.title;
		editOutcome = work.outcome;
		editExpected = work.expected_artifact ?? '';
		editFailure = '';
		editing = !editing;
	}
	async function saveEdit() {
		if (!work || editBusy) return;
		if (!editTitle.trim()) {
			editFailure = 'Give it a title.';
			return;
		}
		const details: Parameters<typeof setWorkDetails>[2] = {};
		if (editTitle.trim() !== work.title) details.title = editTitle.trim();
		if (editOutcome.trim() !== work.outcome.trim()) details.outcome = editOutcome.trim();
		if (editExpected.trim() !== (work.expected_artifact ?? '').trim())
			details.expected_artifact = editExpected.trim();
		if (!Object.keys(details).length) {
			editing = false;
			return;
		}
		await changeDetails(details, 'Saved');
		if (!editFailure) editing = false;
	}
	/* Priority and Goal change in place, from the facts beside the Work. */
	let detailFailure = $state('');
	async function changeDetails(details: Parameters<typeof setWorkDetails>[2], done: string) {
		if (!work || editBusy) return;
		editBusy = true;
		editFailure = '';
		detailFailure = '';
		try {
			await setWorkDetails(companyId, work.id, details);
			toast(done);
			retryWork();
		} catch (cause) {
			const sentence = failureSentence(cause, 'That change was not saved.');
			if (editing) editFailure = sentence;
			else detailFailure = sentence;
		} finally {
			editBusy = false;
		}
	}
	const movable = $derived(
		maySteer && !!work && work.status !== 'completed' && work.status !== 'abandoned'
	);
	/* Who could take this Work: the company's agents, and for a team's person only that team. */
	const candidates = $derived(
		people.filter(
			(person) =>
				(person.kind === 'staff' || person.kind === 'exec') &&
				person.actor_id !== work?.owner_id &&
				(ownerAccess || person.team_id === workTeam?.id)
		)
	);

	/* The decision waiting on this Work opens beside its prepared candidate in the Inbox. */
	const pending = $derived(
		(attentionProjection.view?.items ?? []).filter(
			(item) => item.workId === workId && !item.preparing
		)
	);
	const inboxHref = (item: AttentionItem) =>
		`/${encodeURIComponent(companyId)}?${isOutcomeReview(item) ? 'review' : 'item'}=${encodeURIComponent(item.id)}`;

	/* Feedback given to this Work and the spend it ran up. Owner only; a failure says so. */
	let feedbackVersion = $state(0);
	let feedback = $state<WorkFeedback[] | null>(null);
	let feedbackFailed = $state(false);
	$effect(() => {
		const id = workId;
		void feedbackVersion;
		if (!ownerAccess || !id) return;
		let current = true;
		fetchWorkFeedback(companyId, id)
			.then((rows) => {
				if (current) [feedback, feedbackFailed] = [rows, false];
			})
			.catch(() => {
				if (current) feedbackFailed = true;
			});
		return () => (current = false);
	});
	let spend = $state<number | null | 'unavailable'>(null);
	$effect(() => {
		const id = workId;
		void feedbackVersion;
		if (!ownerAccess || !id) return;
		let current = true;
		fetchWorkSpend(companyId)
			.then((rows) => {
				if (current) spend = rows.get(id) ?? 0;
			})
			.catch(() => {
				if (current) spend = 'unavailable';
			});
		return () => (current = false);
	});
	const goalInfo = $derived(goal ? goalAbout(goal) : '');
	const attempts = $derived(
		(graph?.attempts ?? [])
			.filter((attempt) => attempt.work_id === workId)
			.toSorted((a, b) => a.attempt_no - b.attempt_no)
	);
	const latestAttempt = $derived(attempts.at(-1) ?? null);
	const blocker = $derived(
		work?.status === 'blocked' ? blockerSentence(work.resolution, latestAttempt?.summary) : ''
	);
	/* What each run read: the outputs it took as input, and the feedback it was given. */
	function attemptInputs(attemptId: string): string[] {
		if (!graph || !('attempt_inputs' in graph)) return [];
		return graph.attempt_inputs
			.filter((input) => input.attempt_id === attemptId)
			.flatMap((input) => {
				const artifact = graph.artifacts.find((row) => row.id === input.artifact_ref_id);
				return artifact ? [artifactLabel(artifact)] : [];
			});
	}
	const feedbackFor = (attemptId: string | null) =>
		(feedback ?? []).filter((row) => row.attempt_id === attemptId);
	const topology = $derived(
		work && isOwnerWork(work)
			? work.producing_topology === 'locally_closing_parallel_unit'
				? 'One unit of a parallel batch'
				: 'One worker end to end'
			: ''
	);
	const activity = $derived(
		ownerAccess && work && latestAttempt?.state === 'running'
			? workActivityStream(companyId, work.owner_id, workId)
			: null
	);
	$effect(() => activity?.attach());
	const workTurn = $derived.by(() => {
		if (!activity?.live || !latestAttempt) return null;
		return {
			triggerMessageId: latestAttempt.attempt_no,
			since: activity.live.startedAt ?? activity.live.updatedAt,
			live: activity.live,
			transport: activity.transport
		};
	});
	const artifacts = $derived(
		(graph?.artifacts ?? [])
			.filter((artifact) => artifact.work_id === workId)
			.toSorted((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at))
	);
	const workspace = $derived(
		work && isOwnerWork(work)
			? {
					location: work.worktree || work.repo,
					integrationBranch: work.integration_branch
				}
			: null
	);
	const outcomeParts = $derived(splitOutcome(work?.outcome ?? ''));
	/* A settled Work says what happened; unsettled Work keeps its outcome, and a
	 * pause or resume note shows once, as the blocker line under the title. */
	const settledResolution = $derived(
		work && (work.status === 'completed' || work.status === 'abandoned')
			? steerResolution(work.resolution)
			: ''
	);
	const readerSummary = $derived((settledResolution || outcomeParts.opening).trim());
	const readerSummaryLabel = $derived(
		settledResolution ? 'What happened' : 'What this Work delivers'
	);
	const executionContract = $derived(outcomeParts.contract || work?.outcome || '');
	const outcomeIsLong = $derived(executionContract.length > OUTCOME_CLAMP_CHARS);
	$effect(() => {
		/* A different Work is a different contract: never carry one open state
		 * onto the next page. */
		void workId;
		outcomeExpanded = false;
	});
	const gates = $derived(
		graph && 'gates' in graph ? graph.gates.filter((gate) => gate.work_id === workId) : []
	);
	const passedGates = $derived(gates.filter((gate) => gatePassed(gate)).length);
	const unverifiedCompletion = $derived(
		work?.status === 'completed' && artifacts.length === 0 && passedGates === 0
	);
	const prerequisites = $derived(
		(graph?.edges ?? [])
			.filter((edge) => edge.to_work_id === workId && edge.kind === 'requires')
			.flatMap((edge) => graph?.work.filter((item) => item.id === edge.from_work_id) ?? [])
	);
	const dependents = $derived(
		(graph?.edges ?? [])
			.filter((edge) => edge.from_work_id === workId && edge.kind === 'requires')
			.flatMap((edge) => graph?.work.filter((item) => item.id === edge.to_work_id) ?? [])
	);
	const revisions = $derived(
		(graph?.edges ?? [])
			.filter(
				(edge) =>
					edge.kind === 'revises' && (edge.from_work_id === workId || edge.to_work_id === workId)
			)
			.flatMap((edge) => {
				const relatedId = edge.from_work_id === workId ? edge.to_work_id : edge.from_work_id;
				return graph?.work.filter((item) => item.id === relatedId) ?? [];
			})
	);
	const workOwner = $derived(people.find((person) => person.actor_id === work?.owner_id));
	const accountableLeadId = $derived(
		workOwner?.team_id
			? (teams.find((team) => team.id === workOwner.team_id)?.lead_actor_id ?? work?.owner_id)
			: work?.owner_id
	);
	const accountableLead = $derived(
		people.find((person) => person.actor_id === accountableLeadId) ?? null
	);
	const unknownRecovery = $derived(
		ownerAccess &&
			work?.status === 'blocked' &&
			latestAttempt?.state === 'failed' &&
			latestAttempt.summary.includes('productive outcome unknown')
	);
	const recoveryArtifacts = $derived(
		unknownRecovery && latestAttempt
			? artifacts.filter((artifact) => artifact.attempt_id === latestAttempt.id)
			: []
	);
	const preservedCandidate = $derived(
		recoveryArtifacts.find((artifact) => artifact.kind !== 'git_worktree_observation') ??
			recoveryArtifacts.at(0) ??
			null
	);
	const workIsLeadOwned = $derived(!!work?.owner_id && work.owner_id === accountableLeadId);

	function gatePassed(gate: WorkGateRow): boolean {
		if (!latestAttempt || !graph || !('gate_runs' in graph)) return false;
		return (
			graph.gate_runs.some(
				(run) => run.gate_id === gate.id && run.attempt_id === latestAttempt?.id && run.passed
			) ?? false
		);
	}

	function isOwnerWork(item: WorkItem): item is WorkRow {
		return 'owner_review_required' in item;
	}

	function splitOutcome(value: string): { opening: string; contract: string } {
		const boundary = value.indexOf('\n\n');
		if (boundary < 0) return { opening: '', contract: value };
		return {
			opening: value.slice(0, boundary).trim(),
			contract: value.slice(boundary + 2).trim()
		};
	}

	function ownerName(actorId: string): string {
		return (
			people.find((person) => person.actor_id === actorId)?.display ??
			actorId.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
		);
	}

	function backHref(): string {
		const selectedGoal = page.url.searchParams.get('goal') || work?.goal_id || '';
		const query = new URLSearchParams();
		if (page.url.searchParams.get('view') === 'board') query.set('view', 'board');
		if (selectedGoal) query.set('goal', selectedGoal);
		const searched = page.url.searchParams.get('q');
		if (searched) query.set('q', searched);
		const search = query.toString();
		return `/${encodeURIComponent(companyId)}/work${search ? `?${search}` : ''}`;
	}

	/* Talking about this Work keeps where the owner came from: its goal, board view and search. */
	function talkHref(actorId: string): string {
		const url = new URL(page.url);
		url.searchParams.set('talk', actorId);
		return `${url.pathname}${url.search}`;
	}

	function relatedHref(item: WorkItem): string {
		const url = new URL(page.url);
		url.searchParams.set('work', item.id);
		url.searchParams.delete('person');
		return `${url.pathname}${url.search}`;
	}

	const ARTIFACT_STATE_LABEL: Record<string, string> = {
		available: 'Available',
		stale: 'Out of date',
		missing: 'Missing',
		superseded: 'Replaced',
		unknown: 'Not checked yet'
	};
	function artifactState(artifact: Artifact): string {
		return ARTIFACT_STATE_LABEL[artifact.state] ?? artifact.state.replaceAll('_', ' ');
	}

	/* Older Runtime-created artifacts used the whole expected-output contract as
	 * their label. The equality and source kind identify that exact mechanical
	 * path; never shorten a genuinely authored artifact label. */
	function isLegacyAutomaticArtifact(artifact: Artifact): boolean {
		return (
			!!work &&
			'created_by' in artifact &&
			artifact.label === work.expected_artifact &&
			artifact.created_by === work.owner_id &&
			['file', 'repository_tree'].includes(artifact.kind)
		);
	}

	/** Where the Library opens it, when known: the owner's records and a teammate's projection both carry it. */
	function artifactUri(artifact: Artifact): string | null {
		return 'uri' in artifact && typeof artifact.uri === 'string' ? artifact.uri : null;
	}

	function artifactLabel(artifact: Artifact): string {
		return isLegacyAutomaticArtifact(artifact)
			? `Output from: ${work?.title ?? 'this work'}`
			: artifactUri(artifact)
				? fileLabel(artifact.label, artifactUri(artifact)!, work?.title)
				: artifact.label || artifact.kind;
	}

	/* An output the Library can show opens there, in place: a site live, a deck as a slideshow. */
	/* A company file the lead asked for reads as its name and opens in the Library. */
	const expectedPath = $derived(
		/^\/company\/\S+\.[A-Za-z0-9]+$/.test(work?.expected_artifact?.trim() ?? '')
			? work!.expected_artifact!.trim()
			: ''
	);
	function libraryHref(artifact: Artifact): string | null {
		const uri = artifactUri(artifact);
		if (artifact.state !== 'available' || !uri) return null;
		return libraryKind({ kind: artifact.kind, uri, label: artifact.label })
			? `/${encodeURIComponent(companyId)}/library?file=${encodeURIComponent(artifact.id)}`
			: null;
	}

	function artifactNote(artifact: Artifact): string {
		if (!isLegacyAutomaticArtifact(artifact)) {
			if (artifact.note) return artifact.note;
			const uri = artifactUri(artifact);
			const kind = uri ? libraryKind({ kind: artifact.kind, uri, label: artifact.label }) : null;
			const locator = artifactLocator(artifact);
			const name = locator?.split(/[?#]/)[0].split('/').filter(Boolean).at(-1) ?? '';
			return kind
				? `${LIBRARY_KIND_LABEL[kind].replace(/s$/, '')}${name ? ` · ${name}` : ''}`
				: name || 'Linked evidence for this Work';
		}
		return artifact.kind === 'file'
			? 'The exact file produced by this work and observed in the company runtime.'
			: 'The saved result produced by this work; Restless observed it with no uncommitted changes.';
	}

	function artifactLocator(artifact: Artifact): string | null {
		return 'href' in artifact ? artifact.href : artifact.uri;
	}

	function canOpenOutsideCompany(uri: string): boolean {
		try {
			const parsed = new URL(uri);
			return (
				parsed.protocol === 'https:' ||
				(parsed.protocol === 'http:' &&
					!['localhost', '127.0.0.1', '::1'].includes(parsed.hostname))
			);
		} catch {
			return false;
		}
	}

	function displayDate(value: string | null | undefined): string {
		if (!value) return 'Not recorded';
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return value;
		return formatMoment(date);
	}
	holdQueries(() => [
		principalProjection,
		attentionProjection,
		cockpitProjection,
		collaborationProjection
	]);
</script>

<article class="work-detail-screen work-sheet-detail cockpit-pane" aria-label="Work details">
	{#if failure && work}
		<div class="cockpit-error">
			<FailureNotice error={failure} subject="Work" stale={Boolean(work)} onretry={retryWork} />
		</div>
	{/if}

	{#if work}
		<header class="work-detail-head">
			<nav class="work-crumbs" aria-label="Breadcrumb">
				{#if onclose}<button
						class="work-sheet-close"
						type="button"
						aria-label="Close Work details"
						title="Close"
						onclick={onclose}><X size={15} strokeWidth={2} aria-hidden="true" /></button
					>{/if}
				<a href={backHref()}>Work</a>
				<i aria-hidden="true">/</i>
				{#if goal}<a
						href={`/${encodeURIComponent(companyId)}/work?goal=${encodeURIComponent(goal.id)}`}
						>{goal.title}</a
					>{:else}<a href={`/${encodeURIComponent(companyId)}/work?goal=other`}>No goal</a>{/if}
				{#if goalInfo}<InfoTip text={goalInfo} />{/if}
			</nav>
			<div class="work-title-row">
				<h1 style:view-transition-name={`work-title-${work.id}`}>{work.title}</h1>
				<div
					class="work-detail-status status-{work.status}"
					class:unverified={unverifiedCompletion}
				>
					<i aria-hidden="true"></i>
					<span>{unverifiedCompletion ? 'Done · unverified' : workStatusLabel(work.status)}</span>
					{#if unverifiedCompletion}<InfoTip
							text="Completion was recorded, but no output or passing check supports it yet."
						/>{/if}
				</div>
				{#if maySteer}
					<div class="work-steer">
						{#if work.status === 'blocked'}<button
								class="btn small"
								type="button"
								aria-expanded={steering === 'resume'}
								onclick={() => openSteer('resume')}>Resume…</button
							>{/if}
						{#if work.status === 'completed' && !pending.length}<button
								class="btn small"
								type="button"
								title="Send this Work back with the change you want; it reaches the next run"
								aria-expanded={steering === 'reopen'}
								onclick={() => openSteer('reopen')}>Request changes…</button
							>{/if}
						{#if work.status === 'abandoned'}<button
								class="btn small"
								type="button"
								title="Bring this Work back; it continues where it stopped"
								aria-expanded={steering === 'restore'}
								onclick={() => openSteer('restore')}>Restore…</button
							>{/if}
						<!-- The occasional changes sit in one menu, as an issue's do, so a narrow sheet keeps
						     its title and status on one clear line (20 October). -->
						<ActionMenu label="More actions for this Work">
							{#if (work.status === 'proposed' || work.status === 'active') && !pending.length}<button
									type="button"
									title="Hold this Work; Resume continues it"
									onclick={() => openSteer('pause')}>Pause…</button
								>{/if}
							<button type="button" title="Rename this Work or change its brief" onclick={openEdit}
								>Edit…</button
							>
							{#if work.status !== 'completed' && work.status !== 'abandoned'}<button
									type="button"
									title="Retire this Work; nobody works on it again unless it is restored"
									onclick={() => openSteer('abandon')}>Stop for good…</button
								>{/if}
						</ActionMenu>
					</div>
				{/if}
				{#if ownerAccess && !page.url.pathname.endsWith('/people')}
					<div class="work-talk">
						<a class="btn small" href={talkHref('exec')} title="Talk about this Work with the Exec"
							>Discuss</a
						>
						{#if accountableLead && accountableLead.actor_id !== 'exec'}<a
								class="btn small"
								href={talkHref(accountableLead.actor_id)}
								title={`Talk about this Work with ${accountableLead.display}, who leads it`}
								>Ask {accountableLead.display}</a
							>{/if}
					</div>
				{/if}
			</div>
			{#if blocker}<p class="work-blocker">{blocker}</p>{/if}

			{#if pending.length}
				<section class="work-pending" aria-label="Needs you">
					{#each pending as item (item.id)}
						<div class="pending-item">
							<div class="pending-copy">
								<strong>{attentionTitle(item)}</strong>
								{#if item.requestedAction}<p>{item.requestedAction}</p>{/if}
							</div>
							<div class="pending-actions">
								<a class="btn small primary" href={inboxHref(item)}>{isOutcomeReview(item) ? 'Review in Inbox' : 'Open in Inbox'}</a>
							</div>
						</div>
					{/each}
				</section>
			{/if}

			{#if editing && work}
				<form
					class="form work-edit-form"
					onsubmit={(event) => {
						event.preventDefault();
						void saveEdit();
					}}
				>
					<Field label="Title" for="work-edit-title">
						<input id="work-edit-title" bind:value={editTitle} maxlength="200" required />
					</Field>
					<Field label="Brief" for="work-edit-brief">
						<textarea id="work-edit-brief" bind:value={editOutcome} rows="6"></textarea>
					</Field>
					<Field label="Expected output" hint="What it should produce." for="work-edit-output">
						<input id="work-edit-output" bind:value={editExpected} />
					</Field>
					{#if editFailure}<p class="form-error" role="alert">{editFailure}</p>{/if}
					<FormFoot
						submit="Save"
						busy={editBusy}
						busyLabel="Saving…"
						oncancel={() => (editing = false)}
					/>
				</form>
			{/if}

			{#if steering && work}
				<form
					class="work-steer-form"
					onsubmit={(event) => {
						event.preventDefault();
						void steer();
					}}
				>
					{#if steering === 'reassign'}<p class="steer-note">
							Move this Work to <strong>{ownerName(moveTo)}</strong>
						</p>{/if}
					<!-- svelte-ignore a11y_autofocus -->
					<textarea
						bind:value={steerReason}
						rows="2"
						autofocus
						aria-label={STEER[steering].placeholder}
						placeholder={STEER[steering].placeholder}></textarea>
					{#if STEER[steering].warning}<p class="steer-note" role="note">
							{STEER[steering].warning}
						</p>{/if}
					{#if steerFailure}<p class="steer-failure" role="alert">{steerFailure}</p>{/if}
					<div class="steer-actions">
						<button
							class="btn small {steering === 'abandon' ? 'danger' : 'primary'}"
							disabled={steerBusy || !(steerReason.trim() || STEER[steering].fallback)}
							>{steerBusy ? 'Recording…' : STEER[steering].submit}</button
						>
						{#if !steerReason.trim() && !STEER[steering].fallback}<span class="steer-hint"
								>Needs a short reason</span
							>{/if}
						<button class="btn small ghost" type="button" onclick={() => (steering = '')}
							>Cancel</button
						>
					</div>
				</form>
			{/if}

			{#if workTurn}
				<section class="work-live-activity" aria-label="Live Work activity">
					<ConversationTurnDock
						participantName={accountableLead?.display ?? ownerName(work.owner_id)}
						turn={workTurn}
					/>
				</section>
			{/if}
		</header>

		{#if unknownRecovery}
			<section class="recovery-brief" aria-labelledby="recovery-heading">
				<div class="recovery-mark" aria-hidden="true">
					<MatrixGlyph rows={GLYPHS.ring} size={9} />
				</div>
				<div class="recovery-copy">
					<h2 id="recovery-heading">A candidate is preserved. Its outcome is still unknown.</h2>
					<p>
						The cognitive process ended before it reported a trustworthy result. Restless has not
						called this success or failure. {accountableLead?.display ??
							ownerName(accountableLeadId ?? work.owner_id)}
						owns the next judgement: inspect the same candidate, then revise, resume, reassign, or abandon
						it.
					</p>
					{#if preservedCandidate}
						<div class="preserved-candidate">
							<div>
								<h3>Preserved candidate</h3>
								<strong>{preservedCandidate.label || preservedCandidate.kind}</strong>
								<code title="Exact Runtime or external target preserved with this Attempt"
									>{artifactLocator(preservedCandidate)}</code
								>
							</div>
							{#if artifactLocator(preservedCandidate) && canOpenOutsideCompany(artifactLocator(preservedCandidate)!)}
								<a
									class="preserved-link"
									href={artifactLocator(preservedCandidate)!}
									target="_blank"
									rel="noreferrer"
									title="Open this exact preserved target without deciding the Work"
									>Open target ↗</a
								>
							{/if}
						</div>
					{/if}
					<details class="recovery-evidence">
						<summary
							title="Process observations and linked outputs; these support review but do not decide quality"
						>
							Show recovery evidence
						</summary>
						<div>
							<p>{latestAttempt?.summary}</p>
							{#each recoveryArtifacts as artifact (artifact.id)}
								<div class="recovery-artifact">
									<strong>{artifactLabel(artifact)}</strong>
									<code>{artifactLocator(artifact)}</code>
								</div>
							{/each}
						</div>
					</details>
				</div>
			</section>
		{/if}

		<div class="work-detail-scroll">
			<div
				class="work-detail-layout"
				use:resizePane={{
					key: `${companyId}:work-detail`,
					label: 'Resize Work facts pane',
					target: '.work-detail-rail',
					variable: '--work-facts-w',
					side: 'end',
					min: 220,
					minOther: 280,
					defaultSize: 300
				}}
			>
				<section class="work-detail-main">
					<section class="work-description" aria-label={readerSummaryLabel}>
						<h2 class="work-section-label">{readerSummary ? 'Result' : 'Brief'}</h2>
						<div
							class="outcome-body"
							class:clamped={!readerSummary && outcomeIsLong && !outcomeExpanded}
						>
							<Markdown text={readerSummary || executionContract} />
						</div>
						{#if !readerSummary && outcomeIsLong}
							<button
								type="button"
								class="outcome-toggle"
								onclick={() => (outcomeExpanded = !outcomeExpanded)}
								aria-expanded={outcomeExpanded}
							>
								{outcomeExpanded ? 'Show less' : 'Read the full brief'}
							</button>
						{/if}
					</section>

					<section class="work-evidence-section" aria-labelledby="work-evidence-heading">
						<header>
							<h2 id="work-evidence-heading">Evidence</h2>
							<!-- An empty count repeats the sentence below it. -->
							{#if artifacts.length || gates.length}<span class="evidence-score"
									>{artifacts.length} output{artifacts.length === 1 ? '' : 's'} · {ownerAccess
										? gates.length
											? `${passedGates}/${gates.length} checks passed`
											: 'no checks'
										: 'checks owner-visible'}</span
								>{/if}
						</header>
						{#if work.expected_artifact}
							<p class="expected-artifact" title="The output the accountable lead asked for">
								<span>Expected</span>{#if expectedPath}<a
										href={`/${encodeURIComponent(companyId)}/library?path=${encodeURIComponent(expectedPath)}`}
										title={expectedPath}>{expectedPath.split('/').pop()}</a
									>{:else}{work.expected_artifact}{/if}
							</p>
						{/if}
						{#if artifacts.length || gates.length || !ownerAccess}
							<div class="evidence-columns">
								{#if artifacts.length}
									<div class="artifact-list">
										<h3 class="detail-sublabel">Outputs</h3>
										{#each artifacts as artifact (artifact.id)}
											<div class="detail-artifact">
												<MatrixGlyph rows={GLYPHS.work} size={7} />
												<span>
													<strong>{artifactLabel(artifact)}</strong>
													<small>{artifactNote(artifact)}</small>
												</span>
												<div class="artifact-actions">
													<em class:available={artifact.state === 'available'}
														>{artifactState(artifact)}</em
													>
													{#if libraryHref(artifact)}
														<a href={libraryHref(artifact)}>Open</a>
													{:else if artifactLocator(artifact) && canOpenOutsideCompany(artifactLocator(artifact)!)}
														<a href={artifactLocator(artifact)!} target="_blank" rel="noreferrer"
															>Open ↗</a
														>
													{/if}
												</div>
											</div>
										{/each}
									</div>
								{/if}
								{#if ownerAccess && gates.length}
									<div class="gate-list">
										<h3 class="detail-sublabel">Automated checks</h3>
										{#each gates as gate (gate.id)}
											<div class:passed={gatePassed(gate)} class="detail-gate">
												<MatrixGlyph
													rows={gatePassed(gate) ? GLYPHS.check : GLYPHS.ring}
													size={7}
												/>
												<span
													><strong>{gate.name}</strong><small
														>{gatePassed(gate) ? 'Passed' : 'Not passed'}</small
													></span
												>
											</div>
										{/each}
									</div>
								{:else if !ownerAccess}
									<p class="detail-empty">
										Check definitions and raw run output stay in the owner surface.
									</p>
								{/if}
							</div>
						{:else}
							<p class="detail-empty">
								Nothing is linked yet. Outputs and automated checks appear here as the Work runs.
							</p>
						{/if}
					</section>

					<details class="work-technical-details">
						<summary>Run details</summary>
						<div class="work-technical-body">
							{#if readerSummary}<section class="work-detail-section outcome-contract">
									<span
										class="detail-label"
										title="The exact instructions the accountable actor executes. Shown verbatim; Restless never rewrites them."
									>
										Exact execution contract
									</span>
									<div class="outcome-body" class:clamped={outcomeIsLong && !outcomeExpanded}>
										<Markdown text={executionContract} />
									</div>
									{#if outcomeIsLong}
										<button
											type="button"
											class="outcome-toggle"
											onclick={() => (outcomeExpanded = !outcomeExpanded)}
											aria-expanded={outcomeExpanded}
										>
											{outcomeExpanded ? 'Show less' : 'Read the full contract'}
										</button>
									{/if}
								</section>{/if}

							<section class="work-detail-section">
								<div class="detail-section-head">
									<span class="detail-label">Runs</span>
									{#if work && isOwnerWork(work)}<small
											title="How this Work is produced, and how many runs each revision allows"
											>{topology}{work.attempt_limit
												? ` · up to ${work.attempt_limit} run${work.attempt_limit === 1 ? '' : 's'} a revision`
												: ''}</small
										>{/if}
								</div>
								{#each attempts.toReversed() as attempt (attempt.id)}
									{@const inputs = attemptInputs(attempt.id)}
									{@const notes = feedbackFor(attempt.id)}
									<div class="attempt-row">
										<div class="attempt-head">
											<strong class="attempt-state state-{attempt.state}"
												>{runStateLabel(attempt.state)}</strong
											>
											<small>Run {attempt.attempt_no} · revision {attempt.revision}</small>
										</div>
										<p>{attempt.summary || 'This run has not recorded a summary yet.'}</p>
										<div class="attempt-meta">
											<span>Started {displayDate(attempt.started_at)}</span>
											{#if ownerAccess && 'model' in attempt && attempt.model}<span
													>{attempt.model}</span
												>{/if}
										</div>
										{#if inputs.length}<p class="attempt-io">
												<span>Read</span>{inputs.join(' · ')}
											</p>{/if}
										{#each notes as note (note.message_id)}<p class="attempt-io">
												<span>Feedback from {ownerName(note.from_actor)}</span>{note.body}
											</p>{/each}
									</div>
								{:else}
									<p class="detail-empty">No run has started yet.</p>
								{/each}
								{#if feedbackFor(null).length}
									<div class="attempt-row">
										<div class="attempt-head"><strong>For the next run</strong></div>
										{#each feedbackFor(null) as note (note.message_id)}<p class="attempt-io">
												<span>Feedback from {ownerName(note.from_actor)}</span>{note.body}
											</p>{/each}
									</div>
								{/if}
								{#if feedbackFailed}<p class="detail-empty">
										Feedback for this Work could not be read right now.
									</p>{/if}
							</section>

							{#if workspace?.location}
								<section class="work-detail-section">
									<span class="detail-label">Workspace</span>
									<p class="mono workspace-path">
										{workspace.location}{#if workspace.integrationBranch}<br
											/>{workspace.integrationBranch}{/if}
									</p>
								</section>
							{/if}

							<section class="contribution-trace">
								<h2>{workIsLeadOwned ? 'Who decides' : 'Who does what'}</h2>
								{#if workIsLeadOwned}
									<p>
										{accountableLead?.display ?? ownerName(accountableLeadId ?? work.owner_id)} makes
										the final call on this outcome. Each contribution keeps its own record.
									</p>
								{:else}
									<p>
										{ownerName(work.owner_id)} does this part for
										{accountableLead?.display ?? ownerName(accountableLeadId ?? work.owner_id)}, who
										accepts the whole outcome.
									</p>
								{/if}
								{#if work.status === 'abandoned'}
									<p class="contribution-status">
										This Work was stopped and is not presented as accepted output.
									</p>
								{:else if revisions.length}
									<p class="contribution-status">Revisions are shown in the Work graph below.</p>
								{/if}
							</section>
						</div>
					</details>
				</section>

				<div class="work-detail-rail">
					<dl class="work-properties" aria-label="Work properties">
						{#snippet mover(label: string)}
							{#if movable && candidates.length}
								<ActionMenu label={`Move this Work from ${label}`}>
									{#snippet trigger()}<span class="pick-face">{label}</span>{/snippet}
									{#each candidates as person (person.actor_id)}
										<button type="button" onclick={() => openSteer('reassign', person.actor_id)}
											><span>{person.display}</span></button
										>
									{/each}
								</ActionMenu>
							{:else}{label}{/if}
						{/snippet}
						<dt>Lead <InfoTip text="The lead accountable for integrating the whole outcome" /></dt>
						<dd class="pick-cell">
							{#if workIsLeadOwned}{@render mover(
									accountableLead?.display ?? ownerName(accountableLeadId ?? work.owner_id)
								)}{:else}{accountableLead?.display ??
									ownerName(accountableLeadId ?? work.owner_id)}{/if}
						</dd>
						{#if !workIsLeadOwned}
							<dt>Staff <InfoTip text="The Staff member doing this work under the lead" /></dt>
							<dd class="pick-cell">{@render mover(ownerName(work.owner_id))}</dd>
						{/if}
						<dt>Goal</dt>
						<dd class="pick-cell">
							{#if maySteer && goals.length}
								<ActionMenu label="Goal">
									{#snippet trigger()}<span class="pick-face">{goal?.title ?? 'No goal'}</span
										>{/snippet}
									{#each goals.filter((row) => !row.closed_at || row.id === goal?.id) as option (option.id)}
										<button
											type="button"
											disabled={editBusy}
											onclick={() => changeDetails({ goal_id: option.id }, 'Moved to the Goal')}
											><span>{option.title}</span>{#if option.id === goal?.id}<Check
													size={14}
													aria-label="Current"
												/>{/if}</button
										>
									{/each}
									<button
										type="button"
										disabled={editBusy || !goal}
										onclick={() => changeDetails({ goal_id: null }, 'Taken out of its Goal')}
										><span>No goal</span>{#if !goal}<Check
												size={14}
												aria-label="Current"
											/>{/if}</button
									>
								</ActionMenu>
							{:else if goal}<a
									href={`/${encodeURIComponent(companyId)}/work?goal=${encodeURIComponent(goal.id)}`}
									>{goal.title}</a
								>{:else}<a href={`/${encodeURIComponent(companyId)}/work?goal=other`}>No goal</a
								>{/if}
						</dd>
						<dt>Priority <InfoTip text="Higher priority Work is picked up first" /></dt>
						<dd class="pick-cell">
							{#if maySteer}
								<ActionMenu label="Priority">
									{#snippet trigger()}<span class="pick-face"
											>{priorityLabel(work.priority).replace(' priority', '') || 'Normal'}</span
										>{/snippet}
									{#each PRIORITIES as option (option.value)}
										<button
											type="button"
											disabled={editBusy}
											onclick={() =>
												changeDetails({ priority: option.value }, `${option.label} priority`)}
											><span>{option.label}</span
											>{#if Math.sign(work.priority) === option.value}<Check
													size={14}
													aria-label="Current"
												/>{/if}</button
										>
									{/each}
								</ActionMenu>
							{:else}{priorityLabel(work.priority).replace(' priority', '') || 'Normal'}{/if}
							{#if detailFailure}<span class="pick-failure" role="alert">{detailFailure}</span>{/if}
						</dd>
						{#if ownerAccess && spend !== null}
							<dt>Cost <InfoTip text="Metered model spend on this Work so far" /></dt>
							<dd>
								{spend === 'unavailable'
									? 'Unavailable right now'
									: spend > 0
										? `$${spend.toFixed(2)}`
										: 'Nothing metered yet'}
							</dd>
						{/if}
						{#if latestAttempt}
							<dt>Latest run</dt>
							<dd>{runStateLabel(latestAttempt.state)}</dd>
						{/if}
						<dt>Updated</dt>
						<dd>{displayDate(work.updated_at)}</dd>
					</dl>

					{#if prerequisites.length || dependents.length || revisions.length}
						<section class="work-relations-section" aria-label="Related Work">
							{#snippet relation(item: WorkItem, revision = false)}
								<a class:revision href={relatedHref(item)}>
									<i class="relation-dot status-{item.status}" aria-hidden="true"></i>
									<span>{item.title}</span>
									<small>{workStatusLabel(item.status)}</small>
								</a>
							{/snippet}
							{#if prerequisites.length}
								<div class="relation-group">
									<h3 class="detail-sublabel">Requires</h3>
									{#each prerequisites as item (item.id)}{@render relation(item)}{/each}
								</div>
							{/if}
							{#if dependents.length}
								<div class="relation-group">
									<h3 class="detail-sublabel">Hands over to</h3>
									{#each dependents as item (item.id)}{@render relation(item)}{/each}
								</div>
							{/if}
							{#if revisions.length}
								<div class="relation-group">
									<h3 class="detail-sublabel">Revision loop</h3>
									{#each revisions as item (item.id)}{@render relation(item, true)}{/each}
								</div>
							{/if}
						</section>
					{/if}
				</div>
			</div>
		</div>
	{:else if loaded && !error && coordination}
		<FailureNotice error={coordination} subject="this Work" variant="page" onretry={retryWork} />
	{:else if loaded && !error}
		<div class="work-detail-missing">
			<MatrixGlyph rows={GLYPHS.ring} size={14} />
			<h1>Work not found</h1>
			<p>This Work no longer exists in this company.</p>
			<a class="btn small" href={backHref()}>Back to Work</a>
		</div>
	{:else if !error}
		<div class="work-detail-loading">
			<Skeleton label="Loading Work" variant="page" count={5} />
		</div>
	{:else}
		<FailureNotice error={failure} subject="this Work" variant="page" onretry={retryWork} />
	{/if}
</article>

<style>
	.work-sheet-close {
		display: grid;
		flex: none;
		place-items: center;
		width: 26px;
		height: 26px;
		margin-left: -6px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.work-sheet-close:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	/* In a side sheet the detail columns stack. */
	.work-sheet-detail {
		container-type: inline-size;
	}
	@container (max-width: 760px) {
		:global(.bridge-root) .work-detail-layout {
			grid-template-columns: minmax(0, 1fr);
			gap: 20px;
		}
		.work-detail-layout > :global(.pane-resizer) {
			display: none;
		}
		/* The title takes the full width; status and actions wrap beneath it. */
		:global(.bridge-root) .work-title-row {
			flex-wrap: wrap;
			gap: 10px;
		}
		:global(.bridge-root) .work-title-row h1 {
			flex: 1 1 100%;
			font-size: var(--t-head);
		}
	}
	.work-steer {
		display: flex;
		gap: 6px;
	}
	.work-steer-form {
		display: grid;
		gap: 8px;
		max-width: 560px;
		margin: 0;
	}
	.work-steer-form textarea {
		padding: 8px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
		resize: vertical;
	}
	.steer-actions {
		display: flex;
		gap: 6px;
	}
	.work-blocker {
		margin: 4px 0 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.work-pending {
		display: grid;
		gap: 8px;
		max-width: 720px;
		margin: 0;
	}
	.pending-item {
		display: flex;
		flex-wrap: wrap;
		align-items: flex-start;
		gap: 10px 16px;
		padding: 12px 14px;
		border: 1px solid color-mix(in srgb, var(--intent-authority) 45%, var(--border-strong));
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.pending-copy {
		display: grid;
		flex: 1 1 260px;
		gap: 4px;
		min-width: 0;
	}
	.pending-copy p {
		margin: 0;
		color: var(--text-secondary);
	}
	.pending-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.work-edit-form {
		max-width: 720px;
	}
	.workspace-path {
		font-size: var(--t-label);
		overflow-wrap: anywhere;
	}
	/* Why the submit button is not ready yet, in words a touch screen shows. */
	.steer-hint {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.steer-note {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.work-steer {
		flex-wrap: wrap;
	}
	.attempt-row + .attempt-row {
		margin-top: 10px;
		padding-top: 10px;
		border-top: 1px solid var(--border);
	}
	.attempt-head {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}
	.attempt-head small {
		color: var(--text-tertiary);
	}
	.attempt-io {
		margin: 4px 0 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.attempt-io span {
		margin-right: 6px;
		color: var(--text-tertiary);
	}
	.steer-failure {
		margin: 0;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
	.work-section-label {
		margin: 0 0 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 600;
	}
</style>
