<script lang="ts">
	/* How much of something is used, as a bar or a ring. It fills from empty when it appears. */
	import { onMount } from 'svelte';

	let {
		fraction,
		shape = 'bar',
		size = 28,
		tone = 'conversation',
		label
	}: {
		/** 0–1; clamped. */
		fraction: number;
		shape?: 'bar' | 'ring';
		/** Ring diameter in pixels. */
		size?: number;
		tone?: 'conversation' | 'warning' | 'danger';
		label: string;
	} = $props();

	const value = $derived(Math.max(0, Math.min(1, Number.isFinite(fraction) ? fraction : 0)));
	let filled = $state(false);
	onMount(() => {
		const frame = requestAnimationFrame(() => (filled = true));
		return () => cancelAnimationFrame(frame);
	});
	const shown = $derived(filled ? value : 0);
	const radius = $derived(size / 2 - 2.5);
	const circumference = $derived(2 * Math.PI * radius);
</script>

{#if shape === 'ring'}
	<svg
		class="meter-ring {tone}"
		width={size}
		height={size}
		viewBox={`0 0 ${size} ${size}`}
		role="meter"
		aria-label={label}
		aria-valuemin="0"
		aria-valuemax="100"
		aria-valuenow={Math.round(value * 100)}
	>
		<circle class="track" cx={size / 2} cy={size / 2} r={radius} />
		<circle
			class="fill"
			cx={size / 2}
			cy={size / 2}
			r={radius}
			stroke-dasharray={circumference}
			stroke-dashoffset={circumference * (1 - shown)}
		/>
	</svg>
{:else}
	<div
		class="meter-bar {tone}"
		role="meter"
		aria-label={label}
		aria-valuemin="0"
		aria-valuemax="100"
		aria-valuenow={Math.round(value * 100)}
	>
		<span style:width={`${shown * 100}%`}></span>
	</div>
{/if}

<style>
	.meter-ring,
	.meter-bar {
		--meter: var(--intent-conversation);
		flex: none;
	}
	.warning {
		--meter: var(--intent-authority);
	}
	.danger {
		--meter: var(--state-danger);
	}
	.meter-ring circle {
		fill: none;
		stroke-width: 3;
	}
	.meter-ring .track {
		stroke: var(--accent-soft);
	}
	.meter-ring .fill {
		stroke: var(--meter);
		stroke-linecap: round;
		transform: rotate(-90deg);
		transform-origin: center;
		transition: stroke-dashoffset 1.1s var(--ease-out) 0.2s;
	}
	.meter-bar {
		height: 6px;
		overflow: hidden;
		border-radius: 3px;
		background: var(--accent-soft);
	}
	.meter-bar span {
		display: block;
		height: 100%;
		border-radius: 3px;
		background: linear-gradient(
			90deg,
			var(--meter),
			color-mix(in srgb, var(--meter) 72%, var(--intent-direction))
		);
		transition: width 1.1s var(--ease-out) 0.2s;
	}
	@media (prefers-reduced-motion: reduce) {
		.meter-ring .fill,
		.meter-bar span {
			transition: none;
		}
	}
</style>
