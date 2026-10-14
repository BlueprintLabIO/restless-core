<script lang="ts">
	/* The clear Inbox leads with what the company did and what it is doing next; the floor of
	 * people sits below it (Sprint 83). Both lists come from the Work graph the page already
	 * holds, so this adds no reading of its own. */
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { askExec } from '$lib/model/ask-exec';
	import type { WorkRow } from '$lib/model/generated/orgintel';

	let { companyId, work = [] }: { companyId: string; work?: readonly WorkRow[] } = $props();

	const LIMIT = 5;
	const newest = (a: WorkRow, b: WorkRow) => Date.parse(b.updated_at) - Date.parse(a.updated_at);
	const finished = $derived(work.filter((row) => row.status === 'completed').sort(newest));
	/* Blocked first (it may need someone), then by priority, then what moved last. */
	const next = $derived(
		work
			.filter((row) => row.status === 'active' || row.status === 'blocked')
			.sort(
				(a, b) =>
					Number(b.status === 'blocked') - Number(a.status === 'blocked') ||
					b.priority - a.priority ||
					newest(a, b)
			)
	);
	const href = (row: WorkRow) =>
		`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(row.id)}`;
</script>

<div class="clear-home">
	<section aria-labelledby="clear-outcomes">
		<h2 id="clear-outcomes" title="Work the company finished, newest first">Recent outcomes</h2>
		{#if finished.length}
			<ul>
				{#each finished.slice(0, LIMIT) as row (row.id)}
					<li>
						<a href={href(row)} title={row.outcome || row.title}>{row.title}</a>
						<RelativeTime value={row.updated_at} />
					</li>
				{/each}
			</ul>
			{#if finished.length > LIMIT}<a class="more" href={`/${encodeURIComponent(companyId)}/work`}
					>All {finished.length}</a
				>{/if}
		{:else}
			<p class="empty">Nothing finished yet.</p>
		{/if}
	</section>
	<section aria-labelledby="clear-next">
		<h2 id="clear-next" title="Work in progress: blocked first, then by priority">Next</h2>
		{#if next.length}
			<ul>
				{#each next.slice(0, LIMIT) as row (row.id)}
					<li>
						<a href={href(row)} title={row.outcome || row.title}>{row.title}</a>
						{#if row.status === 'blocked'}<span class="blocked">Blocked</span>{/if}
					</li>
				{/each}
			</ul>
			{#if next.length > LIMIT}<a class="more" href={`/${encodeURIComponent(companyId)}/work`}
					>All {next.length}</a
				>{/if}
		{:else}
			<p class="empty">Nothing in progress.</p>
		{/if}
		<button class="btn small primary" type="button" onclick={() => askExec()}
			>Tell Exec what you want</button
		>
	</section>
</div>

<style>
	.clear-home {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
		gap: 16px 32px;
		max-width: 920px;
	}
	section {
		display: grid;
		align-content: start;
		justify-items: start;
		gap: 8px;
		min-width: 0;
	}
	h2 {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 600;
	}
	ul {
		display: grid;
		gap: 2px;
		width: 100%;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	li {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	li a {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		padding: 4px 0;
		color: var(--ink);
		font-size: var(--t-body);
		text-decoration: none;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	li a:hover {
		text-decoration: underline;
	}
	.blocked {
		color: var(--state-danger);
	}
	.more {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.empty {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
</style>
