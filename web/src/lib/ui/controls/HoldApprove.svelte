<script lang="ts">
	import { onDestroy } from 'svelte';
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
			if (disabled) {
				stop();
				return;
			}
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

	onDestroy(stop);

	function guardClick(event: MouseEvent) {
		/* Completion already invokes the callback or submits the form once. */
		event.preventDefault();
	}
</script>

<button
	bind:this={button}
	type="submit"
	class="hold-approve"
	class:small
	class:done
	{disabled}
	aria-label={done ? completeLabel : label}
	style="--pct: {pct}%"
	onkeydown={(event) => {
		if (event.key === ' ' || event.key === 'Enter') {
			event.preventDefault();
			if (!event.repeat) start();
		}
	}}
	onkeyup={(event) => {
		if (event.key === ' ' || event.key === 'Enter') {
			event.preventDefault();
			stop();
		}
	}}
	onblur={stop}
	onpointerdown={(event) => {
		if (event.isPrimary && event.button === 0) start();
	}}
	onpointerup={stop}
	onpointerleave={stop}
	onpointercancel={stop}
	onclick={guardClick}
	{title}
>
	<!-- Keep every label in the layout so progress cannot shrink the hit area
	     beneath the pointer and trigger pointerleave, cancelling the hold. -->
	<span aria-hidden="true" class:concealed={done || pct > 2}>{label}</span>
	<span aria-hidden="true" class:concealed={done || pct <= 2}>hold… {Math.round(pct)}%</span>
	<span aria-hidden="true" class:concealed={!done}>{completeLabel}</span>
</button>

<style>
	.hold-approve {
		display: inline-grid;
		place-items: center;
		touch-action: none;
		user-select: none;
	}
	span {
		grid-area: 1 / 1;
	}
	.concealed {
		visibility: hidden;
	}
</style>
