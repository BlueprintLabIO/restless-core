<script lang="ts">
	/* One card on the work board.
	 *
	 * Extracted from OpsSurface's local snippet (UIR-011) so the Ops pane and the expanded
	 * /ops/work board render a card the same way. Two copies of this markup would drift, and a
	 * card that reads differently either side of an expand arrow makes the arrow feel like it
	 * went somewhere else. */

	import { initialsOf, type KanbanCard } from '$lib/model/view';

	let {
		card,
		currency
	}: {
		card: KanbanCard;
		currency: string;
	} = $props();

	function money(cents: number): string {
		return new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(cents / 100);
	}
</script>

<div class="k-title">{card.title}</div>
{#if card.stateReason}
	<div class="caption" style="margin-top: 3px">{card.stateReason}</div>
{/if}
<div class="k-foot">
	<span style="display: flex; align-items: center; gap: 6px; min-width: 0">
		{#if card.ownerName}
			<span class="avatar sm" style={`background: var(--pig-${card.ownerPig})`}
				>{initialsOf(card.ownerName)}</span
			>
			<span class="caption" style="white-space: nowrap; overflow: hidden; text-overflow: ellipsis"
				>{card.ownerName}</span
			>
		{/if}
	</span>
	<!-- An unmetered run records 0, so no figure is shown rather than a confident "$0.00" —
	     measured-as-nothing is not the same claim as free. -->
	<span class="k-id">{card.costCents > 0 ? money(card.costCents) : ''}</span>
</div>
