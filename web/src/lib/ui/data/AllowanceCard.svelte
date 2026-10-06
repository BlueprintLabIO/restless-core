<script lang="ts">
	/* What is left of a monthly allowance, small enough for a rail's foot: a ring that fills,
	 * the amount, when it renews, and one action. */
	import type { Snippet } from 'svelte';
	import Meter from './Meter.svelte';
	import CountUp from './CountUp.svelte';

	let {
		available,
		total,
		unit = 'AI credit',
		renews,
		format = (n: number) => n.toLocaleString('en-US', { style: 'currency', currency: 'USD' }),
		href,
		action
	}: {
		available: number;
		total: number;
		unit?: string;
		/** Display date it renews, such as "Nov 1". */
		renews?: string;
		format?: (n: number) => string;
		/** Where the card leads, such as the plan page. */
		href?: string;
		action?: Snippet;
	} = $props();
	const left = $derived(total > 0 ? Math.max(0, available) / total : 0);
</script>

<div class="allowance">
	<a class="allowance-top" {href} title={`${format(available)} of ${format(total)} ${unit} left`}>
		<Meter
			shape="ring"
			size={28}
			fraction={left}
			tone={available <= 0 ? 'danger' : left < 0.2 ? 'warning' : 'conversation'}
			label={`${unit} left`}
		/>
		<span>
			<strong><CountUp value={available} {format} /> left</strong>
			<small>of {format(total)} {unit}{renews ? ` · renews ${renews}` : ''}</small>
		</span>
	</a>
	{#if action}<div class="allowance-action">{@render action()}</div>{/if}
</div>

<style>
	.allowance {
		display: grid;
		gap: 10px;
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
	}
	.allowance-top {
		display: flex;
		align-items: center;
		gap: 10px;
		color: inherit;
		text-decoration: none;
	}
	.allowance strong {
		display: block;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 600;
	}
	.allowance small {
		display: block;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		line-height: 1.35;
	}
	/* The one action, whatever the host renders it as, is a full-width quiet button. */
	.allowance-action > :global(:is(a, button, form)) {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		min-height: 30px;
		height: 30px;
		padding: 0 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: none;
		color: var(--ink);
		font: 500 var(--t-body) var(--font-ui);
		text-decoration: none;
		cursor: pointer;
		transition:
			transform var(--motion-press) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard);
	}
	.allowance-action > :global(form) {
		padding: 0;
		border: 0;
	}
	.allowance-action > :global(form button) {
		width: 100%;
	}
	.allowance-action > :global(:is(a, button):hover) {
		box-shadow: 0 1px 3px rgba(20, 30, 50, 0.1);
	}
	.allowance-action > :global(:is(a, button):active) {
		transform: scale(0.97);
	}
</style>
