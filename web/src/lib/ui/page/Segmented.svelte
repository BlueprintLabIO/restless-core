<script lang="ts" generics="T extends string">
	/* A small set of mutually exclusive choices shown at once, such as a
	 * view switch. Arrow keys move between options, as with native radios. */
	let {
		options,
		value = $bindable(),
		label,
		disabled = false,
		onchange
	}: {
		options: readonly { value: T; label: string; title?: string }[];
		value: T;
		label: string;
		disabled?: boolean;
		/** Called after `value` changes; forms usually just bind `value`. */
		onchange?: (value: T) => void;
	} = $props();

	function choose(next: T) {
		if (next === value) return;
		value = next;
		onchange?.(next);
	}
	function key(event: KeyboardEvent, index: number) {
		const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[event.key];
		if (!step) return;
		event.preventDefault();
		const next = options[(index + step + options.length) % options.length];
		choose(next.value);
		const group = (event.currentTarget as HTMLElement).parentElement;
		requestAnimationFrame(() =>
			group?.querySelector<HTMLButtonElement>('[aria-checked="true"]')?.focus()
		);
	}
</script>

<div class="segmented" role="radiogroup" aria-label={label}>
	{#each options as option, index (option.value)}
		<button
			type="button"
			role="radio"
			aria-checked={option.value === value}
			tabindex={option.value === value ? 0 : -1}
			title={option.title}
			{disabled}
			onclick={() => choose(option.value)}
			onkeydown={(event) => key(event, index)}>{option.label}</button
		>
	{/each}
</div>

<style>
	.segmented {
		display: inline-flex;
		max-width: 100%;
		overflow-x: auto;
		scrollbar-width: none;
		gap: 2px;
		padding: 2px;
		border: 1px solid var(--border-strong);
		border-radius: calc(var(--radius-control) + 2px);
		background: var(--surface-alt);
	}
	button {
		flex: none;
		white-space: nowrap;
		min-height: 26px;
		padding: 0 12px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-body);
		font-weight: 500;
		cursor: pointer;
		transition:
			background-color var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard);
	}
	button:hover:not(:disabled) {
		color: var(--ink);
	}
	button[aria-checked='true'] {
		background: var(--segment-thumb);
		box-shadow: var(--control-depth);
		color: var(--ink);
	}
	button:disabled {
		cursor: not-allowed;
		opacity: 0.55;
	}
	@media (pointer: coarse) {
		button {
			min-height: 40px;
		}
	}
</style>
