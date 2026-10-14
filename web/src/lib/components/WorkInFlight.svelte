<script lang="ts">
	/* What Exec has under way, one quiet line each, above the composer: who is on it and whether
	 * it is moving, done, blocked or has gone quiet. A promise to "come back to you" is then never
	 * the only sign that work exists; silence reads as a state the owner can see and ask about. */
	import {
		attentionQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		companyPrincipalQuery
	} from '$lib/model/queries.svelte';
	import { personName } from '$lib/model/initials';
	import { formatMoment } from '$lib/ui/time';
	import { judgementHolders, workWaitingOnOwner } from '$lib/model/attention-presentation';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	/* The owner reads the whole company; everyone else reads the Work they can see, from their
	 * collaboration view, and "Needs you" means handed to them. */
	let { companyId, owner = true }: { companyId: string; owner?: boolean } = $props();
	const attention = $derived(attentionQuery(companyId, () => owner));
	const cockpit = $derived(cockpitQuery(companyId, () => owner));
	const principal = $derived(companyPrincipalQuery(companyId));
	const collaboration = $derived(
		collaborationBootstrapQuery(companyId, () => (owner ? null : principal.view))
	);

	const QUIET_MS = 20 * 60_000;
	const RECENT_MS = 3 * 60 * 60_000;
	let now = $state(Date.now());
	$effect(() => {
		const timer = setInterval(() => (now = Date.now()), 30_000);
		return () => clearInterval(timer);
	});

	type Line = {
		id: string;
		title: string;
		goal: string;
		who: string;
		state: string;
		tone: string;
		at: string;
	};
	type Source = {
		work: {
			id: string;
			title: string;
			status: string;
			updated_at: string;
			owner_id: string;
			goal_id: string | null;
		}[];
		attempts: { work_id: string; state: string; started_at: string }[];
		waiting: Set<string>;
		heldBy: Map<string, string>;
		people: { actor_id: string; display: string }[];
		goals: { id: string; title: string }[];
	};
	const source = $derived.by<Source | null>(() => {
		if (owner) {
			const graph = attention.view?.workGraph;
			if (!graph) return null;
			return {
				work: graph.work,
				attempts: graph.attempts,
				/* Waiting on the owner means what the Inbox shows; a lead or Exec may still hold the judgement. */
				waiting: workWaitingOnOwner(attention.view?.items ?? []),
				heldBy: judgementHolders(graph.handoffs ?? []),
				people: cockpit.view?.people ?? [],
				goals: cockpit.view?.goals ?? []
			};
		}
		const view = collaboration.view;
		if (!view) return null;
		return {
			work: view.work_graph.work,
			attempts: view.work_graph.attempts,
			waiting: new Set(view.attention.handoffs.map((handoff) => handoff.work_id)),
			heldBy: new Map(),
			people: view.people,
			goals: view.goals
		};
	});
	const lines = $derived.by<Line[]>(() => {
		const from = source;
		if (!from) return [];
		const name = (actor: string) =>
			personName(from.people.find((person) => person.actor_id === actor)?.display ?? actor);
		const out: Line[] = [];
		for (const work of from.work) {
			const updated = new Date(work.updated_at).getTime();
			const running = from.attempts.find(
				(attempt) => attempt.work_id === work.id && attempt.state === 'running'
			);
			const waiting = from.waiting.has(work.id);
			const holder = from.heldBy.get(work.id);
			let state = '';
			let tone = '';
			if (work.status === 'completed' && now - updated < RECENT_MS)
				[state, tone] = ['Done', 'done'];
			else if (work.status === 'blocked') [state, tone] = ['Blocked', 'blocked'];
			else if (work.status === 'active' && waiting) [state, tone] = ['Needs you', 'waiting'];
			else if (work.status === 'active' && holder) [state, tone] = [`With ${name(holder)}`, ''];
			else if (work.status === 'active' && running)
				[state, tone] = [`On it since ${formatMoment(running.started_at)}`, 'moving'];
			else if (work.status === 'active' && now - updated > QUIET_MS)
				[state, tone] = [`Quiet since ${formatMoment(work.updated_at)}`, 'quiet'];
			else if (work.status === 'active') [state, tone] = ['Starting', 'moving'];
			if (!state) continue;
			out.push({
				id: work.id,
				title: work.title,
				goal: from.goals.find((goal) => goal.id === work.goal_id)?.title ?? '',
				who: name(work.owner_id),
				state,
				tone,
				at: work.updated_at
			});
		}
		return out.toSorted((a, b) => b.at.localeCompare(a.at)).slice(0, 3);
	});
	holdQueries(() => [attention, cockpit, collaboration]);
</script>

{#if lines.length}
	<section class="in-flight" aria-labelledby="in-flight-label">
		<p class="label" id="in-flight-label">Under way</p>
		<ul>
			{#each lines as line (line.id)}
				<li>
					<!-- The Work's own title leads: many Work items share one Goal, whose long title
					     made every line read the same. The Goal is in the tooltip. -->
					<a
						href={`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(line.id)}`}
						title={[
							line.title,
							line.goal && `Toward ${line.goal}`,
							line.tone === 'quiet' && 'Nothing has moved for a while. Ask Exec about it.'
						]
							.filter(Boolean)
							.join('\n')}
					>
						<i class="dot" data-tone={line.tone} aria-hidden="true"></i>
						<span class="title">{line.title}</span>
						<span class="meta">{line.who} · {line.state}</span>
					</a>
				</li>
			{/each}
		</ul>
	</section>
{/if}

<style>
	/* Never wider than the rail: each line truncates its title, never scrolls sideways. */
	.in-flight {
		min-width: 0;
		margin: 0 10px 6px;
		font-size: var(--t-label);
	}
	.label {
		margin: 0;
		padding: 2px 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
		letter-spacing: 0.02em;
	}
	ul {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 1px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	li {
		min-width: 0;
	}
	a {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		padding: 3px 6px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		text-decoration: none;
	}
	a:hover {
		background: var(--wash-hover);
	}
	.dot {
		flex: none;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-tertiary);
	}
	.dot[data-tone='moving'] {
		background: var(--intent-conversation);
		animation: breathe 2.4s var(--ease-standard) infinite;
	}
	.dot[data-tone='done'] {
		background: var(--state-success);
	}
	.dot[data-tone='waiting'] {
		background: var(--intent-authority);
	}
	.dot[data-tone='blocked'] {
		background: var(--state-danger);
	}
	.title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		color: var(--ink);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.meta {
		flex: 0 1 auto;
		min-width: 0;
		max-width: 45%;
		overflow: hidden;
		color: var(--text-tertiary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	@keyframes breathe {
		50% {
			opacity: 0.35;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.dot[data-tone='moving'] {
			animation: none;
		}
	}
</style>
