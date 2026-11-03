<script lang="ts">
	import { onDestroy } from 'svelte';
	import Check from '@lucide/svelte/icons/check';
	/* Press-and-hold approval button. Pointer-driven rAF fill over `duration`;
	 * releasing early resets, reaching 100% fires `onapprove` once. Renders a
	 * plain submit button so it still works in a form without JS.
	 *
	 * Holding is not possible for everyone, so a short press arms the button for a few
	 * seconds and a second press confirms. Assistive tech that activates the button with
	 * a click (no press-and-hold) takes the same two-step route.
	 *
	 * Completing it is a small celebration (3 November, the owner: "make it pop"): a check pops
	 * in and an accent glow rings the button, then the decision is sent and the card resolves
	 * away. Only transform, opacity and box-shadow move, so nothing around it shifts; with
	 * reduced motion the check simply fades in. The decision waits for the pop's peak (a few
	 * hundred milliseconds), never longer, and is sent at once if the button goes away first. */

	let {
		label,
		small = false,
		duration = 900,
		disabled = false,
		title = 'Hold to approve',
		completeLabel = 'approved ✓',
		armedLabel = 'press again to approve',
		onapprove
	}: {
		label: string;
		small?: boolean;
		duration?: number;
		disabled?: boolean;
		title?: string;
		completeLabel?: string;
		armedLabel?: string;
		onapprove?: () => void;
	} = $props();

	let pct = $state(0);
	let done = $state(false);
	let raf: number | null = null;
	let button: HTMLButtonElement;
	let armed = $state(false);
	let armTimer: ReturnType<typeof setTimeout> | undefined;
	let lastPress = -Infinity; /* when a press last started or ended */
	let celebration: ReturnType<typeof setTimeout> | undefined;
	let pending = false; /* approved, decision not yet handed on */
	/* Long enough to see the pop land, short enough that the answer still feels instant. */
	const CELEBRATE_MS = 280;
	const FADE_MS = 90;

	function reducedMotion(): boolean {
		return (
			typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)').matches
		);
	}

	function handOn() {
		clearTimeout(celebration);
		celebration = undefined;
		if (!pending) return;
		pending = false;
		if (onapprove) {
			onapprove();
		} else {
			button?.closest('form')?.requestSubmit();
		}
	}

	function arm() {
		armed = true;
		clearTimeout(armTimer);
		armTimer = setTimeout(() => (armed = false), 4000);
	}

	function approve() {
		raf = null;
		armed = false;
		clearTimeout(armTimer);
		done = true;
		pending = true;
		celebration = setTimeout(handOn, reducedMotion() ? FADE_MS : CELEBRATE_MS);
	}

	function start() {
		if (done || disabled || raf !== null) return;
		lastPress = performance.now();
		const t0 = performance.now();
		const tick = (now: number) => {
			if (disabled) {
				stop();
				return;
			}
			pct = Math.min(100, ((now - t0) / duration) * 100);
			if (pct >= 100) {
				approve();
				return;
			}
			raf = requestAnimationFrame(tick);
		};
		raf = requestAnimationFrame(tick);
	}

	function stop() {
		lastPress = performance.now();
		if (raf !== null) {
			cancelAnimationFrame(raf);
			raf = null;
		}
		if (!done) pct = 0;
	}

	/* A press that ended before the fill completed: arm, or confirm if already armed. */
	function release() {
		const short = raf !== null && pct < 100;
		stop();
		if (!short || done || disabled) return;
		if (armed) approve();
		else arm();
	}

	onDestroy(() => {
		stop();
		clearTimeout(armTimer);
		/* Never lose an approval to its celebration. */
		handOn();
	});

	function guardClick(event: MouseEvent) {
		/* Completion already invokes the callback or submits the form once. */
		event.preventDefault();
		/* A click with no press before it came from assistive tech or a synthetic activation. */
		if (performance.now() - lastPress > 400 && !done && !disabled) {
			if (armed) approve();
			else arm();
		}
	}
</script>

<button
	bind:this={button}
	type="submit"
	class="hold-approve"
	class:small
	class:done
	{disabled}
	aria-label={done ? completeLabel : armed ? armedLabel : label}
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
			release();
		}
	}}
	onblur={stop}
	onpointerdown={(event) => {
		if (event.isPrimary && event.button === 0) start();
	}}
	onpointerup={release}
	onpointerleave={stop}
	onpointercancel={stop}
	onclick={guardClick}
	{title}
>
	<!-- Keep every label in the layout so progress cannot shrink the hit area
	     beneath the pointer and trigger pointerleave, cancelling the hold. -->
	<span aria-hidden="true" class:concealed={done || pct > 2 || armed}>{label}</span>
	<span aria-hidden="true" class:concealed={done || pct > 2 || !armed}>{armedLabel}</span>
	<span aria-hidden="true" class:concealed={done || pct <= 2}>hold… {Math.round(pct)}%</span>
	<span aria-hidden="true" class="complete" class:concealed={!done}
		><span class="check"><Check size={14} strokeWidth={2.5} /></span>{completeLabel}</span
	>
</button>
<!-- The label swap above is silent to a screen reader; this says the hold completed. -->
<span class="sr-only" role="status" aria-live="polite"
	>{done ? completeLabel : armed ? armedLabel : ''}</span
>

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
	.complete {
		display: inline-flex;
		align-items: center;
		gap: 5px;
	}
	.check {
		display: inline-flex;
		opacity: 0;
	}
	/* The check pops in; the button's glow and settle are in style/primitives.css. */
	.done .check {
		animation: hold-approve-check 420ms var(--ease-spring, cubic-bezier(0.16, 1, 0.3, 1)) 40ms both;
	}
	@keyframes hold-approve-check {
		0% {
			opacity: 0;
			transform: scale(0.3) rotate(-12deg);
		}
		60% {
			opacity: 1;
			transform: scale(1.25) rotate(0deg);
		}
		100% {
			opacity: 1;
			transform: scale(1);
		}
	}
	@keyframes hold-approve-fade {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.done .check,
		.done .complete {
			animation: hold-approve-fade 90ms linear both;
		}
	}
</style>
