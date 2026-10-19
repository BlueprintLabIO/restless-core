<script module lang="ts">
	export type ChoiceOption<V extends string> = {
		value: V;
		title: string;
		detail?: string;
		/** A short state on the right: "In use", "Offline". */
		mark?: string;
		disabled?: boolean;
	};
</script>

<script lang="ts" generics="T extends string">
	/* The one way an Inbox card offers choices (S101): fixed-height rows, each a radio with a
	 * title, an optional detail and an optional mark on the right. Choosing never acts; the
	 * footer's primary action names and does what is chosen. */

	let {
		label,
		options,
		value = $bindable()
	}: { label: string; options: ChoiceOption<T>[]; value: T | null } = $props();

	const name = $props.id();
</script>

<div class="choice" role="radiogroup" aria-label={label}>
	{#each options as option (option.value)}
		<label class="row" class:on={value === option.value} class:off={option.disabled}>
			<input
				type="radio"
				{name}
				value={option.value}
				checked={value === option.value}
				disabled={option.disabled}
				onchange={() => (value = option.value)}
			/>
			<span class="text">
				<span class="title">{option.title}</span>
				{#if option.detail}<span class="detail">{option.detail}</span>{/if}
			</span>
			{#if option.mark}<span class="mark">{option.mark}</span>{/if}
		</label>
	{/each}
</div>

<style>
	.choice {
		display: grid;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		overflow: hidden;
	}
	.row {
		display: grid;
		grid-template-columns: 16px minmax(0, 1fr) auto;
		align-items: center;
		gap: 10px;
		height: 48px;
		padding: 0 12px;
		cursor: pointer;
	}
	.row + .row {
		border-top: 1px solid var(--border);
	}
	.row.on {
		background: var(--surface-alt);
	}
	.row.off {
		cursor: default;
		opacity: 0.55;
	}
	input {
		margin: 0;
		accent-color: var(--ink);
	}
	.text {
		display: grid;
		min-width: 0;
	}
	.title,
	.detail {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.title {
		color: var(--ink);
	}
	.detail,
	.mark {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
</style>
