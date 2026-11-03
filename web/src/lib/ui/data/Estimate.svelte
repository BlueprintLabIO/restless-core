<script lang="ts">
	/* A rough estimate of how far something has come: a thin bar and one line of words, such as
	 * "~60% · next: export print files". It draws its final state at once and holds exactly one
	 * line, so a row or card never grows, shifts or animates when an estimate lands or changes. */
	let {
		fill,
		text,
		title,
		tone = 'progress',
		wide = false,
		label = 'Estimated progress'
	}: {
		/** 0–100: how far the bar fills. */
		fill: number;
		/** The words beside the bar. Empty shows the bar alone. */
		text: string;
		/** Who estimated it and when, shown on hover. */
		title?: string;
		/** `blocked` keeps the last number on the bar, quietened; `stopped` greys it. */
		tone?: 'progress' | 'blocked' | 'done' | 'stopped';
		/** A longer bar, for a detail pane. */
		wide?: boolean;
		label?: string;
	} = $props();

	const value = $derived(Math.max(0, Math.min(100, Number.isFinite(fill) ? Math.round(fill) : 0)));
</script>

<span class="estimate {tone}" class:wide {title}>
	<span
		class="bar"
		role="meter"
		aria-label={label}
		aria-valuemin="0"
		aria-valuemax="100"
		aria-valuenow={value}
		aria-valuetext={text || `${value}%`}><span class="fill" style:width={`${value}%`}></span></span
	>{#if text}<span class="words">{text}</span>{/if}
</span>

<style>
	.estimate {
		--estimate-fill: var(--intent-conversation);
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		max-width: 100%;
		height: 1.4em;
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.4;
		font-variant-numeric: tabular-nums;
		vertical-align: middle;
	}
	.done {
		--estimate-fill: var(--state-success);
	}
	.blocked {
		--estimate-fill: color-mix(in srgb, var(--intent-authority) 70%, var(--text-tertiary));
	}
	.stopped {
		--estimate-fill: var(--text-tertiary);
		color: var(--text-tertiary);
	}
	.bar {
		flex: none;
		position: relative;
		width: 40px;
		height: 4px;
		overflow: hidden;
		border-radius: 2px;
		background: color-mix(in srgb, var(--ink) 11%, transparent);
	}
	.wide .bar {
		width: 120px;
		height: 6px;
		border-radius: 3px;
	}
	.fill {
		position: absolute;
		inset: 0 auto 0 0;
		border-radius: inherit;
		background: var(--estimate-fill);
	}
	.words {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	@media (forced-colors: active) {
		.bar {
			border: 1px solid CanvasText;
		}
		.fill {
			background: CanvasText;
		}
	}
</style>
