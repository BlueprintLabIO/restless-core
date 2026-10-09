<script lang="ts">
	/* What Exec has under way, one quiet line each, above the composer: who is on it and whether
	 * it is moving, done, blocked or has gone quiet. A promise to "come back to you" is then never
	 * the only sign that work exists; silence reads as a state the owner can see and ask about. */
	import { attentionQuery, cockpitQuery } from '$lib/model/queries.svelte';
	import { personName } from '$lib/model/initials';
	import { formatMoment } from '$lib/ui/time';

	let { companyId }: { companyId: string } = $props();
	const attention = $derived(attentionQuery(companyId));
	const cockpit = $derived(cockpitQuery(companyId));

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
	const lines = $derived.by<Line[]>(() => {
		const graph = attention.view?.workGraph;
		if (!graph) return [];
		const name = (actor: string) =>
			personName(cockpit.view?.people.find((person) => person.actor_id === actor)?.display ?? actor);
		const out: Line[] = [];
		for (const work of graph.work) {
			const updated = new Date(work.updated_at).getTime();
			const running = graph.attempts.find(
				(attempt) => attempt.work_id === work.id && attempt.state === 'running'
			);
			const waiting = graph.handoffs?.some(
				(handoff) =>
					handoff.work_id === work.id && (handoff.state === 'pending' || handoff.state === 'preparing')
			);
			let state = '';
			let tone = '';
			if (work.status === 'completed' && now - updated < RECENT_MS) [state, tone] = ['Done', 'done'];
			else if (work.status === 'blocked') [state, tone] = ['Blocked', 'blocked'];
			else if (work.status === 'active' && waiting) [state, tone] = ['Waiting on you', 'waiting'];
			else if (work.status === 'active' && running)
				[state, tone] = [`On it since ${formatMoment(running.started_at)}`, 'moving'];
			else if (work.status === 'active' && now - updated > QUIET_MS)
				[state, tone] = [`Quiet since ${formatMoment(work.updated_at)}`, 'quiet'];
			else if (work.status === 'active') [state, tone] = ['Starting', 'moving'];
			if (!state) continue;
			out.push({
				id: work.id,
				title: work.title,
				goal: cockpit.view?.goals.find((goal) => goal.id === work.goal_id)?.title ?? '',
				who: name(work.owner_id),
				state,
				tone,
				at: work.updated_at
			});
		}
		return out.toSorted((a, b) => b.at.localeCompare(a.at)).slice(0, 3);
	});
</script>

{#if lines.length}
	<ul class="in-flight" aria-label="Work under way">
		{#each lines as line (line.id)}
			<li>
				<a
					href={`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(line.id)}`}
					title={line.tone === 'quiet'
						? `Nothing has moved for a while. Ask Exec about “${line.title}”.`
						: line.title}
				>
					<i class="dot" data-tone={line.tone} aria-hidden="true"></i>
					<span class="title"
						>{#if line.goal}<span class="goal" title={`Toward ${line.goal}`}>{line.goal} ›</span>
						{/if}{line.title}</span
					>
					<span class="who">{line.who}</span>
					<span class="state">{line.state}</span>
				</a>
			</li>
		{/each}
	</ul>
{/if}

<style>
	.in-flight {
		display: grid;
		gap: 1px;
		margin: 0 10px 6px;
		padding: 0;
		list-style: none;
		font-size: var(--t-label);
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
	.goal {
		color: var(--text-tertiary);
	}
	.who,
	.state {
		flex: none;
		color: var(--text-tertiary);
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
