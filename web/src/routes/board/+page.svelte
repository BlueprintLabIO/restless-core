<script lang="ts">
	import AppShell from '$lib/components/AppShell.svelte';
	import BoardSurface from '$lib/surfaces/BoardSurface.svelte';
	import { getCommitments, getGoals, type Outcome } from '$lib/api/client';
	import { toColumns, toGoalTree } from '$lib/api/map';
	import type { KanbanColumn, TreeNode } from '$lib/model/view';

	let tree = $state<TreeNode[]>([]);
	let columns = $state<KanbanColumn[]>([]);
	let outcome = $state<Outcome<unknown>>({ state: 'ok', data: null });

	$effect(() => {
		let cancelled = false;
		(async () => {
			const [goals, commitments] = await Promise.all([getGoals(), getCommitments()]);
			if (cancelled) return;
			// One failure is the surface's failure: a board of goals with no
			// commitments, or the reverse, is a misleading half-answer.
			if (goals.state !== 'ok') return (outcome = goals);
			if (commitments.state !== 'ok') return (outcome = commitments);
			outcome = { state: 'ok', data: null };
			tree = toGoalTree(goals.data, commitments.data);
			columns = toColumns(commitments.data);
		})();
		return () => {
			cancelled = true;
		};
	});
</script>

<svelte:head><title>Board</title></svelte:head>

<AppShell surface="board">
	<BoardSurface {tree} {columns} {outcome} />
</AppShell>
