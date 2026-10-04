<script lang="ts">
	/* The way back from an editor to the Library it was opened from. */
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	let { companyId, show }: { companyId: string; show: 'docs' | 'sheets' } = $props();
</script>

<a
	class="crumb"
	href={`/${encodeURIComponent(companyId)}/library?show=${show}`}
	title="Back to the Library"
	><ArrowLeft size={15} strokeWidth={1.9} aria-hidden="true" /><span>Library</span></a
><span class="sep" aria-hidden="true">/</span>

<style>
	.crumb {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: 6px;
		height: 28px;
		padding: 0 8px 0 6px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
		transition: background-color var(--motion-state) var(--ease-standard);
	}
	.crumb:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.sep {
		flex: none;
		color: var(--text-tertiary);
	}
	@container (max-width: 560px) {
		.crumb span,
		.sep {
			display: none;
		}
	}
</style>
