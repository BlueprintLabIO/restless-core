<script lang="ts">
	/* What the owner set aside as not needed: one quiet fold, out of the way, each item restorable.
	 * The gateway drops items whose request has gone away, so what is listed can come back. */
	import { useQueryClient } from '@tanstack/svelte-query';
	import { fetchSetAside, restoreAttention, type SetAside } from '$lib/model/attention';
	import { failureSentence } from '$lib/model/failure';
	import { refreshAttention } from '$lib/model/queries.svelte';

	let { companyId, version = 0 }: { companyId: string; version?: number } = $props();
	const client = useQueryClient();
	let items = $state<SetAside[]>([]);
	let restoring = $state('');
	let message = $state('');
	let stale = $state('');

	/* The company whose items are listed; the fold is reused across a company switch. */
	let listedFor = '';
	async function load() {
		const company = companyId;
		if (company !== listedFor) {
			items = [];
			message = '';
			stale = '';
			listedFor = company;
		}
		try {
			const fresh = await fetchSetAside(company);
			if (company !== companyId) return;
			items = fresh;
			stale = '';
		} catch (cause) {
			if (company !== companyId) return;
			/* Keep what was listed; say it could not be refreshed only when there is a list. */
			stale = items.length ? failureSentence(cause, 'Set aside could not be refreshed.') : '';
		}
	}
	$effect(() => {
		void companyId;
		void version;
		void load();
	});
	async function restore(entry: SetAside) {
		if (restoring) return;
		restoring = entry.item_id;
		message = '';
		try {
			const { waiting } = await restoreAttention(companyId, entry.item_id);
			await refreshAttention(client, companyId);
			if (waiting === false) message = 'Already resolved. There was nothing to bring back.';
			await load();
		} catch (cause) {
			message = failureSentence(cause, 'That was not restored.');
		} finally {
			restoring = '';
		}
	}
</script>

{#if items.length || message}
	<details class="set-aside">
		<summary title="Items you marked not needed. Restore one to bring it back."
			>Set aside{#if items.length}<span>{items.length}</span>{/if}</summary
		>
		{#if message || stale}<p class="set-aside-message" role="status">{message || stale}</p>{/if}
		{#each items as entry (entry.item_id)}
			<div class="set-aside-row">
				<span class="set-aside-title" title={entry.title}>{entry.title}</span>
				<button
					type="button"
					class="btn small ghost"
					disabled={!!restoring}
					onclick={() => void restore(entry)}
					>{restoring === entry.item_id ? 'Restoring…' : 'Restore'}</button
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
	.set-aside-message {
		max-width: 360px;
		margin: 2px 0 4px;
		padding: 0 8px;
		color: var(--text-secondary);
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
