<script lang="ts">
	/* The React control in a message's hover toolbar: a small picker of the fixed reactions. */
	import SmilePlus from '@lucide/svelte/icons/smile-plus';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { REACTIONS, type ReactionSummary } from '$lib/model/reactions.svelte';

	let {
		reactions,
		onreact
	}: {
		reactions: ReactionSummary[];
		onreact: (emoji: string, on: boolean) => void;
	} = $props();
</script>

<span class="reaction-add" title="React">
	<ActionMenu label="React">
		{#snippet trigger()}<SmilePlus size={14} aria-hidden="true" />{/snippet}
		<div class="picker">
			{#each REACTIONS as emoji (emoji)}
				<button
					type="button"
					aria-label={`React ${emoji}`}
					onclick={() =>
						onreact(emoji, !reactions.find((reaction) => reaction.emoji === emoji)?.mine)}
					>{emoji}</button
				>
			{/each}
		</div>
	</ActionMenu>
</span>

<style>
	.reaction-add {
		display: inline-flex;
	}
	.picker {
		display: flex;
		gap: 2px;
	}
	.picker :global(button) {
		width: 34px !important;
		justify-content: center;
		font-size: var(--t-head) !important;
		transition: transform var(--motion-press) var(--ease-standard);
	}
	.picker :global(button:hover) {
		transform: scale(1.18);
	}
</style>
