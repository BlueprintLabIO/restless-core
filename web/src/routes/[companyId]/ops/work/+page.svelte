<script lang="ts">
	/* The work board, expanded.
	 *
	 * The Ops pane truncates — `doneOlder` is a count of rows it cannot show — which is what
	 * earned "The work" its expand chevron. This is the untruncated board with filters.
	 *
	 * It leads with the work, not with a control panel. An expand page is where a control plane
	 * most easily rots into an administration table, so the filters sit in the pane's own head
	 * rather than as a toolbar the board hangs beneath. */

	import { page } from '$app/state';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import KanbanCard from '$lib/primitives/KanbanCard.svelte';
	import { boardCount, boardOwners, composeWorkBoard, type LaneKey } from '$lib/model/work-board';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
	const hq = desk.hq;
	const owners = boardOwners(hq);

	let ownerId = $state('');
	let lane = $state<LaneKey | ''>('');
	let query = $state('');

	const lanes = $derived(
		composeWorkBoard(hq, {
			ownerId: ownerId || null,
			lane: lane || null,
			query: query || null
		})
	);
	const shown = $derived(boardCount(lanes));
	const total = boardCount(composeWorkBoard(hq));
	const filtered = $derived(Boolean(ownerId || lane || query));
</script>

<svelte:head><title>The work — {desk.company.name}</title></svelte:head>

<div class="bridge-page bridge-bleed bridge-ops">
	<div class="page-head">
		<div style="display: flex; align-items: center; gap: 10px">
			<a class="btn small" href="/{companyId}/ops">‹ Ops</a>
			<h1>The work — every lane</h1>
		</div>
	</div>

	<div class="pane-frame">
		<section class="pane op-pane op-p-work">
			<PaneHeader
				title={filtered ? `Showing ${shown} of ${total}` : `${total} work items`}
				hint="Every lane, untruncated — including the completed work older than a week that the Ops pane can only count."
				hintLabel="What this board shows"
			>
				{#snippet action()}
					<input
						class="chip wk-search"
						type="search"
						placeholder="Find by title"
						aria-label="Find work by title"
						bind:value={query}
					/>
					<select class="chip" bind:value={ownerId} aria-label="Filter by assignee">
						<option value="">Anyone</option>
						{#each owners as owner (owner.id)}<option value={owner.id}>{owner.name}</option>{/each}
					</select>
					<select class="chip" bind:value={lane} aria-label="Filter by lane">
						<option value="">Every lane</option>
						{#each lanes as entry (entry.key)}<option value={entry.key}>{entry.label}</option
							>{/each}
					</select>
				{/snippet}
			</PaneHeader>

			<div class="bridge-kanban">
				{#each lanes as entry (entry.key)}
					<div class="kan-col">
						<div class="kan-head">
							<span>{entry.label}</span>
							<span
								>{entry.cards.length}{entry.olderCount > 0
									? ` (+${entry.olderCount} older)`
									: ''}</span
							>
						</div>
						{#each entry.cards as card (card.id)}
							{#if entry.key === 'needsReview'}
								<a class="kan-card" href="/{companyId}" title="Open the inbox">
									<KanbanCard {card} currency={hq.currency} />
								</a>
							{:else}
								<div class="kan-card">
									<KanbanCard {card} currency={hq.currency} />
								</div>
							{/if}
						{:else}
							<p class="caption" style="padding: 4px">
								{filtered ? 'Nothing here matches.' : 'Empty.'}
							</p>
						{/each}
					</div>
				{/each}
			</div>

			{#if lanes.some((entry) => entry.olderCount > 0)}
				<p class="caption" style="margin-top: 12px">
					Completed work older than a week is counted rather than listed — the full history is on
					the tape.
				</p>
			{/if}
		</section>
	</div>
</div>

<style>
	/* Wider than a chip so a title fragment is actually typeable. */
	.wk-search {
		width: 160px;
	}
</style>
