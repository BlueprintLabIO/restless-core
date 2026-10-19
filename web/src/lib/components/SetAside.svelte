<script lang="ts">
	/* What the owner skipped as not needed: one quiet fold, out of the way, each item restorable,
	 * with who asked and when it was skipped. The gateway drops items whose request has gone
	 * away, so what is listed can come back. A list that cannot be read says so. */
	import { useQueryClient } from '@tanstack/svelte-query';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { cockpitQuery } from '$lib/model/queries.svelte';
	import { fetchSetAside, restoreAttention, type SetAside } from '$lib/model/attention';
	import { failureSentence } from '$lib/model/failure';
	import { refreshAttention } from '$lib/model/queries.svelte';

	let { companyId, version = 0 }: { companyId: string; version?: number } = $props();
	const client = useQueryClient();
	let items = $state<SetAside[]>([]);
	let restoring = $state('');
	let message = $state('');
	let stale = $state('');
	let query = $state('');
	const cockpit = cockpitQuery(() => companyId);
	const asker = (entry: SetAside) =>
		entry.responsible_actor
			? (cockpit.view?.people.find((person) => person.actor_id === entry.responsible_actor)
					?.display ?? entry.responsible_actor.replaceAll('-', ' '))
			: '';
	const shown = $derived(
		query.trim()
			? items.filter((entry) =>
					`${entry.title} ${asker(entry)}`.toLowerCase().includes(query.trim().toLowerCase())
				)
			: items
	);

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
			/* Keep what was listed, and say plainly that the list could not be read. */
			stale = failureSentence(
				cause,
				items.length ? 'Skipped could not be refreshed.' : 'Skipped could not be read.'
			);
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

{#if items.length || message || stale}
	<details class="set-aside">
		<summary title="Items you skipped. Restore one to bring it back."
			>Skipped{#if items.length}<span>{items.length}</span>{/if}</summary
		>
		{#if message || stale}<p class="set-aside-message" role="status">
				{message || stale}
				{#if stale && !message}<button
						type="button"
						class="btn small ghost"
						onclick={() => void load()}>Retry</button
					>{/if}
			</p>{/if}
		{#if items.length > 6}<input
				class="set-aside-search"
				type="search"
				bind:value={query}
				aria-label="Search skipped"
				placeholder="Search"
			/>{/if}
		{#each shown as entry (entry.item_id)}
			<div class="set-aside-row">
				<span class="set-aside-main">
					<span class="set-aside-title" title={entry.title}>{entry.title}</span>
					<span class="set-aside-meta"
						>{#if asker(entry)}{asker(entry)} ·
						{/if}skipped <RelativeTime value={entry.dismissed_at} /></span
					>
				</span>
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
	.set-aside-main {
		display: grid;
		flex: 1;
		min-width: 0;
	}
	.set-aside-title {
		min-width: 0;
		overflow: hidden;
		color: var(--text-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.set-aside-meta {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.set-aside-meta :global(time) {
		color: inherit;
	}
	.set-aside-search {
		width: calc(100% - 16px);
		max-width: 344px;
		margin: 2px 8px 4px;
	}
</style>
