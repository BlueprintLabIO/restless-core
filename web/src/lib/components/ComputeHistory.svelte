<script lang="ts">
	/* The account pool's last day or week: live memory against its limit as a line, the moments new
	 * work waited for memory as bands beneath it. The plot keeps its height whether or not there
	 * are points yet, so nothing below it moves when the first reading arrives. */
	import type { ComputePoint } from '$lib/model/company';
	import { Segmented } from '$lib/ui/page';
	import { formatClock, formatDay } from '$lib/ui/time';

	/* `undefined` while the first reading is on its way: the plot holds its box and says nothing yet,
	 * so its caption appears once rather than changing from a guess to the answer. */
	let {
		points,
		height = 96
	}: { points: ComputePoint[] | null | undefined; height?: number } = $props();
	let span = $state<'day' | 'week'>('day');
	let W = $state(600);

	const shown = $derived.by(() => {
		const since = Date.now() - (span === 'day' ? 1 : 7) * 86_400_000;
		return (points ?? []).filter((point) => Date.parse(point.at) >= since);
	});
	const from = $derived(Date.now() - (span === 'day' ? 1 : 7) * 86_400_000);
	const x = (at: string) => ((Date.parse(at) - from) / (Date.now() - from)) * W;
	const share = (point: ComputePoint) =>
		point.memory_limit_bytes > 0
			? Math.min(1, point.working_set_bytes / point.memory_limit_bytes)
			: 0;
	const y = (fraction: number) => height - 4 - fraction * (height - 12);
	const line = $derived(
		shown
			.map(
				(point, index) =>
					`${index ? 'L' : 'M'}${x(point.at).toFixed(1)} ${y(share(point)).toFixed(1)}`
			)
			.join(' ')
	);
	const step = $derived(
		span === 'day' ? (300_000 / 86_400_000) * W : (300_000 / (7 * 86_400_000)) * W
	);
	const peak = $derived(
		shown.reduce<ComputePoint | null>(
			(top, point) => (!top || share(point) > share(top) ? point : top),
			null
		)
	);
	/* History only exists from the first reading this host recorded (it began with the release that
	 * added it), so a young plot says when collecting began rather than looking empty or broken. It
	 * is drawn inside the plot's fixed box: nothing moves when more readings arrive. */
	const first = $derived(
		(points ?? []).reduce<number | null>((earliest, point) => {
			const at = Date.parse(point.at);
			return Number.isNaN(at) || (earliest != null && earliest <= at) ? earliest : at;
		}, null)
	);
	const since = $derived(first == null ? '' : `${formatClock(first)}, ${formatDay(first)}`);
	/* The span before the first reading, when it is a visible part of the plot. */
	const before = $derived(first != null && first > from ? ((first - from) / (Date.now() - from)) * W : 0);
	const waited = $derived(shown.filter((point) => point.waiting).length);
	const throttled = $derived(shown.reduce((sum, point) => sum + point.throttled, 0));
	const killed = $derived(shown.reduce((sum, point) => sum + point.oom_kills, 0));
	const cpuPeak = $derived(Math.max(0, ...shown.map((point) => point.cpu_cores ?? 0)));
	const gb = (value: number) =>
		`${new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(value / 1024 ** 3)} GB`;
	const duration = (count: number) => {
		const minutes = count * 5;
		return minutes < 60 ? `${minutes} min` : `${Math.round(minutes / 6) / 10} h`;
	};
</script>

<div class="history">
	<div class="head">
		<h3>History</h3>
		<Segmented
			label="History span"
			options={[
				{ value: 'day', label: '24 hours' },
				{ value: 'week', label: '7 days' }
			]}
			bind:value={span}
		/>
	</div>
	<div class="plot" bind:clientWidth={W}>
		<svg
			viewBox={`0 0 ${W} ${height}`}
			style:height={`${height}px`}
			role="img"
			aria-label="Account memory over time, against its limit"
		>
			{#if shown.length > 1 && before > 24}<rect
					class="uncollected"
					x="0"
					y="0"
					width={before}
					{height}
				/>{/if}
			<line class="limit" x1="0" x2={W} y1={y(1)} y2={y(1)} />
			{#each shown as point (point.at)}{#if point.waiting}<rect
						class="wait"
						x={x(point.at) - step}
						y="0"
						width={Math.max(2, step)}
						{height}
					/>{/if}{/each}
			{#if shown.length > 1}<path class="line" d={line} />{/if}
		</svg>
		{#if points === undefined}{:else if shown.length < 2}<span class="none"
				>{points === null
					? 'No history on this host'
					: since
						? `Collecting since ${since}`
						: 'Collecting a reading every five minutes'}</span
			>{:else if before > 24}<span class="since" style:max-width={`${Math.max(0, before - 8)}px`}
				>Collecting since {since}</span
			>{/if}
	</div>
	<dl class="facts">
		<div>
			<dt>Peak memory</dt>
			<dd>{peak ? `${gb(peak.working_set_bytes)} of ${gb(peak.memory_limit_bytes)}` : '—'}</dd>
		</div>
		<div>
			<dt>Peak CPU</dt>
			<dd>
				{shown.length > 1
					? `${new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 }).format(cpuPeak)} cores`
					: '—'}
			</dd>
		</div>
		<div>
			<dt>New work waited</dt>
			<dd>{shown.length ? (waited ? `about ${duration(waited)}` : 'Never') : '—'}</dd>
		</div>
		<div>
			<dt>Slowed at limit</dt>
			<dd>
				{shown.length ? (throttled ? `${throttled} times` : 'Never') : '—'}{killed
					? ` · ${killed} stopped for memory`
					: ''}
			</dd>
		</div>
	</dl>
</div>

<style>
	.history {
		padding: 16px;
		border-top: 1px solid var(--border);
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		height: 28px;
		margin-bottom: 12px;
	}
	h3 {
		margin: 0;
		font-size: var(--t-body);
		font-weight: 500;
		color: var(--text-secondary);
	}
	.plot {
		position: relative;
	}
	svg {
		display: block;
		width: 100%;
	}
	.limit {
		stroke: var(--border-strong);
		stroke-dasharray: 3 3;
	}
	.wait {
		fill: var(--intent-authority);
		opacity: 0.14;
	}
	.line {
		fill: none;
		stroke: var(--accent);
		stroke-width: 1.75;
		stroke-linejoin: round;
	}
	.uncollected {
		fill: var(--surface-alt);
		opacity: 0.6;
	}
	.since {
		position: absolute;
		top: 4px;
		left: 8px;
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: nowrap;
		text-overflow: ellipsis;
		pointer-events: none;
	}
	.none {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
	.facts {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: 12px;
		margin: 12px 0 0;
	}
	.facts div {
		min-width: 0;
	}
	dt {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	dd {
		margin: 2px 0 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-variant-numeric: tabular-nums;
	}
	@container page (max-width: 720px) {
		.facts {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}
</style>
