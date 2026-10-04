<script lang="ts">
	/* The one presentation of a failed read or write. The copy comes from
	 * describeFailure, so a surface names only what failed ("Attention") and how
	 * to try again; it never renders transport text itself.
	 *
	 * inline — one line inside a pane, beside the content it concerns.
	 * block  — replaces a pane's content when nothing is cached.
	 * page   — replaces the whole surface (access, not found).
	 *
	 * `stale` means the surface is still showing its last good data: the notice
	 * then says so instead of implying the content vanished. While the app-level
	 * banner reports a lost connection, a stale surface shows nothing here: one
	 * voice speaks for the outage instead of every pane repeating it. */
	import CircleAlert from '@lucide/svelte/icons/circle-alert';
	import CloudOff from '@lucide/svelte/icons/cloud-off';
	import Lock from '@lucide/svelte/icons/lock';
	import SearchX from '@lucide/svelte/icons/search-x';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import { describeFailure, type FailureKind } from '$lib/model/failure';
	import { connection } from '$lib/model/connection.svelte';

	let {
		error,
		subject,
		variant = 'inline',
		stale = false,
		onretry,
		children
	}: {
		error: unknown;
		subject?: string;
		variant?: 'inline' | 'block' | 'page';
		stale?: boolean;
		onretry?: () => unknown;
		children?: import('svelte').Snippet;
	} = $props();

	const view = $derived(describeFailure(error, subject));
	const transient = $derived(
		(['offline', 'unreachable', 'busy', 'unavailable'] as FailureKind[]).includes(view.kind)
	);
	const Icon = $derived(
		view.kind === 'offline' || view.kind === 'unreachable'
			? CloudOff
			: view.kind === 'access' || view.kind === 'signed_out'
				? Lock
				: view.kind === 'missing'
					? SearchX
					: CircleAlert
	);
	const detail = $derived(stale && transient ? 'Showing the last update.' : view.detail);
	// Over data still on screen the failure is that this source did not
	// refresh — not, say, that the whole connection was lost.
	const title = $derived(
		stale && transient ? (subject ? `Couldn’t refresh ${subject}` : 'Couldn’t refresh') : view.title
	);
	// Quiet only while the app banner is actually speaking for this outage; a
	// single failing endpoint must still be reported where it failed.
	const silent = $derived(
		stale &&
			((view.kind === 'unreachable' && connection.lost) ||
				(view.kind === 'offline' && typeof navigator !== 'undefined' && !navigator.onLine))
	);

	let retrying = $state(false);
	async function retry() {
		if (!onretry || retrying) return;
		retrying = true;
		try {
			await onretry();
		} catch {
			// The owning query keeps the new failure; this control only reports progress.
		} finally {
			retrying = false;
		}
	}
</script>

{#if !silent}
	<div
		class="failure-notice failure-{variant}"
		class:transient
		role={transient ? 'status' : 'alert'}
		aria-live={transient ? 'polite' : 'assertive'}
	>
		<span class="failure-icon" aria-hidden="true"
			><Icon size={variant === 'page' ? 20 : 15} strokeWidth={1.8} /></span
		>
		<div class="failure-copy">
			<p class="failure-title">
				{title}
				{#if view.technical}<InfoTip text={view.technical} />{/if}
			</p>
			<p class="failure-detail">{detail}</p>
			{#if children}{@render children()}{/if}
		</div>
		{#if onretry && view.retryable}
			<button class="btn small failure-retry" type="button" onclick={retry} disabled={retrying}>
				{retrying ? 'Retrying…' : 'Retry'}
			</button>
		{/if}
	</div>
{/if}

<style>
	.failure-notice {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		margin: 0;
		padding: 10px 12px;
		border: 1px solid color-mix(in srgb, var(--state-danger) 24%, var(--border));
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--state-danger-soft) 55%, var(--surface-pane));
		color: var(--text-secondary);
		font-size: var(--t-body);
		animation: bridge-disclosure-in var(--motion-disclosure) var(--ease-standard) both;
	}

	.failure-notice.transient {
		border-color: var(--border-strong);
		background: var(--surface-alt);
	}

	.failure-icon {
		display: inline-flex;
		flex: none;
		padding-top: 1px;
		color: var(--state-danger);
	}

	.transient .failure-icon {
		color: var(--text-tertiary);
	}

	.failure-copy {
		display: grid;
		flex: 1;
		min-width: 0;
		gap: 2px;
	}

	.failure-copy p {
		margin: 0;
	}

	.failure-title {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1);
		color: var(--ink);
		font-weight: 600;
	}

	.failure-detail {
		color: var(--text-tertiary);
		overflow-wrap: anywhere;
	}

	.failure-retry {
		flex: none;
		align-self: center;
	}

	.failure-block {
		padding: var(--space-4);
	}

	.failure-page {
		flex-direction: column;
		align-items: center;
		align-self: center;
		gap: var(--space-2);
		max-width: 420px;
		margin: auto;
		padding: var(--space-6);
		border: 0;
		background: none;
		text-align: center;
	}

	.failure-page.transient {
		background: none;
	}

	.failure-page .failure-copy {
		justify-items: center;
		gap: var(--space-1);
	}

	.failure-page .failure-title {
		font-size: var(--t-head);
	}

	.failure-page .failure-retry {
		align-self: center;
		margin-top: var(--space-2);
	}
</style>
