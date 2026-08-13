<script lang="ts">
	/* Every standing grant.
	 *
	 * The Mission pane shows 14 chips and counts the rest behind a fold. At this scale that is a
	 * page: a fold makes you expand a wall of chips inside a pane sized for a summary, with no
	 * way to ask "what can anyone do without my word?" — which is the actual question. */

	import { page } from '$app/state';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import { composeAuthorityBoard, countGrants } from '$lib/model/authority-board';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);

	let subject = $state<'member' | 'staff' | ''>('');
	let approvalOnly = $state(false);
	let query = $state('');

	const groups = $derived(
		composeAuthorityBoard(desk.authority, {
			subject: subject || null,
			approvalOnly,
			query: query || null
		})
	);
	const counts = $derived(countGrants(groups));
	const all = countGrants(composeAuthorityBoard(desk.authority));
	const filtered = $derived(Boolean(subject || approvalOnly || query));
</script>

<svelte:head><title>Standing authority — {desk.company.name}</title></svelte:head>

<div class="bridge-page bridge-bleed bridge-authority">
	<div class="page-head">
		<div style="display: flex; align-items: center; gap: 10px">
			<a class="btn small" href="/{companyId}/mission">‹ Mission</a>
			<h1>Standing authority</h1>
		</div>
	</div>

	<div class="pane-frame">
		<section class="pane au-pane au-p-summary">
			<PaneHeader
				title={filtered ? `${counts.total} of ${all.total} grants` : `${all.total} standing grants`}
				hint="A standing grant is what may happen without asking. An approval-gated one still needs a human's word each time — both are recorded, and every act under either lands on the tape."
				hintLabel="What a standing grant is"
			>
				{#snippet action()}
					<input
						class="chip au-search"
						type="search"
						placeholder="Find a capability"
						aria-label="Find a capability"
						bind:value={query}
					/>
					<select class="chip" bind:value={subject} aria-label="Filter by who holds it">
						<option value="">Anyone</option>
						<option value="staff">Employees</option>
						<option value="member">People</option>
					</select>
					<label class="chip" class:on={approvalOnly}>
						<input class="sr-only" type="checkbox" bind:checked={approvalOnly} />
						Needs your word
					</label>
				{/snippet}
			</PaneHeader>
			<p class="caption">
				{counts.approval} of these need your word before anything happens; the other
				{counts.total - counts.approval} are autonomous inside their limits.
			</p>
		</section>

		{#each groups as group (group.domain)}
			<section class="pane au-pane">
				<PaneHeader title={group.domain} />
				<div class="au-grid">
					{#each group.rows as row (row.id)}
						<div class="au-row">
							<span class="au-action">{row.action}</span>
							<span class="au-who caption">{row.subject === 'staff' ? 'employee' : 'person'}</span>
							<span class="mi-chip" class:approval={row.needsApproval}>
								{row.needsApproval ? 'needs your word' : 'standing'}
							</span>
						</div>
					{/each}
				</div>
			</section>
		{:else}
			<section class="pane au-pane">
				<p class="caption">
					{filtered
						? 'No grant matches — loosen the filters.'
						: 'No standing grants. Nothing acts on its own.'}
				</p>
			</section>
		{/each}
	</div>
</div>

<style>
	.au-search {
		width: 150px;
	}
	.au-grid {
		display: flex;
		flex-direction: column;
	}
	/* One row per grant, not a chip cloud: at this count the eye needs a column to run down,
	 * and the mode has to sit somewhere consistent to be scannable. */
	.au-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 90px 130px;
		align-items: center;
		gap: 12px;
		padding: 7px 0;
		border-bottom: 1px solid var(--border);
	}
	.au-row:last-child {
		border-bottom: 0;
	}
	.au-action {
		font-size: 13px;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.au-who {
		white-space: nowrap;
	}
	@media (max-width: 640px) {
		.au-row {
			grid-template-columns: minmax(0, 1fr) 120px;
		}
		.au-who {
			display: none;
		}
	}
</style>
