<script lang="ts">
	/* Something is on its way: one small steady spinner and what it is doing, in place of the thing
	 * it will become. It never alternates with an error or an empty state while it waits. */
	let { label }: { label: string } = $props();
</script>

<span class="pending" role="status"><i aria-hidden="true"></i>{label}</span>

<style>
	.pending {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: nowrap;
	}
	i {
		width: 11px;
		height: 11px;
		border: 1.5px solid color-mix(in srgb, var(--intent-conversation) 22%, transparent);
		border-top-color: var(--intent-conversation);
		border-radius: 50%;
		animation: spin 0.9s linear infinite;
	}
	@keyframes spin {
		to {
			transform: rotate(1turn);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		i {
			animation: pulse 1.6s ease-in-out infinite;
		}
		@keyframes pulse {
			50% {
				opacity: 0.4;
			}
		}
	}
</style>
