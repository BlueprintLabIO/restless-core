<script lang="ts">
	/* A small text file read in place: Markdown formatted, a CSV or TSV as a table, JSON indented,
	 * anything else as written. Links in a note open like links anywhere else in the cockpit. */
	import Markdown from './Markdown.svelte';
	import { parseDelimited, readableJson, type TextFormat } from '$lib/model/file-text';

	let { text, format }: { text: string; format: TextFormat } = $props();

	const table = $derived(
		format === 'csv' || format === 'tsv'
			? parseDelimited(text, format === 'csv' ? ',' : '\t')
			: null
	);
	const header = $derived(table?.rows[0] ?? []);
	const body = $derived(table?.rows.slice(1) ?? []);
</script>

{#if format === 'markdown'}
	<div class="text-paper"><Markdown {text} /></div>
{:else if table}
	<div class="text-table-wrap">
		{#if table.rows.length}
			<table class="text-table">
				<thead
					><tr
						><th class="row-number" aria-label="Row"></th>{#each header as cell, column (column)}<th
								>{cell}</th
							>{/each}</tr
					></thead
				>
				<tbody>
					{#each body as row, index (index)}
						<tr
							><td class="row-number">{index + 1}</td>{#each row as cell, column (column)}<td
									>{cell}</td
								>{/each}</tr
						>
					{/each}
				</tbody>
			</table>
			{#if table.truncated}<p class="text-more">
					The first {table.rows.length.toLocaleString()} rows. Download the file for the rest.
				</p>{/if}
		{:else}<p class="text-more">This file is empty.</p>{/if}
	</div>
{:else}
	<pre class="text-plain">{format === 'json' ? readableJson(text) : text}</pre>
{/if}

<style>
	.text-paper {
		max-width: 72ch;
		margin: 0 auto;
		padding: 28px 28px 56px;
		color: var(--text-secondary);
		line-height: 1.6;
	}
	.text-paper :global(:is(strong, h1, h2, h3, h4)) {
		color: var(--ink);
	}
	.text-table-wrap {
		height: 100%;
		overflow: auto;
		background: var(--surface-pane, var(--surface));
	}
	.text-table {
		border-collapse: collapse;
		font-size: var(--t-body);
		font-variant-numeric: tabular-nums;
	}
	.text-table :is(th, td) {
		max-width: 320px;
		padding: 5px 10px;
		overflow: hidden;
		border-right: 1px solid var(--border);
		border-bottom: 1px solid var(--border);
		text-align: left;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.text-table thead th {
		position: sticky;
		top: 0;
		z-index: 1;
		background: var(--surface-alt);
		color: var(--ink);
		font-weight: 600;
	}
	.text-table .row-number {
		min-width: 40px;
		color: var(--text-tertiary);
		text-align: right;
	}
	.text-more {
		margin: 0;
		padding: 12px 16px;
		color: var(--text-tertiary);
	}
	.text-plain {
		height: 100%;
		margin: 0;
		padding: 20px 24px;
		overflow: auto;
		background: var(--surface-pane, var(--surface));
		color: var(--ink);
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: var(--t-label);
		line-height: 1.55;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
</style>
