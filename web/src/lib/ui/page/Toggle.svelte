<script lang="ts">
	/* An on/off switch with an accessible name, for settings that apply at once. */
	let {
		checked,
		label,
		title,
		disabled = false,
		onchange
	}: {
		checked: boolean;
		label: string;
		title?: string;
		disabled?: boolean;
		onchange: (next: boolean) => void;
	} = $props();
</script>

<button
	class="toggle"
	type="button"
	role="switch"
	aria-checked={checked}
	aria-label={label}
	{title}
	{disabled}
	onclick={() => onchange(!checked)}
>
	<span class="track" aria-hidden="true"><i></i></span>
</button>

<style>
	.toggle {
		display: inline-grid;
		place-items: center;
		min-width: 36px;
		min-height: 28px;
		padding: 0;
		border: 0;
		background: transparent;
		cursor: pointer;
	}
	.toggle:disabled {
		cursor: not-allowed;
		opacity: 0.5;
	}
	.track {
		position: relative;
		width: 30px;
		height: 18px;
		border-radius: 999px;
		background: var(--border-strong);
		box-shadow: inset 0 1px 2px rgba(20, 30, 50, 0.12);
		transition: background-color var(--motion-state) var(--ease-standard);
	}
	.track i {
		position: absolute;
		top: 2px;
		left: 2px;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--surface-raised);
		box-shadow: 0 1px 2px rgba(20, 30, 50, 0.25);
		transition: transform var(--motion-state) var(--ease-spring);
	}
	[aria-checked='true'] .track {
		background: var(--state-success);
	}
	[aria-checked='true'] .track i {
		transform: translateX(12px);
	}
	@media (pointer: coarse) {
		.toggle {
			min-width: 44px;
			min-height: 44px;
		}
	}
</style>
