<script lang="ts">
	/* Thinking effort, one company setting (S101), as the slider beside every conversation's model
	 * chip and in a card where an agent proposes a change: Low to Max, the chosen step named on the
	 * right and what it costs under it. It shows where it is dragged; `onchange` fires when let go. */
	import { EFFORTS } from '$lib/model/intelligence.svelte';

	let {
		value,
		disabled = false,
		hint = true,
		onchange
	}: {
		/** The effort chosen now, such as "high". */
		value: string | null | undefined;
		disabled?: boolean;
		/** The chosen step's cost in a line under the slider. */
		hint?: boolean;
		onchange: (effort: string) => void;
	} = $props();

	const id = $props.id();
	let draft = $state<number | null>(null);
	const index = $derived(
		draft ??
			Math.max(
				0,
				EFFORTS.findIndex((row) => row.value === value)
			)
	);
</script>

<label class="effort" title="Applies to every agent from their next session">
	<span class="head"><span>Effort</span><strong>{EFFORTS[index]?.label}</strong></span>
	<input
		type="range"
		min="0"
		max={EFFORTS.length - 1}
		step="1"
		list={`${id}-steps`}
		value={index}
		{disabled}
		aria-valuetext={EFFORTS[index]?.label}
		oninput={(event) => (draft = +event.currentTarget.value)}
		onchange={(event) => {
			const chosen = EFFORTS[+event.currentTarget.value];
			draft = null;
			if (chosen) onchange(chosen.value);
		}}
	/>
	<datalist id={`${id}-steps`}
		>{#each EFFORTS as _, step (step)}<option value={step}></option>{/each}</datalist
	>
	{#if hint}<small>{EFFORTS[index]?.hint}</small>{/if}
</label>

<style>
	.effort {
		display: grid;
		gap: 6px;
		width: 100%;
		max-width: 420px;
	}
	.head {
		display: flex;
		justify-content: space-between;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.head strong {
		color: var(--ink);
		font-weight: 500;
	}
	input {
		width: 100%;
		margin: 0;
		accent-color: var(--primary, var(--intent-conversation));
		cursor: pointer;
	}
	small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
</style>
