<script lang="ts">
	/* A person's Inbox: what the team handed them, the teams they own, and For you (mentions,
	 * replies and changes to what they follow). People own teams and agents run them, so a team's
	 * judgement calls and finished outcomes come here for its person; the server checks every
	 * answer against the team. */
	import { Page, Section, Item, Empty } from '$lib/ui/page';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import ForYou from './ForYou.svelte';
	import type { ForYouSource } from '$lib/model/for-you.svelte';
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import { failureSentence } from '$lib/model/failure';
	import { personName } from '$lib/model/initials';
	import CompanyOffice from '$lib/office/LazyCompanyOffice.svelte';
	import type { WorkGraphSnapshot } from '$lib/model/generated/orgintel';
	import type {
		CollaborationBootstrap,
		CollaborationHandoff,
		CollaborationWork
	} from '$lib/model/collaboration';

	let {
		companyId,
		forYou,
		view,
		onchanged
	}: {
		companyId: string;
		forYou: ForYouSource;
		view: CollaborationBootstrap | null;
		onchanged: () => void;
	} = $props();

	const CATEGORY: Record<string, string> = {
		owner_judgement: 'Your call',
		identity: 'Identity check',
		captcha: 'Human check',
		mfa: 'Sign-in code',
		legal_attestation: 'Legal sign-off',
		payment_confirmation: 'Payment'
	};

	let open = $state('');
	let answer = $state('');
	let busy = $state('');
	/* A refusal shows under the row it is about; what was done is said once in the toast. */
	let failure = $state('');
	let stopping = $state('');

	const me = $derived(view?.principal.actor_id ?? '');
	/* The floor reads the Work a member may see; gates and handoffs beyond theirs are not in it. */
	const floorGraph = $derived(
		view
			? ({
					gates: [],
					gate_runs: [],
					handoffs: [],
					attempt_inputs: [],
					attempt_feedback: [],
					standards: [],
					...view.work_graph
				} as unknown as WorkGraphSnapshot)
			: null
	);
	const handoffs = $derived(view?.attention.handoffs ?? []);
	const myTeams = $derived((view?.teams ?? []).filter((team) => team.owner_person === me));
	const name = (actorId: string) =>
		actorId === 'exec'
			? 'Exec'
			: personName(view?.people.find((person) => person.actor_id === actorId)?.display ?? actorId);
	const workTitle = (workId: string | null) =>
		(workId && view?.work_graph.work.find((work) => work.id === workId)?.title) || '';
	const teamOf = (work: CollaborationWork) =>
		view?.people.find((person) => person.actor_id === work.owner_id)?.team_id ?? null;
	const teamWork = (teamId: string, status: CollaborationWork['status']) =>
		(view?.work_graph.work ?? []).filter(
			(work) => work.status === status && teamOf(work) === teamId
		);
	const isReview = (handoff: CollaborationHandoff) =>
		handoff.category === 'owner_judgement' && handoff.owner_brief?.kind === 'outcome_review';
	const workHref = (workId: string) =>
		`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(workId)}`;

	async function send(path: string, body: object, done: string) {
		busy = path;
		failure = '';
		try {
			const response = await fetch(`/api/companies/${encodeURIComponent(companyId)}/${path}`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify(body)
			});
			if (!response.ok) {
				const detail = (await response.json().catch(() => null)) as { message?: string } | null;
				throw new Error(detail?.message ?? 'That was not recorded. Try again.');
			}
			toast(done);
			open = '';
			answer = '';
			stopping = '';
			onchanged();
		} catch (cause) {
			failure = failureSentence(cause, 'That was not recorded.');
		} finally {
			busy = '';
		}
	}
	const handoffPath = (handoff: CollaborationHandoff, action: string) =>
		`handoffs/${encodeURIComponent(handoff.id)}/${action}`;
	const workPath = (work: CollaborationWork, action: string) =>
		`work/${encodeURIComponent(work.id)}/${action}`;

	function toggle(key: string) {
		open = open === key ? '' : key;
		answer = '';
		failure = '';
		stopping = '';
	}
</script>

{#snippet refusal()}{#if failure}<p class="refusal" role="alert">{failure}</p>{/if}{/snippet}

<Page
	title="Inbox"
	info="What your teams need you to decide, the teams you own, and where people or agents mentioned you, replied to you or changed what you follow."
>
	{#if !view}
		<Empty compact title="Reading your Inbox…" />
	{:else if !handoffs.length && !myTeams.length && !forYou.items.length && forYou.status === 'live' && forYou.filter === 'all'}
		<!-- Archived stays one click away when nothing else is here. -->
		<Empty
			title="Nothing waiting for you"
			info="When a team you own needs a decision, someone hands you a step, mentions you or replies to you, or changes a document you follow, it shows here."
		>
			{#snippet action()}<button
					type="button"
					class="btn small ghost"
					title="For you items you archived"
					onclick={() => forYou.setFilter('archived')}>Archived</button
				>{/snippet}
		</Empty>
	{:else}
		{#if handoffs.length}
			<Section title="Waiting for you" count={handoffs.length}>
				{#each handoffs as handoff (handoff.id)}
					{@const review = isReview(handoff)}
					{@const judgement = handoff.category === 'owner_judgement'}
					<Item
						title={review
							? (handoff.owner_brief?.headline ?? handoff.requested_action)
							: handoff.requested_action}
						meta={[
							review ? 'Outcome to review' : (CATEGORY[handoff.category] ?? 'Step'),
							workTitle(handoff.work_id),
							`from ${name(handoff.escalated_from ?? handoff.requested_by)}`
						]
							.filter(Boolean)
							.join(' · ')}
						onclick={() => toggle(handoff.id)}
						selected={open === handoff.id}
						unread
					>
						{#snippet trailing()}<time title={formatMoment(handoff.created_at)}
								>{formatRelative(handoff.created_at, '')}</time
							>{/snippet}
						{#if open === handoff.id}
							<div class="detail">
								{#if handoff.owner_brief}
									<p class="prepared">{handoff.owner_brief.situation}</p>
									<p class="prepared">
										<strong>Recommended:</strong>
										{handoff.owner_brief.recommendation}
									</p>
									<p class="resume">If nobody decides: {handoff.owner_brief.no_action}</p>
								{:else}
									{#if handoff.prepared_state}<p class="prepared">{handoff.prepared_state}</p>{/if}
									{#if handoff.resume_condition}<p class="resume">
											Then: {handoff.resume_condition}
										</p>{/if}
								{/if}
								<a class="link" href={workHref(handoff.work_id)}>Open the Work</a>
								{#if review}
									<textarea
										bind:value={answer}
										aria-label="Feedback"
										placeholder="Feedback for the team (needed to request changes)"></textarea>
									<div class="bar">
										<button
											class="btn primary small"
											type="button"
											disabled={!!busy}
											onclick={() =>
												send(
													handoffPath(handoff, 'review'),
													{ decision: 'accept', feedback: answer.trim() },
													'Accepted. The Work is done.'
												)}>Accept</button
										>
										<button
											class="btn small"
											type="button"
											disabled={!!busy || !answer.trim()}
											onclick={() =>
												send(
													handoffPath(handoff, 'review'),
													{ decision: 'request_changes', feedback: answer.trim() },
													'Changes requested. The team picks it up.'
												)}>Request changes</button
										>
										<button
											class="btn small quiet"
											type="button"
											disabled={!!busy || !answer.trim()}
											title="Say why; the owner gets your reason"
											onclick={() =>
												send(
													handoffPath(handoff, 'escalate'),
													{ reason: answer.trim() },
													'Passed to the owner.'
												)}>Pass to the owner</button
										>
									</div>
								{:else if judgement}
									<textarea
										bind:value={answer}
										aria-label="Your answer"
										placeholder="Your answer, sent to the team"></textarea>
									<div class="bar">
										<button
											class="btn primary small"
											type="button"
											disabled={!!busy || !answer.trim()}
											onclick={() =>
												send(
													handoffPath(handoff, 'decision'),
													{ resolution: answer.trim() },
													'Answer sent to the team.'
												)}>Send answer</button
										>
										<button
											class="btn small"
											type="button"
											disabled={!!busy || !answer.trim()}
											title="Say why; the team gets your reason"
											onclick={() =>
												send(
													handoffPath(handoff, 'decision'),
													{ resolution: answer.trim(), declined: true },
													'Declined. The team has your reason.'
												)}>Decline</button
										>
										<button
											class="btn small quiet"
											type="button"
											disabled={!!busy || !answer.trim()}
											title="Say why; the owner gets your reason"
											onclick={() =>
												send(
													handoffPath(handoff, 'escalate'),
													{ reason: answer.trim() },
													'Passed to the owner.'
												)}>Pass to the owner</button
										>
									</div>
								{:else if handoff.category !== 'payment_confirmation'}
									<div class="bar">
										<button
											class="btn primary small"
											type="button"
											disabled={!!busy}
											onclick={() =>
												send(
													handoffPath(handoff, 'complete'),
													{},
													'Marked done. The team picks it up from here.'
												)}>Mark done</button
										>
									</div>
								{/if}
								{@render refusal()}
							</div>
						{/if}
					</Item>
				{/each}
			</Section>
		{/if}
		{#each myTeams as team (team.id)}
			{@const blocked = teamWork(team.id, 'blocked')}
			<Section
				title={`Your team: ${team.name}`}
				info={`${name(team.lead_actor_id)} runs it for you · ${team.in_motion_count} in progress · ${team.blocked_count} blocked`}
				count={blocked.length || undefined}
			>
				{#each blocked as work (work.id)}
					{@const key = `work:${work.id}`}
					<Item
						title={work.title}
						meta={['Blocked', name(work.owner_id)].join(' · ')}
						onclick={() => toggle(key)}
						selected={open === key}
					>
						{#if open === key}
							<div class="detail">
								{#if work.resolution}<p class="prepared">{work.resolution}</p>{/if}
								<a class="link" href={workHref(work.id)}>Open the Work</a>
								<textarea
									bind:value={answer}
									aria-label="Reason"
									placeholder="What changed, or why it should stop"></textarea>
								<div class="bar">
									<button
										class="btn primary small"
										type="button"
										disabled={!!busy || !answer.trim()}
										onclick={() =>
											send(workPath(work, 'resume'), { reason: answer.trim() }, 'Resumed.')}
										>Resume</button
									>
									{#if stopping === work.id}
										<span class="ask">Stop for good? The team won’t pick it up again.</span>
										<button
											class="btn small danger"
											type="button"
											disabled={!!busy || !answer.trim()}
											onclick={() =>
												send(
													workPath(work, 'abandon'),
													{ reason: answer.trim() },
													'Stopped. The team will not pick it up again.'
												)}>Stop</button
										>
										<button class="btn small ghost" type="button" onclick={() => (stopping = '')}
											>Keep it</button
										>
									{:else}
										<button
											class="btn small"
											type="button"
											disabled={!!busy || !answer.trim()}
											onclick={() => (stopping = work.id)}>Stop this Work…</button
										>
									{/if}
								</div>
								{@render refusal()}
							</div>
						{/if}
					</Item>
				{:else}
					<Item
						title="Nothing blocked"
						meta={`Talk to ${name(team.lead_actor_id)} or Exec to direct the team`}
					/>
				{/each}
			</Section>
		{/each}
		<Section group>
			<ForYou source={forYou} />
		</Section>
	{/if}
	<!-- The company itself, below what is the person's own: who is at work on what. -->
	{#if view}
		<section class="member-floor" aria-label="Company floor">
			<CompanyOffice {companyId} graph={floorGraph} sourceHealth={{}} collaboration={view} />
		</section>
	{/if}
</Page>

<style>
	.member-floor {
		height: clamp(320px, 52vh, 560px);
		margin-top: 12px;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
	}
	.detail {
		display: grid;
		gap: 10px;
		padding: 4px 0 8px;
	}
	.prepared,
	.resume {
		margin: 0;
		color: var(--text-secondary);
		white-space: pre-wrap;
	}
	.resume {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.link {
		width: fit-content;
		color: var(--accent-ink, var(--text-secondary));
		font-size: var(--t-label);
	}
	textarea {
		min-height: 72px;
		padding: 8px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
		resize: vertical;
	}
	.bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
	}
	.ask {
		color: var(--text-secondary);
	}
	.refusal {
		margin: 0;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
