<script lang="ts">
	/* An on/off switch with an accessible name, for settings that apply at once. It moves the moment
	 * it is pressed and stays there while the change is saved (the page disables it meanwhile); if
	 * the saved value comes back unchanged, the change failed and it returns. */
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

	let wanted = $state<boolean | null>(null);
	let saving = false;
	const shown = $derived(wanted ?? checked);
	$effect(() => {
		// The saved value arrived: it is the truth from here, whether it matches or not.
		if (wanted !== null && checked === wanted) wanted = null;
	});
	$effect(() => {
		if (disabled) saving = true;
		else if (saving) {
			saving = false;
			if (wanted !== null && checked !== wanted) wanted = null;
		}
	});
</script>

<button
	class="toggle"
	type="button"
	role="switch"
	aria-checked={shown}
	aria-label={label}
	{title}
	{disabled}
	onclick={() => {
		wanted = !shown;
		onchange(wanted);
	}}
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
