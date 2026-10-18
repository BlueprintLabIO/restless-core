<script lang="ts" generics="T extends string">
	/* One choice among a few, as a single switch (style/forms.css). Arrow keys move between
	 * options, as in any radio group. */
	let {
		options,
		value = $bindable(),
		label,
		disabled = false,
		onchange = undefined
	}: {
		options: readonly { value: T; label: string; title?: string }[];
		value: T;
		/** What the choice is, for assistive technology. */
		label: string;
		disabled?: boolean;
		onchange?: (value: T) => void;
	} = $props();

	let group: HTMLDivElement | undefined = $state();
	function choose(next: T) {
		if (next === value) return;
		value = next;
		onchange?.(next);
	}
	function keydown(event: KeyboardEvent) {
		const step =
			event.key === 'ArrowRight' || event.key === 'ArrowDown'
				? 1
				: event.key === 'ArrowLeft' || event.key === 'ArrowUp'
					? -1
					: 0;
		if (!step) return;
		event.preventDefault();
		const index = options.findIndex((option) => option.value === value);
		const next = options[(index + step + options.length) % options.length];
		choose(next.value);
		group?.querySelectorAll('button')[options.indexOf(next)]?.focus();
	}
</script>

<div
	class="segmented"
	role="radiogroup"
	tabindex="-1"
	aria-label={label}
	bind:this={group}
	onkeydown={keydown}
>
	{#each options as option (option.value)}
		<button
			type="button"
			role="radio"
			aria-checked={option.value === value}
			tabindex={option.value === value ? 0 : -1}
			title={option.title}
			{disabled}
			onclick={() => choose(option.value)}>{option.label}</button
		>
	{/each}
</div>
