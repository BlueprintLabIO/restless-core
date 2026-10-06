<script lang="ts" module>
	export type UsageDay = { date: string; value: number };
</script>

<script lang="ts">
	/* Use over days as an area: the line draws itself in, the area fades up beneath it, and a
	 * pointer or keyboard focus reads any day exactly. Missing days are zero, not gaps, so the
	 * shape is honest about quiet days. */
	import { onMount } from 'svelte';

	let {
		days,
		span = 30,
		end,
		format = (value: number) => value.toLocaleString(),
		label,
		height = 148
	}: {
		/** ISO dates (YYYY-MM-DD) with their value; any order, any gaps. */
		days: UsageDay[];
		/** How many days to show, ending at `end`. */
		span?: number;
		/** The last day shown, YYYY-MM-DD. Defaults to today (UTC). */
		end?: string;
		format?: (value: number) => string;
		/** What the chart shows, for assistive technology. */
		label: string;
		height?: number;
	} = $props();

	const DAY = 86_400_000;
	/* Drawn at its real width, never stretched: strokes, dashes and the draw-in stay true. */
	let W = $state(600);
	const series = $derived.by(() => {
		const last = Date.parse(`${end ?? new Date().toISOString().slice(0, 10)}T00:00:00Z`);
		const byDate = new Map(days.map((day) => [day.date, day.value]));
		return Array.from({ length: span }, (_, index) => {
			const date = new Date(last - (span - 1 - index) * DAY).toISOString().slice(0, 10);
			return { date, value: byDate.get(date) ?? 0 };
		});
	});
	/* Cumulative: a month's spend reads as a rising line against its allowance. */
	const cumulative = $derived(
		series.reduce<number[]>((sums, day, index) => [...sums, (sums[index - 1] ?? 0) + day.value], [])
	);
	const top = $derived(Math.max(...cumulative, 0) || 1);
	const x = (index: number) => (index / Math.max(1, span - 1)) * W;
	const y = (value: number) => height - 6 - (value / top) * (height - 18);
	const line = $derived(
		cumulative
			.map((value, index) => `${index ? 'L' : 'M'}${x(index).toFixed(1)} ${y(value).toFixed(1)}`)
			.join(' ')
	);
	const area = $derived(`${line} L${W} ${height} L0 ${height} Z`);
	const quiet = $derived(cumulative.at(-1) === 0);
	const short = (date: string) =>
		new Date(`${date}T00:00:00Z`).toLocaleDateString(undefined, {
			month: 'short',
			day: '2-digit',
			timeZone: 'UTC'
		});

	let active = $state<number | null>(null);
	let svg = $state<SVGSVGElement>();
	function point(event: PointerEvent) {
		if (!svg) return;
		const box = svg.getBoundingClientRect();
		const ratio = Math.max(0, Math.min(1, (event.clientX - box.left) / box.width));
		active = Math.round(ratio * (span - 1));
	}
	function key(event: KeyboardEvent) {
		if (event.key === 'ArrowLeft') active = Math.max(0, (active ?? span) - 1);
		else if (event.key === 'ArrowRight') active = Math.min(span - 1, (active ?? -1) + 1);
		else return;
		event.preventDefault();
	}
	let drawn = $state(false);
	onMount(() => {
		const frame = requestAnimationFrame(() => (drawn = true));
		return () => cancelAnimationFrame(frame);
	});
	const id = `usage-${Math.random().toString(36).slice(2, 8)}`;
</script>

<figure class="usage-chart" class:drawn class:quiet>
	<!-- Scrubbing the chart is choosing a day: a slider, read aloud as that day's use. -->
	<div
		class="plot"
		bind:clientWidth={W}
		role="slider"
		tabindex="0"
		aria-label={`${label}: ${format(cumulative.at(-1) ?? 0)} over ${span} days`}
		aria-valuemin="0"
		aria-valuemax={span - 1}
		aria-valuenow={active ?? span - 1}
		aria-valuetext={`${short(series[active ?? span - 1].date)}: ${format(series[active ?? span - 1].value)}`}
		onpointermove={point}
		onpointerleave={() => (active = null)}
		onkeydown={key}
		onblur={() => (active = null)}
	>
		<svg
			bind:this={svg}
			viewBox={`0 0 ${W} ${height}`}
			style:height={`${height}px`}
			aria-hidden="true"
		>
			<defs>
				<linearGradient id={`${id}-fade`} x1="0" x2="0" y1="0" y2="1">
					<stop offset="0" stop-color="var(--chart-line)" stop-opacity="0.22" />
					<stop offset="1" stop-color="var(--chart-line)" stop-opacity="0" />
				</linearGradient>
				<pattern id={`${id}-dots`} width="12" height="12" patternUnits="userSpaceOnUse">
					<circle cx="1" cy="1" r="1" class="grid-dot" />
				</pattern>
			</defs>
			<rect width={W} {height} fill={`url(#${id}-dots)`} />
			<path class="area" d={area} fill={`url(#${id}-fade)`} />
			<path class="line" d={line} pathLength="1" />
			{#if active !== null}
				<line class="cursor" x1={x(active)} x2={x(active)} y1="0" y2={height} />
			{/if}
		</svg>
	</div>
	{#if active !== null}
		{@const day = series[active]}
		<div
			class="readout"
			style:left={`${(active / Math.max(1, span - 1)) * 100}%`}
			style:top={`${y(cumulative[active])}px`}
		>
			<i aria-hidden="true"></i>
			<span><b>{format(day.value)}</b> {short(day.date)}</span>
		</div>
	{:else}
		<i class="now" style:top={`${y(cumulative.at(-1) ?? 0)}px`} aria-hidden="true"></i>
	{/if}
	<figcaption>
		<span>{short(series[0].date)}</span>
		{#if quiet}<span class="quiet-note">No use yet</span>{:else}<span
				>{short(series[Math.floor(span / 2)].date)}</span
			>{/if}
		<span>{short(series[span - 1].date)}</span>
	</figcaption>
</figure>

<style>
	.usage-chart {
		--chart-line: var(--intent-conversation);
		position: relative;
		margin: 0;
	}
	.plot {
		border-radius: var(--radius-sm);
		outline: none;
		cursor: crosshair;
		touch-action: pan-y;
	}
	.plot:focus-visible {
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--intent-conversation) 40%, transparent);
	}
	svg {
		display: block;
		width: 100%;
		overflow: visible;
	}
	.grid-dot {
		fill: var(--text-tertiary);
		opacity: 0.2;
	}
	.line {
		fill: none;
		stroke: var(--chart-line);
		stroke-width: 2;
		stroke-linecap: round;
		stroke-linejoin: round;
		stroke-dasharray: 1;
		stroke-dashoffset: 1;
	}
	.area {
		opacity: 0;
	}
	.drawn .line {
		stroke-dashoffset: 0;
		transition: stroke-dashoffset 1.4s var(--ease-out) 0.15s;
	}
	.drawn .area {
		opacity: 1;
		transition: opacity 0.8s var(--ease-standard) 0.8s;
	}
	.quiet .line {
		stroke: var(--text-tertiary);
		stroke-dasharray: none;
		stroke-dashoffset: 0;
	}
	.cursor {
		stroke: var(--text-tertiary);
		stroke-width: 1;
		stroke-dasharray: 3 3;
	}
	.now,
	.readout i {
		position: absolute;
		width: 9px;
		height: 9px;
		border: 2px solid var(--chart-line);
		border-radius: 50%;
		background: var(--surface-pane);
		transform: translate(-50%, -50%);
	}
	.now {
		right: -4px;
		opacity: 0;
	}
	.drawn .now {
		opacity: 1;
		transition: opacity 0.3s var(--ease-standard) 1.5s;
	}
	.readout {
		position: absolute;
		pointer-events: none;
	}
	.readout span {
		position: absolute;
		bottom: 10px;
		left: 0;
		padding: 3px 7px;
		border-radius: var(--radius-sm);
		background: var(--ink);
		color: var(--bg-app);
		font-size: var(--t-label);
		white-space: nowrap;
		transform: translateX(-50%);
		animation: readout-in var(--motion-state) var(--ease-out);
	}
	.readout b {
		font-family: var(--font-mono);
		font-weight: 500;
	}
	@keyframes readout-in {
		from {
			opacity: 0;
			transform: translate(-50%, 3px);
		}
	}
	figcaption {
		display: flex;
		justify-content: space-between;
		margin-top: 6px;
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-mono);
	}
	@media (prefers-reduced-motion: reduce) {
		.drawn .line,
		.drawn .area,
		.drawn .now {
			transition: none;
		}
		.readout span {
			animation: none;
		}
	}
</style>
