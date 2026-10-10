<script module lang="ts">
	export type BoardItem = {
		id: string;
		title: string;
		/** One quiet line: run state, gates, outputs. */
		signal?: string;
		status: 'proposed' | 'active' | 'blocked' | 'completed' | (string & {});
		ownerName: string;
		revision: number;
		href: string;
	};

	export type BoardColumn = {
		key: string;
		label: string;
		items: BoardItem[];
		/** Shown when the column has no items. Omit for a silent empty column. */
		emptyNote?: string;
	};
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import { listFlip, listIn, listOut } from '../motion';

	let {
		columns,
		footer,
		label = 'Work board'
	}: {
		columns: BoardColumn[];
		/** Per-column footer, for controls that belong to the container (for example history). */
		footer?: Snippet<[BoardColumn]>;
		label?: string;
	} = $props();
</script>

<div class="work-board" aria-label={label}>
	{#each columns as column (column.key)}
		<section class="board-column">
			<header><span>{column.label}</span><b>{column.items.length}</b></header>
			{#each column.items as item (item.id)}
				<a
					class="board-item status-{item.status}"
					animate:listFlip
					in:listIn
					out:listOut
					href={item.href}
					aria-label={`Open Work: ${item.title}`}
				>
					<strong style:view-transition-name={`work-title-${item.id}`}>{item.title}</strong>
					{#if item.signal}<p>{item.signal}</p>{/if}
					<footer>
						<span>{item.ownerName}</span>
						{#if item.revision > 1}<span title="Revision">R{item.revision}</span>{/if}
					</footer>
				</a>
			{:else}
				<p class="column-empty">
					{#if column.emptyNote}{column.emptyNote}{:else}<span class="sr-only">No work here</span
						>{/if}
				</p>
			{/each}
			{@render footer?.(column)}
		</section>
	{/each}
</div>

<style>
	.work-board {
		min-width: 0;
		min-height: 0;
		display: grid;
		/* Four columns fit beside the full-width sidebar and the Exec rail; a narrower pane scrolls. */
		grid-template-columns: repeat(4, minmax(150px, 1fr));
		overflow: auto;
	}

	.board-column {
		--column-tone: var(--intent-direction);
		min-width: 0;
		padding: 12px;
		border-right: 1px solid var(--border);
		background: color-mix(in srgb, var(--highlight) 46%, transparent);
	}
	.board-column:nth-child(2) {
		--column-tone: var(--intent-conversation);
	}
	.board-column:nth-child(3) {
		--column-tone: var(--intent-authority);
	}
	.board-column:nth-child(4) {
		--column-tone: var(--state-success);
	}
	.board-column::before {
		content: '';
		display: block;
		height: 2px;
		margin: -12px -12px 11px;
		background: color-mix(in srgb, var(--column-tone) 62%, transparent);
	}
	.board-column > header {
		display: flex;
		align-items: baseline;
		gap: 8px;
		margin-bottom: 10px;
		color: var(--ink);
		font-weight: 600;
	}
	.board-column > header b {
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
	}

	.board-item {
		position: relative;
		display: block;
		width: 100%;
		margin-bottom: 9px;
		padding: 12px 13px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--highlight) 82%, transparent);
		box-shadow:
			var(--bevel-subtle),
			0 1px 2px rgba(43, 51, 66, 0.07),
			0 5px 14px rgba(43, 51, 66, 0.045);
		color: var(--ink);
		text-align: left;
		text-decoration: none;
		cursor: pointer;
		transition:
			background-color var(--motion-state) var(--ease-standard),
			border-color var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard),
			transform var(--motion-press) var(--ease-standard);
	}
	.board-item:hover {
		border-color: color-mix(in srgb, var(--ink) 24%, transparent);
		background-color: var(--surface-raised);
		box-shadow:
			var(--bevel),
			0 10px 26px rgba(43, 51, 66, 0.09);
	}
	.board-item:active {
		transform: translateY(1px);
	}
	.board-item:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.board-item strong {
		display: block;
		font-size: var(--t-body);
		line-height: 1.35;
	}
	.board-item p {
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
		margin: 6px 0 9px;
		font-size: var(--t-body);
		line-height: 1.45;
		color: var(--text-secondary);
	}
	.board-item footer {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		margin-top: 10px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}

	.column-empty {
		padding: 4px 2px 8px;
		color: var(--text-tertiary);
	}

	@media (max-width: 760px) {
		.work-board {
			grid-template-columns: 1fr;
			align-content: start;
		}
		.board-column {
			border-right: 0;
			border-bottom: 1px solid var(--border);
		}
		/* Stacked, a full-width tone bar reads as a stray rule; the tone moves to a
		 * status dot beside the column name, like the dots on cards. */
		.board-column::before {
			display: none;
		}
		.board-column > header::before {
			width: 6px;
			height: 6px;
			align-self: center;
			flex: none;
			border-radius: 50%;
			background: var(--column-tone);
			content: '';
		}
		.board-column:has(.column-empty) {
			padding-block: 12px 4px;
		}
	}
</style>
