<script lang="ts">
	import type { Snippet } from 'svelte';
	import { clearOfToasts } from '$lib/ui/feedback/toast.svelte';

	let { children, fields }: { children: Snippet; fields?: Snippet } = $props();
</script>

<!-- Kept outside the scrolling content by each attention view. Toasts leave it clear. -->
<footer
	class="attention-footer"
	class:with-fields={!!fields}
	aria-label="Decision actions"
	data-inbox-actions
	{@attach clearOfToasts}
>
	{#if fields}<div class="footer-fields">{@render fields()}</div>{/if}
	<div class="footer-actions">{@render children()}</div>
</footer>

<style>
	.attention-footer {
		flex: none;
		display: grid;
		align-content: end;
		gap: var(--space-3);
		min-width: 0;
		/* As tall as the Exec composer beside it, so the two top rules are one line, with the
		 * buttons on the line of its send button (measured, owner 12 Nov). */
		min-height: var(--pane-foot-h);
		max-height: 100%;
		padding: 12px 24px max(15px, env(safe-area-inset-bottom));
		border-top: 1px solid var(--border);
		background: var(--surface-pane);
	}
	.attention-footer.with-fields {
		grid-template-rows: minmax(0, 1fr) auto;
	}
	.footer-fields {
		min-height: 0;
		max-height: min(32dvh, 240px);
		overflow: auto;
	}
	.footer-actions {
		min-width: 0;
	}
	.attention-footer :global(.action-row) {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-2);
		min-width: 0;
	}
	@media (max-width: 760px) {
		/* No rail beside it on a phone: the footer is just its buttons. */
		.attention-footer {
			min-height: 64px;
			padding-inline: 14px;
		}
	}
</style>
