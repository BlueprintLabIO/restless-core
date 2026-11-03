<script lang="ts">
	/* A tiny trend line of percentages, oldest first, ending in a dot at the latest. It keeps its
	 * size with one point or none, so the line beside it never moves when history arrives. */
	let {
		values,
		width = 56,
		height = 16,
		label
	}: {
		/** 0–100 each, oldest first. */
		values: readonly number[];
		width?: number;
		height?: number;
		label: string;
	} = $props();

	const points = $derived.by(() => {
		const shown = values.filter((value) => Number.isFinite(value)).slice(-12);
		if (shown.length < 2) return [];
		const step = (width - 4) / (shown.length - 1);
		return shown.map((value, index) => {
			const clamped = Math.max(0, Math.min(100, value));
			return [2 + step * index, height - 2 - (clamped / 100) * (height - 4)] as const;
		});
	});
</script>

<svg
	class="trend"
	{width}
	{height}
	viewBox={`0 0 ${width} ${height}`}
	role="img"
	aria-label={label}
	aria-hidden={points.length ? undefined : 'true'}
>
	{#if points.length}
		<polyline points={points.map(([x, y]) => `${x},${y}`).join(' ')} />
		<circle cx={points.at(-1)?.[0]} cy={points.at(-1)?.[1]} r="2" />
	{/if}
</svg>

<style>
	.trend {
		flex: none;
		display: block;
		overflow: visible;
		color: var(--intent-conversation);
	}
	polyline {
		fill: none;
		stroke: currentColor;
		stroke-width: 1.5;
		stroke-linecap: round;
		stroke-linejoin: round;
		opacity: 0.7;
	}
	circle {
		fill: currentColor;
	}
</style>
