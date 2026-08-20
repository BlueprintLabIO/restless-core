<script lang="ts">
	/* Press-and-hold approval button. Pointer-driven rAF fill over `duration`;
	 * releasing early resets, reaching 100% fires `onapprove` once. Renders a
	 * plain submit button so it still works in a form without JS. */

	let {
		label,
		small = false,
		duration = 900,
		disabled = false,
		title = 'Hold to approve',
		completeLabel = 'approved ✓',
		onapprove
	}: {
		label: string;
		small?: boolean;
		duration?: number;
		disabled?: boolean;
		title?: string;
		completeLabel?: string;
		onapprove?: () => void;
	} = $props();

	let pct = $state(0);
	let done = $state(false);
	let raf: number | null = null;
	let button: HTMLButtonElement;

	function start() {
		if (done || disabled || raf !== null) return;
		const t0 = performance.now();
		const tick = (now: number) => {
			pct = Math.min(100, ((now - t0) / duration) * 100);
			if (pct >= 100) {
				raf = null;
				done = true;
				if (onapprove) {
					onapprove();
				} else {
					button.closest('form')?.requestSubmit();
				}
				return;
			}
			raf = requestAnimationFrame(tick);
		};
		raf = requestAnimationFrame(tick);
	}

	function stop() {
		if (raf !== null) {
			cancelAnimationFrame(raf);
			raf = null;
		}
		if (!done) pct = 0;
	}

	function guardClick(event: MouseEvent) {
		/* With JS on, a plain click must not bypass the hold. */
		if (!done) event.preventDefault();
	}
</script>

<button
	bind:this={button}
	type="submit"
	class="hold-approve"
	class:small
	class:done
	{disabled}
	style="--pct: {pct}%"
	onpointerdown={start}
	onpointerup={stop}
	onpointerleave={stop}
	onclick={guardClick}
	{title}
>
	{#if done}
		{completeLabel}
	{:else if pct > 2}
		hold… {Math.round(pct)}%
	{:else}
		{label}
	{/if}
</button>
