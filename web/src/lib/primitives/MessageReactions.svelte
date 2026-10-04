<script lang="ts">
	/* Reaction pills under a message and a quiet way to add one. */
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

<div class="reactions" class:empty={!reactions.length}>
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
	<div class="add">
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
	</div>
</div>

<style>
	.reactions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px;
		margin-top: 4px;
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
		cursor: pointer;
	}
	.pill.mine {
		border-color: color-mix(in srgb, var(--intent-conversation) 45%, var(--border));
		background: color-mix(in srgb, var(--intent-conversation) 10%, var(--surface-raised));
		color: var(--ink);
	}
	.add :global(summary) {
		width: 22px;
		height: 22px;
		border-radius: 999px;
		color: var(--text-tertiary);
	}
	/* With a mouse, the add control waits for the message being pointed at. */
	@media (hover: hover) and (pointer: fine) {
		.add {
			opacity: 0;
			transition: opacity var(--motion-state) var(--ease-standard);
		}
		:global(.conversation-message:hover) .add,
		.add:focus-within,
		.reactions:not(.empty) .add {
			opacity: 1;
		}
	}
	.reactions.empty {
		height: 0;
		margin: 0;
		overflow: visible;
	}
	.reactions.empty .add {
		position: absolute;
		right: 46px;
		top: 6px;
	}
	.picker {
		display: flex;
		gap: 2px;
	}
	.picker :global(button) {
		width: 34px !important;
		justify-content: center;
		font-size: var(--t-head) !important;
	}
</style>
