<script lang="ts">
	/* A short burst of square pixels, in the product's own intent colours. Decorative only. */
	let { seed = 0 }: { seed?: number } = $props();

	const COLOURS = [
		'var(--intent-direction)',
		'var(--intent-feedback)',
		'var(--intent-conversation)',
		'var(--intent-authority)'
	];

	/* Deterministic, so the burst looks the same on every render of the same seed. */
	function rand(index: number, salt: number) {
		const value = Math.sin((index + 1) * 12.9898 + (seed + 1) * 78.233 + salt) * 43758.5453;
		return value - Math.floor(value);
	}

	const pieces = $derived(
		Array.from({ length: 32 }, (_, index) => {
			const angle = (index / 32) * Math.PI * 2 + rand(index, 1) * 0.4;
			const distance = 70 + rand(index, 2) * 130;
			return {
				x: Math.round(Math.cos(angle) * distance),
				y: Math.round(Math.sin(angle) * distance - 30),
				size: 4 + Math.round(rand(index, 3) * 3) * 2,
				colour: COLOURS[index % COLOURS.length],
				delay: Math.round(rand(index, 4) * 120)
			};
		})
	);
</script>

<span class="burst" aria-hidden="true">
	{#each pieces as piece, index (index)}
		<i
			style:--x="{piece.x}px"
			style:--y="{piece.y}px"
			style:--s="{piece.size}px"
			style:--c={piece.colour}
			style:--d="{piece.delay}ms"
		></i>
	{/each}
</span>

<style>
	.burst {
		position: absolute;
		left: 50%;
		top: 50%;
		width: 0;
		height: 0;
		pointer-events: none;
	}
	i {
		position: absolute;
		left: 0;
		top: 0;
		width: var(--s);
		height: var(--s);
		background: var(--c);
		opacity: 0;
		animation: burst 900ms steps(9, end) var(--d) forwards;
	}
	@keyframes burst {
		0% {
			opacity: 1;
			transform: translate(0, 0);
		}
		100% {
			opacity: 0;
			transform: translate(var(--x), calc(var(--y) + 60px));
		}
	}
	@media (prefers-reduced-motion: reduce) {
		i {
			display: none;
		}
	}
</style>
