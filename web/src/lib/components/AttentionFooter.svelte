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
		min-height: 64px;
		max-height: 100%;
		padding: 12px 24px max(16px, env(safe-area-inset-bottom));
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
		.attention-footer {
			padding-inline: 14px;
		}
	}
</style>
