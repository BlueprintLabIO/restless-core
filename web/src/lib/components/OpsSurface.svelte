<script lang="ts">
	/* The Ops surface: the business seen through its processes — money and runway
	 * up top, the work in lanes, where the money goes, and what is actually wired.
	 * The people who do the work live on PeopleSurface; this answers "what is
	 * happening in my company?", not "how is each person doing?".
	 *
	 * Takes already-mapped view models rather than a raw desk, so the founding
	 * floor can reuse it against a draft view. `drift` is absent there — the draft
	 * has no strategy to drift from — which is why it defaults to null. */

	import Hint from '$lib/primitives/Hint.svelte';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import KanbanCard from '$lib/primitives/KanbanCard.svelte';
	/* The component and the type share a name; the type is aliased so the import of the
	 * component can keep the obvious one. */
	import type { HqView, ConnectionRow, KanbanCard as KanbanCardData } from '$lib/model/view';
	import MatrixGlyph, { GLYPHS, type GlyphName } from '$lib/primitives/MatrixGlyph.svelte';

	let {
		hq,
		connections = [],
		companyName,
		companyId,
		drift = null,
		draft = false
	}: {
		hq: HqView;
		/**
		 * What is actually wired. "Probe, never guess": these rows must come from a
		 * live check against the real runtime or connector, which is exactly why they
		 * arrive as a prop rather than being inferred here. Empty means nothing has
		 * been probed — which the pane says plainly rather than implying all is well.
		 */
		connections?: ConnectionRow[];
		companyName: string;
		companyId: string;
		drift?: {
			orphanedWork: unknown[];
			contradictoryDescendants: unknown[];
		} | null;
		/** True on the founding floor: there is no company yet to deep-link into. */
		draft?: boolean;
	} = $props();

	function money(cents: number, currency: string): string {
		return new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(cents / 100);
	}

	const lanes = $derived([
		{ key: 'queued', label: 'Queued', cards: hq.columns.queued },
		{ key: 'inProgress', label: 'In progress', cards: hq.columns.inProgress },
		{ key: 'needsReview', label: 'Needs review', cards: hq.columns.needsReview },
		{ key: 'stuck', label: 'Stuck', cards: hq.columns.stuck },
		{ key: 'doneRecent', label: 'Done this week', cards: hq.columns.doneRecent }
	] as Array<{ key: string; label: string; cards: KanbanCardData[] }>);

	const spendShare = $derived(hq.team.reduce((total, member) => total + member.spendCents, 0) || 1);

	/* Work waiting on a human deep-links into the inbox — the queue that needs
	 * the operator's word lives there, not on this board. Draft mode (founding)
	 * has no inbox yet, so its cards stay inert. */
	const inboxHref = $derived(!draft ? `/${companyId}` : null);

	/* The two panes that truncate get an expand route. The founding floor has no
	 * company to route into, so its panes stay flat. */
	const workHref = $derived(`/${companyId}/ops/work`);
	const spendHref = $derived(`/${companyId}/ops/spend`);

	/* Statuses render raw (never paraphrased); the glyph only reinforces what the
	 * word already says (design-language L6: status is shape first). */
	const glyphOf = (row: ConnectionRow): GlyphName =>
		row.ok ? 'check' : row.failed ? 'cross' : 'ring';
	const connectionRows = $derived(connections);
</script>

<div class="bridge-page bridge-bleed bridge-ops">
	<div class="page-head">
		<h1>Ops — {companyName}</h1>
		{#if !draft}
			<a class="btn small" href="/{companyId}">Open inbox</a>
		{/if}
	</div>

	<div class="pane-frame">
		<div class="metric-row">
			<div class="metric">
				<div class="m-label">Cash on hand</div>
				<div class="m-value">{money(hq.cashCents, hq.currency)}</div>
				<div class="m-sub">recorded treasury balance</div>
			</div>
			<div class="metric">
				<div class="m-label">Spend this month</div>
				<div class="m-value">{money(hq.spendCents, hq.currency)}</div>
				<div class="m-sub">of {money(hq.budgetCents, hq.currency)} budget</div>
			</div>
			<div class="metric">
				<!-- The runway caveat was buried in a raw `title=`. A financial projection's
			     limits must not be less discoverable than the projection, so it now has a
			     real, focusable trigger on the label itself. -->
				<div class="m-label">
					Runway<Hint text={hq.runway.disclaimer} label="How runway is estimated" />
				</div>
				<div class="m-value">
					{hq.runway.months != null ? `${hq.runway.months} mo` : '—'}
				</div>
				<div class="m-sub">
					{hq.runway.months != null ? hq.runway.assumption : (hq.runway.reason ?? '')}
				</div>
			</div>
			<div class="metric" class:lit={hq.needsYou > 0}>
				<div class="m-label">Needs you</div>
				<div class="m-value">{hq.needsYou}</div>
				<div class="m-sub">decisions &amp; approvals</div>
			</div>
			<div class="metric">
				<div class="m-label">Active runs</div>
				<div class="m-value">{hq.activeRuns}</div>
				<div class="m-sub">executing right now</div>
			</div>
		</div>

		{#if drift && (drift.orphanedWork.length > 0 || drift.contradictoryDescendants.length > 0)}
			<section class="pane op-pane op-p-drift">
				<PaneHeader title="Strategy drift" />
				<p class="form-error" style="margin: 0">
					{#if drift.orphanedWork.length > 0}
						{drift.orphanedWork.length} open work item{drift.orphanedWork.length === 1 ? '' : 's'} lost
						their owner.
					{/if}
					{#if drift.contradictoryDescendants.length > 0}
						{drift.contradictoryDescendants.length} still descend from a withdrawn directive.
					{/if}
				</p>
			</section>
		{/if}

		<div class="pane-row op-body">
			<section class="pane op-pane op-p-work">
				<PaneHeader title="The work" href={draft ? null : workHref} />
				<div class="bridge-kanban">
					{#each lanes as lane (lane.key)}
						<div class="kan-col">
							<div class="kan-head">
								<span>{lane.label}</span>
								<span
									>{lane.cards.length}{lane.key === 'doneRecent' && hq.doneOlder > 0
										? ` (+${hq.doneOlder} older)`
										: ''}</span
								>
							</div>
							{#each lane.cards as card (card.id)}
								{#if lane.key === 'needsReview' && inboxHref}
									<a class="kan-card" href={inboxHref} title="Open the inbox">
										<KanbanCard {card} currency={hq.currency} />
									</a>
								{:else}
									<div class="kan-card">
										<KanbanCard {card} currency={hq.currency} />
									</div>
								{/if}
							{:else}
								<p class="caption" style="padding: 4px">Empty.</p>
							{/each}
						</div>
					{/each}
				</div>
			</section>

			<div class="pane-rail">
				<section class="pane op-pane op-p-spend">
					<PaneHeader
						title="Where the money goes"
						hint="Unmetered local runs record 0 — measured-as-nothing, not free."
						hintLabel="How run spend is recorded"
						href={draft ? null : spendHref}
					/>
					{#each hq.team as member (member.id)}
						<div style="margin-bottom: 10px">
							<div class="kv" style="padding-bottom: 2px">
								<span>{member.name}</span>
								<b
									>{money(member.spendCents, hq.currency)}
									<span class="caption">/ {money(member.limitCents, hq.currency)}</span></b
								>
							</div>
							<div class="bar">
								<span
									style={`width: ${Math.min(100, Math.round((member.spendCents / spendShare) * 100))}%; background: var(--pig-${member.pig})`}
								></span>
							</div>
						</div>
					{:else}
						<p class="caption">Nothing recorded yet.</p>
					{/each}
				</section>

				<section class="pane op-pane op-p-conn">
					<PaneHeader
						title="Connections"
						hint="Probed, never guessed — a status here comes from a live check against the real thing."
						hintLabel="How connection status is established"
					/>
					<div class="conn-table">
						<div class="conn-lh mono" aria-hidden="true">
							<span>name</span><span>kind</span><span>status</span><span class="r"
								>last checked</span
							>
						</div>
						{#each connectionRows as row (row.key)}
							<div class="conn-row">
								<span class="conn-name">{row.name}</span>
								<span class="conn-kind mono">{row.kind}</span>
								<span class="conn-status mono" class:lit={row.ok}>
									<MatrixGlyph rows={GLYPHS[glyphOf(row)]} size={9} />{row.status}
								</span>
								<span class="conn-when mono">{row.when}</span>
							</div>
						{:else}
							<p class="caption">
								Nothing connected yet — the mission's registry is the truth of that.
							</p>
						{/each}
					</div>
				</section>
			</div>
		</div>
	</div>
</div>

<style>
	/* connections — the live-truth ledger. It is a table inside the pane now rather than
	 * a borderless card pretending to be one, so it carries its own hairline frame. */
	.conn-table {
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		overflow: hidden;
	}
	.conn-lh,
	.conn-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 150px 140px 130px;
		align-items: center;
		gap: 12px;
		padding: 0 14px;
	}
	.conn-lh {
		height: 30px;
		font-size: 10px;
		letter-spacing: 0.12em;
		text-transform: uppercase;
		color: var(--text-tertiary);
		border-bottom: 1px solid var(--border-strong);
	}
	.conn-lh .r {
		text-align: right;
	}
	.conn-row {
		height: 30px;
		border-bottom: 1px solid var(--border);
	}
	.conn-row:last-child {
		border-bottom: 0;
	}
	.conn-name {
		font-size: 12.5px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.conn-kind {
		font-size: 11px;
		color: var(--text-tertiary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.conn-status {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 11px;
		color: var(--text-secondary);
	}
	.conn-status.lit {
		color: var(--ink);
	}
	.conn-when {
		font-size: 11px;
		color: var(--text-tertiary);
		text-align: right;
		white-space: nowrap;
	}
	@media (max-width: 720px) {
		.conn-lh,
		.conn-row {
			grid-template-columns: minmax(0, 1fr) 120px 110px;
		}
		.conn-kind,
		.conn-lh span:nth-child(2) {
			display: none;
		}
	}
</style>
