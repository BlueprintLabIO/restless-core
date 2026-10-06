<script lang="ts">
	/* A figure that counts to its value when it first appears and when it changes, so a number
	 * reads as fresh rather than printed. The server renders the final value; reduced motion
	 * keeps it still. */
	import { onMount } from 'svelte';

	let {
		value,
		format = (n: number) => Math.round(n).toLocaleString(),
		duration = 900
	}: {
		value: number;
		format?: (n: number) => string;
		duration?: number;
	} = $props();

	let shown = $state<number | null>(null);
	let frame = 0;
	let mounted = false;

	function run(from: number, to: number) {
		cancelAnimationFrame(frame);
		if (matchMedia('(prefers-reduced-motion: reduce)').matches || from === to) {
			shown = to;
			return;
		}
		const start = performance.now();
		const tick = (now: number) => {
			const k = Math.min(1, (now - start) / duration);
			shown = from + (to - from) * (1 - Math.pow(1 - k, 3));
			if (k < 1) frame = requestAnimationFrame(tick);
		};
		frame = requestAnimationFrame(tick);
	}

	onMount(() => {
		mounted = true;
		run(0, value);
		return () => cancelAnimationFrame(frame);
	});
	$effect(() => {
		const target = value;
		if (mounted) run(shown ?? 0, target);
	});
</script>

<span class="count-up">{format(shown ?? value)}</span>

<style>
	.count-up {
		font-variant-numeric: tabular-nums;
	}
</style>
