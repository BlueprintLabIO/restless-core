<script lang="ts">
	/* What the owner set aside as not needed: one quiet fold, out of the way, each item restorable. */
	import { useQueryClient } from '@tanstack/svelte-query';
	import { fetchSetAside, restoreAttention, type SetAside } from '$lib/model/attention';
	import { refreshAttention } from '$lib/model/queries.svelte';

	let { companyId, version = 0 }: { companyId: string; version?: number } = $props();
	const client = useQueryClient();
	let items = $state<SetAside[]>([]);

	async function load() {
		items = await fetchSetAside(companyId).catch(() => items);
	}
	$effect(() => {
		void companyId;
		void version;
		void load();
	});
	async function restore(entry: SetAside) {
		await restoreAttention(companyId, entry.item_id).catch(() => null);
		await refreshAttention(client, companyId);
		await load();
	}
</script>

{#if items.length}
	<details class="set-aside">
		<summary title="Items you marked not needed. Restore one to bring it back."
			>Set aside <span>{items.length}</span></summary
		>
		{#each items as entry (entry.item_id)}
			<div class="set-aside-row">
				<span class="set-aside-title" title={entry.title}>{entry.title}</span>
				<button type="button" class="btn small ghost" onclick={() => void restore(entry)}
					>Restore</button
				>
			</div>
		{/each}
	</details>
{/if}

<style>
	.set-aside {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.set-aside summary {
		padding: 6px 8px;
		border-radius: var(--radius-control);
		cursor: pointer;
		list-style: none;
	}
	.set-aside summary::-webkit-details-marker {
		display: none;
	}
	.set-aside summary:hover {
		background: var(--wash-hover);
	}
	.set-aside summary span {
		margin-left: 4px;
		font-variant-numeric: tabular-nums;
	}
	.set-aside-row {
		display: flex;
		align-items: center;
		gap: 6px;
		max-width: 360px;
		padding: 2px 4px 2px 8px;
	}
	.set-aside-title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
