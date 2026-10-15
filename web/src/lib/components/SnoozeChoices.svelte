<script lang="ts">
	/* Keep deferral bounded and predictable in every Inbox menu. */
	import { snoozeChoices } from '$lib/model/attention-presentation';
	let { onpick, shortcut = false }: { onpick: (until: Date) => void; shortcut?: boolean } =
		$props();
	const choices = $derived(snoozeChoices().slice(0, 2));
</script>

{#each choices as choice, index (choice.label)}
	<button type="button" onclick={() => onpick(choice.until)}
		>Snooze: {choice.label.toLowerCase()}{#if shortcut && index === 0}
			<kbd>H</kbd>{/if}</button
	>
{/each}

<style>
	kbd {
		margin-left: auto;
		padding: 0 4px;
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-mono);
	}
</style>
