<script lang="ts">
	/* The reactions a message already has, as pills under it. Adding one lives in the message's
	 * hover toolbar (ReactionAdd), so nothing waits on top of the text. */
	import type { ReactionSummary } from '$lib/model/reactions.svelte';

	let {
		reactions,
		onreact
	}: {
		reactions: ReactionSummary[];
		onreact: (emoji: string, on: boolean) => void;
	} = $props();
</script>

{#if reactions.length}
	<div class="reactions">
		{#each reactions as reaction (reaction.emoji)}
			<button
				type="button"
				class="pill"
				class:mine={reaction.mine}
				aria-pressed={reaction.mine}
				title={reaction.mine ? 'Remove your reaction' : 'Add this reaction'}
				onclick={() => onreact(reaction.emoji, !reaction.mine)}
				><span aria-hidden="true">{reaction.emoji}</span><span>{reaction.count}</span></button
			>
		{/each}
	</div>
{/if}

<style>
	.reactions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px;
		margin-top: 6px;
	}
	.pill {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		height: 22px;
		padding: 0 7px;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: var(--surface-raised);
		color: var(--text-secondary);
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		cursor: pointer;
		transition:
			border-color var(--motion-state) var(--ease-standard),
			background var(--motion-state) var(--ease-standard),
			transform var(--motion-press) var(--ease-standard);
	}
	.pill:hover {
		border-color: var(--border-strong);
	}
	.pill:active {
		transform: scale(0.94);
	}
	.pill.mine {
		border-color: color-mix(in srgb, var(--intent-conversation) 45%, var(--border));
		background: color-mix(in srgb, var(--intent-conversation) 10%, var(--surface-raised));
		color: var(--ink);
	}
</style>
