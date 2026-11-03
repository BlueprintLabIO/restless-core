<script module lang="ts">
	export type BoardItem = {
		id: string;
		title: string;
		/** One quiet line: run state, gates, outputs. */
		signal?: string;
		/** Why it waits and on whom, in one line. Any string, even an empty one, reserves the line
		 * so the card never grows when it arrives; leave it out on cards that never wait. */
		waiting?: string;
		status: 'proposed' | 'active' | 'blocked' | 'completed' | (string & {});
		ownerName: string;
		revision: number;
		href: string;
		/** A decision or step waits on the viewer: the card says so before anything else. */
		needsYou?: boolean;
		/** Waiting in Next while its owner already has this many in motion (G2): could go to Staff. */
		ownerBusy?: number;
		/** Redone past a few revisions (G3): worth asking the lead why. */
		churning?: boolean;
		/** The creator's rough estimate: a bar and "~60% · next: …", who and when on hover. Any value,
		 * even null, reserves the line so the card never grows when an estimate lands; leave it out
		 * on cards that never carry one. */
		progress?: {
			fill: number;
			text: string;
			title: string;
			tone: 'progress' | 'blocked' | 'done' | 'stopped';
		} | null;
	};

	export type BoardColumn = {
		key: string;
		label: string;
		items: BoardItem[];
		/** Shown when the column has no items. Omit for a silent empty column. */
		emptyNote?: string;
		/** Stacked first when the board is one column on a narrow pane: what most needs a look. */
		lead?: boolean;
	};
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import { listFlip, listIn, listOut } from '../motion';
	import Estimate from '../data/Estimate.svelte';
	import { truncatedTitle } from '../truncated-title';

	const id = $props.id();

	let {
		columns,
		footer,
		label = 'Work board',
		onopen
	}: {
		columns: BoardColumn[];
		/** Per-column footer, for controls that belong to the container (for example history). */
		footer?: Snippet<[BoardColumn]>;
		label?: string;
		/** Opens a card in place instead of following its link. */
		onopen?: (item: BoardItem) => void;
	} = $props();
</script>

<!-- A region of column sections, each named by its heading; a card reads as its own words. -->
<div
	class="work-board"
	role="region"
	aria-label={label}
	style:--board-tracks={columns.map(() => 'minmax(180px, 1fr)').join(' ')}
>
	{#each columns as column (column.key)}
		<section class="board-column" class:lead={column.lead} aria-labelledby={`${id}-${column.key}`}>
			<header>
				<h2 id={`${id}-${column.key}`}>{column.label}</h2>
				<b>{column.items.length}</b>
			</header>
			{#each column.items as item (item.id)}
				<a
					class="board-item status-{item.status}"
					class:needs-you={item.needsYou}
					animate:listFlip
					in:listIn
					out:listOut
					href={item.href}
					onclick={onopen
						? (event) => {
								if (event.metaKey || event.ctrlKey || event.shiftKey) return;
								event.preventDefault();
								onopen(item);
							}
						: undefined}
				>
					{#if item.needsYou}<span class="needs-you-badge">Needs you</span>{/if}
					<strong title={item.title} style:view-transition-name={`work-title-${item.id}`}
						>{item.title}</strong
					>
					{#if item.waiting !== undefined}<span class="waiting" title={item.waiting || undefined}
							>{item.waiting}</span
						>{/if}
					{#if item.progress !== undefined}<span class="progress"
							>{#if item.progress}<Estimate
									fill={item.progress.fill}
									text={item.progress.text}
									title={item.progress.title}
									tone={item.progress.tone}
								/>{/if}</span
						>{/if}
					{#if item.signal}<p use:truncatedTitle={item.signal}>{item.signal}</p>{/if}
					<footer>
						<span
							>{item.ownerName}{#if item.ownerBusy}<em
									class="flag"
									title={`${item.ownerName} already has ${item.ownerBusy} in motion; this could go to someone else`}
								>
									· {item.ownerBusy} in motion</em
								>{/if}</span
						>
						{#if item.revision > 1}<span
								class:flag={item.churning}
								title={item.churning ? 'Redone many times; ask the lead why' : undefined}
								>Revision {item.revision}</span
							>{/if}
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
		/* Equal tracks prevent columns from resizing as work moves between them. */
		grid-template-columns: var(--board-tracks);
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
	.board-column > header h2 {
		margin: 0;
		font: inherit;
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
	.board-item.needs-you {
		border-color: color-mix(in srgb, var(--intent-authority) 55%, var(--border-strong));
	}
	.needs-you-badge {
		display: inline-block;
		margin-bottom: 6px;
		padding: 1px 7px;
		border-radius: 999px;
		background: color-mix(in srgb, var(--intent-authority) 16%, transparent);
		color: color-mix(in srgb, var(--intent-authority) 70%, var(--ink));
		font-size: var(--t-label);
		font-weight: 600;
	}
	.board-item strong {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		overflow-wrap: anywhere;
		font-size: var(--t-body);
		line-height: 1.35;
	}
	.board-item .waiting {
		display: block;
		height: 1.4em;
		margin-top: 6px;
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.4;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* The estimate's line is held whether or not an estimate has landed yet. */
	.board-item .progress {
		display: flex;
		height: 1.4em;
		margin-top: 6px;
		min-width: 0;
		overflow: hidden;
		font-size: var(--t-label);
		line-height: 1.4;
	}
	/* Priority, holder and outputs: one label-sized line like the lines above it. At body size it
	 * wrapped "4 outputs" onto a line of its own in a narrow column (S110). */
	.board-item p {
		overflow: hidden;
		margin: 6px 0 0;
		font-size: var(--t-label);
		line-height: 1.4;
		color: var(--text-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.board-item footer {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		margin-top: 10px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.board-item footer span:first-child {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.board-item footer span:last-child:not(:first-child) {
		flex-shrink: 0;
	}
	.board-item footer .flag {
		color: color-mix(in srgb, var(--intent-authority) 70%, var(--ink));
		font-style: normal;
		font-weight: 600;
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
		.board-column.lead {
			order: -1;
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
