<script lang="ts">
	/* The dot-matrix the wordmark is drawn in, as a slow field behind a greeting: texture that
	 * belongs to the brand rather than decoration from elsewhere. It only animates while visible,
	 * holds one still frame for reduced motion, and never takes a pointer. */
	import { onMount } from 'svelte';

	let { step = 9, opacity = 0.55 }: { step?: number; opacity?: number } = $props();
	let canvas = $state<HTMLCanvasElement>();

	onMount(() => {
		const element = canvas;
		const context = element?.getContext('2d');
		if (!element || !context) return;
		const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
		let frame = 0;
		let visible = true;
		const draw = (time: number) => {
			const ratio = Math.min(2, devicePixelRatio || 1);
			const width = element.clientWidth;
			const height = element.clientHeight;
			if (element.width !== Math.round(width * ratio)) {
				element.width = Math.round(width * ratio);
				element.height = Math.round(height * ratio);
			}
			context.setTransform(ratio, 0, 0, ratio, 0, 0);
			context.clearRect(0, 0, width, height);
			context.fillStyle = getComputedStyle(element).color;
			for (let y = 0; y < height; y += step)
				for (let x = 0; x < width; x += step) {
					const v =
						Math.sin(x * 0.021 + time * 0.0006) +
						Math.cos(y * 0.05 - time * 0.0004) +
						Math.sin((x + y) * 0.013);
					const strength = Math.max(0, (v + 0.6) * 0.55);
					if (strength > 0.15) {
						context.globalAlpha = Math.min(0.5, strength * 0.35);
						context.fillRect(x, y, 2, 2);
					}
				}
			if (!still && visible) frame = requestAnimationFrame(draw);
		};
		const observer = new IntersectionObserver(([entry]) => {
			const was = visible;
			visible = entry.isIntersecting && document.visibilityState === 'visible';
			if (visible && !was && !still) frame = requestAnimationFrame(draw);
		});
		observer.observe(element);
		frame = requestAnimationFrame(draw);
		return () => {
			cancelAnimationFrame(frame);
			observer.disconnect();
		};
	});
</script>

<canvas class="matrix-field" bind:this={canvas} style:opacity aria-hidden="true"></canvas>

<style>
	.matrix-field {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		color: var(--accent);
		pointer-events: none;
		mask-image: linear-gradient(90deg, transparent 30%, black 78%);
	}
</style>
