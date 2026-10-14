<script lang="ts">
	/* Snooze until a chosen time, inside a ⋯ menu: later today, tomorrow morning, next week, or
	 * a picked day (nine in the morning there). */
	import { snoozeChoices, snoozeDay } from '$lib/model/attention-presentation';

	let { onpick, shortcut = false }: { onpick: (until: Date) => void; shortcut?: boolean } =
		$props();
	const choices = $derived(snoozeChoices());
	const tomorrow = $derived.by(() => {
		const at = new Date();
		at.setDate(at.getDate() + 1);
		return `${at.getFullYear()}-${String(at.getMonth() + 1).padStart(2, '0')}-${String(at.getDate()).padStart(2, '0')}`;
	});
</script>

{#each choices as choice, index (choice.label)}
	<button type="button" onclick={() => onpick(choice.until)}
		>Snooze: {choice.label.toLowerCase()}{#if shortcut && index === 0}
			<kbd>H</kbd>{/if}</button
	>
{/each}
<label class="snooze-day" title="Back in your Inbox at nine that morning"
	>Snooze until <input
		type="date"
		min={tomorrow}
		onchange={(event) => {
			const until = snoozeDay(event.currentTarget.value);
			if (until) onpick(until);
		}}
	/></label
>

<style>
	.snooze-day {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.snooze-day input {
		min-width: 0;
		font: inherit;
	}
	kbd {
		margin-left: auto;
		padding: 0 4px;
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-mono);
	}
</style>
