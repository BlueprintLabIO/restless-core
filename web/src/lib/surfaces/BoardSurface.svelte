<script lang="ts">
	/**
	 * Goals as a tree, tasks as flow. The tree carries the abstraction levels —
	 * objective, goal, step, task — so the columns do not have to; they only
	 * carry where the work has got to.
	 */
	import Avatar from '$lib/components/Avatar.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import Unbacked from '$lib/components/Unbacked.svelte';
	import { isCollapsed } from '$lib/model/dock.svelte';
	import type { Outcome } from '$lib/api/client';
	import type { KanbanColumn, TreeNode } from '$lib/model/view';

	let {
		tree,
		columns,
		outcome
	}: { tree: TreeNode[]; columns: KanbanColumn[]; outcome: Outcome<unknown> } = $props();

	const KIND_ICON = { objective: 'target', goal: 'circle-dot', step: 'circle', task: 'file-text' };

	const STATE = {
		done: { icon: 'check', color: 'var(--status-working)' },
		doing: { icon: 'circle-dot', color: 'var(--status-working)' },
		queued: { icon: 'circle', color: 'var(--text-tertiary)' },
		waiting: { icon: 'hourglass', color: 'var(--status-waiting)' }
	} as const;

	/* The dock costs a column. When she is expanded the secondary column stands
	 * down rather than squeezing all four below a readable width. */
	const visible = $derived(columns.filter((c) => !c.secondary || isCollapsed('board')));

	function glyph(node: TreeNode) {
		if (node.kind === 'task') return { icon: KIND_ICON.task, color: 'var(--text-tertiary)' };
		if (node.kind === 'objective') return { icon: KIND_ICON.objective, color: 'var(--accent)' };
		return STATE[node.state];
	}
</script>

<div class="app-surface">
	<div class="surface-bar">
		<span class="surface-title">Board</span>
		<span class="spacer"></span>
		<button class="btn btn-secondary" type="button">
			Active <Icon name="chevron-down" size={14} />
		</button>
		<button class="btn btn-primary" type="button">+ New task</button>
	</div>

	<div class="board-main">
		<section class="goal-spine" aria-label="Goals">
			<div class="spine-rule"></div>
			<div class="spine-inner">
				<div class="spine-head">
					<span class="over-label spacer">Goals</span>
					<Icon name="chevrons-down-up" size={13} color="var(--text-tertiary)" />
				</div>

				{#if outcome.state !== 'ok'}
					<Unbacked {outcome} what="The board" />
				{/if}

				<div class="tree" role="tree" aria-label="Goals and their breakdown">
					{#each tree as node (node.id)}
						{@const g = glyph(node)}
						<button
							class="tree-row"
							type="button"
							role="treeitem"
							aria-level={node.depth + 1}
							aria-expanded={node.expanded ?? undefined}
							aria-selected={node.selected}
							style:font-weight={node.kind === 'task' ? 400 : 500}
						>
							{#each Array(node.depth) as _, i (i)}
								<span class="tree-indent"></span>
							{/each}

							<span class="tree-chevron">
								{#if node.expanded !== null}
									<Icon name={node.expanded ? 'chevron-down' : 'chevron-right'} size={13} />
								{/if}
							</span>

							<span class="tree-icon"><Icon name={g.icon} size={13} color={g.color} /></span>

							<span
								class="tree-label"
								style:color={node.state === 'done' && node.kind === 'step'
									? 'var(--text-secondary)'
									: 'var(--ink)'}
							>
								{node.label}
							</span>

							{#if node.owner}
								<Avatar initials={node.owner.initials} tint={node.owner.tint} />
							{/if}
							{#if node.meta}
								<span
									class="tree-meta"
									style:color={node.state === 'waiting' ? 'var(--status-waiting)' : null}
								>
									{node.meta}
								</span>
							{/if}
						</button>
					{/each}
				</div>

				<a class="link" href="/board">+ new goal</a>
			</div>
		</section>

		<section class="kanban" aria-label="Tasks">
			<div class="kanban-head">
				<span class="display" style="font-size: 15px">Tasks</span>
				<span class="caption">every commitment, by where it has got to</span>
				<span class="spacer"></span>
				{#if !isCollapsed('board')}
					<span class="caption">collapse the Exec (⌘J) to see completed work</span>
				{/if}
			</div>

			<div class="kanban-cols">
				{#each visible as column (column.id)}
					<div class="col">
						<div class="col-head">
							<span class="over-label" style:color={column.waiting ? 'var(--tone-ask-fg)' : null}>
								{column.name}
							</span>
							<span class="mono" style="color: var(--text-tertiary)">{column.count}</span>
						</div>
						{#if column.note}
							<a class="link" href="/inbox" style="color: var(--tone-ask-fg); font-size: 11.5px">
								{column.note}
							</a>
						{/if}
						{#each column.cards as card (card.id)}
							<button class="kcard" type="button">
								<span class="kcard-title">{card.title}</span>
								<span class="kcard-meta">
									<span class="chip">{card.goal}</span>
									<Avatar initials={card.owner.initials} tint={card.owner.tint} />
								</span>
								<span class="kcard-foot">
									<span>{card.taskId}</span>
									<span>{card.cost}</span>
								</span>
							</button>
						{/each}
					</div>
				{/each}
			</div>
		</section>
	</div>
</div>
