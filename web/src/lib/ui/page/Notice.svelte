<script lang="ts">
	/* The one way a page says something is wrong, waiting or worth knowing.
	 *
	 * A short title the owner can read at a glance, an optional second line, and
	 * any raw system or agent text folded behind "Details". Never a wall of red
	 * prose. */
	import type { Snippet } from 'svelte';
	import CircleAlert from '@lucide/svelte/icons/circle-alert';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import Info from '@lucide/svelte/icons/info';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

	let {
		tone = 'info',
		title,
		details,
		role,
		actions,
		children
	}: {
		tone?: 'info' | 'warning' | 'danger' | 'success';
		title: string;
		/** Raw text for anyone who wants it: logs, agent summaries, error bodies. */
		details?: string | null;
		role?: 'alert' | 'status';
		actions?: Snippet;
		children?: Snippet;
	} = $props();
	const Icon = $derived(
		{ info: Info, warning: TriangleAlert, danger: CircleAlert, success: CircleCheck }[tone]
	);
</script>

<div class="notice {tone}" role={role ?? (tone === 'danger' ? 'alert' : 'status')}>
	<span class="notice-icon" aria-hidden="true"><Icon size={15} strokeWidth={1.9} /></span>
	<div class="notice-copy">
		<strong>{title}</strong>
		{#if children}<div class="notice-text">{@render children()}</div>{/if}
		{#if details}<details class="notice-details">
				<summary>Details</summary>
				<p>{details}</p>
			</details>{/if}
	</div>
	{#if actions}<div class="notice-actions">{@render actions()}</div>{/if}
</div>

<style>
	.notice {
		--notice-tone: var(--intent-conversation);
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: start;
		gap: 10px;
		padding: 10px 12px;
		border: 1px solid color-mix(in srgb, var(--notice-tone) 24%, var(--border));
		border-radius: var(--radius-lg);
		background: color-mix(in srgb, var(--notice-tone) 6%, var(--surface-raised));
		font-size: var(--t-body);
	}
	.warning {
		--notice-tone: var(--intent-authority);
	}
	.danger {
		--notice-tone: var(--state-danger);
	}
	.success {
		--notice-tone: var(--state-success);
	}
	.notice-icon {
		display: grid;
		place-items: center;
		height: 20px;
		color: var(--notice-tone);
	}
	.notice-copy {
		display: grid;
		gap: 2px;
		min-width: 0;
		line-height: 20px;
	}
	strong {
		color: var(--ink);
		font-weight: 600;
	}
	.notice-text {
		color: var(--text-secondary);
		overflow-wrap: anywhere;
	}
	.notice-details summary {
		width: fit-content;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		cursor: pointer;
		list-style: none;
	}
	.notice-details summary::-webkit-details-marker {
		display: none;
	}
	.notice-details summary::before {
		content: none !important;
	}
	.notice-details summary:hover {
		color: var(--ink);
	}
	.notice-details p {
		max-height: 220px;
		margin: 6px 0 0;
		overflow: auto;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		line-height: 1.5;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.notice-actions {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	@container page (max-width: 560px) {
		.notice {
			grid-template-columns: auto minmax(0, 1fr);
		}
		.notice-actions {
			grid-column: 2;
		}
	}
</style>
