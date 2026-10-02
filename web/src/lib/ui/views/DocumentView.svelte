<script lang="ts" module>
	import type { ActorKind } from '../glyph/ActorTag.svelte';

	export type DocBlock = {
		kind: 'h1' | 'h2' | 'p' | 'li' | 'quote';
		text: string;
		/** A phrase inside `text` that a collaborator has selected or commented on. */
		mark?: { phrase: string; by: string };
	};
	export type DocCollaborator = { id: string; name: string; kind: ActorKind; color: string };
	/** Where a collaborator's caret is: the end of block `block`. */
	export type DocPresence = { by: string; block: number };
	export type DocComment = {
		id: string;
		author: string;
		kind: ActorKind;
		time: string;
		text: string;
		/** The passage the comment is attached to; absent for the whole document. */
		anchor?: string;
		resolved?: boolean;
	};
	export type DocSummary = { id: string; title: string; meta: string };
</script>

<script lang="ts">
	import Bold from '@lucide/svelte/icons/bold';
	import Check from '@lucide/svelte/icons/check';
	import Code from '@lucide/svelte/icons/code';
	import History from '@lucide/svelte/icons/history';
	import Italic from '@lucide/svelte/icons/italic';
	import List from '@lucide/svelte/icons/list';
	import ListChecks from '@lucide/svelte/icons/list-checks';
	import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';
	import MessagesSquare from '@lucide/svelte/icons/messages-square';
	import Quote from '@lucide/svelte/icons/quote';
	import Scale from '@lucide/svelte/icons/scale';
	import ActorTag from '../glyph/ActorTag.svelte';

	/* A shared document as the owner workspace shows it: the list, the editor chrome, the page with
	 * collaborators' carets and selections, and the comments beside it. Read-only; every state is a
	 * prop, so a page can step through an editing session by changing them. */
	let {
		title,
		kind = 'Brief',
		blocks,
		collaborators = [],
		presence = [],
		comments = [],
		documents = [],
		activeDocument,
		synced = true,
		panel = 'comments',
		showList = true,
		showPanel = true
	}: {
		title: string;
		kind?: string;
		blocks: DocBlock[];
		collaborators?: DocCollaborator[];
		presence?: DocPresence[];
		comments?: DocComment[];
		documents?: DocSummary[];
		activeDocument?: string;
		synced?: boolean;
		panel?: 'comments' | 'review' | 'versions';
		showList?: boolean;
		showPanel?: boolean;
	} = $props();

	const byId = $derived(new Map(collaborators.map((person) => [person.id, person])));
	const open = $derived(comments.filter((comment) => !comment.resolved));

	function split(block: DocBlock) {
		if (!block.mark) return [{ text: block.text, mark: null }];
		const at = block.text.indexOf(block.mark.phrase);
		if (at < 0) return [{ text: block.text, mark: null }];
		return [
			{ text: block.text.slice(0, at), mark: null },
			{ text: block.mark.phrase, mark: byId.get(block.mark.by) ?? null },
			{ text: block.text.slice(at + block.mark.phrase.length), mark: null }
		];
	}
</script>

<div class="document-view" class:no-list={!showList || !documents.length} class:no-panel={!showPanel}>
	{#if showList && documents.length}
		<nav class="doc-list" aria-label="Documents">
			<p class="doc-list-head">Documents</p>
			{#each documents as doc (doc.id)}
				<span class="doc-entry" class:active={doc.id === activeDocument}>
					<strong>{doc.title}</strong>
					<span>{doc.meta}</span>
				</span>
			{/each}
		</nav>
	{/if}

	<section class="doc-main" aria-label={title}>
		<header class="doc-head">
			<strong class="doc-title">{title}</strong>
			<span class="doc-kind">{kind}</span>
			<span class="doc-sync" class:pending={!synced}>
				{#if synced}<Check size={12} strokeWidth={2.6} /> Synced with collaborators{:else}Saving…{/if}
			</span>
			<span class="doc-people" aria-label="Editing now">
				{#each collaborators as person (person.id)}
					<span class="doc-face" style:--c={person.color} title="{person.name}">{person.name.slice(0, 1)}</span>
				{/each}
			</span>
		</header>
		<div class="doc-tools" aria-hidden="true">
			<Bold size={14} /><Italic size={14} /><Code size={14} /><span class="sep"></span><List size={14} /><ListChecks size={14} /><Quote size={14} />
			<span class="doc-comment-tool"><MessageSquarePlus size={13} /> Comment</span>
		</div>
		<div class="doc-page">
			{#each blocks as block, index (index)}
				{@const carets = presence.filter((entry) => entry.block === index)}
				<svelte:element this={block.kind === 'li' ? 'p' : block.kind === 'quote' ? 'blockquote' : block.kind} class="doc-block {block.kind}">
					{#each split(block) as part, p (p)}{#if part.mark}<mark style:--c={part.mark.color}>{part.text}</mark>{:else}{part.text}{/if}{/each}{#each carets as caret (caret.by)}{@const person = byId.get(caret.by)}{#if person}<span class="caret" style:--c={person.color}><i></i><em>{person.name} <ActorTag kind={person.kind} /></em></span>{/if}{/each}
				</svelte:element>
			{/each}
		</div>
	</section>

	{#if showPanel}
		<aside class="doc-panel" aria-label="Discussion">
			<div class="doc-tabs">
				<span class:active={panel === 'comments'}><MessagesSquare size={13} /> Comments {#if open.length}<b>{open.length}</b>{/if}</span>
				<span class:active={panel === 'review'}><Scale size={13} /> Review</span>
				<span class:active={panel === 'versions'}><History size={13} /> Versions</span>
			</div>
			<p class="doc-panel-head"><strong>Discussion</strong> Feedback stays attached to this document.</p>
			<ol class="doc-comments">
				{#each comments as comment (comment.id)}
					<li class:resolved={comment.resolved}>
						<p class="who"><b>{comment.author}</b> <ActorTag kind={comment.kind} /> <time>{comment.time}</time></p>
						{#if comment.anchor}<p class="anchor">“{comment.anchor}”</p>{/if}
						<p class="text">{comment.text}</p>
						{#if comment.resolved}<p class="state">Resolved</p>{/if}
					</li>
				{:else}
					<li class="empty">No comments yet. Start a thread on the document or a passage.</li>
				{/each}
			</ol>
		</aside>
	{/if}
</div>

<style>
	.document-view {
		display: grid;
		grid-template-columns: minmax(150px, 210px) minmax(0, 1fr) minmax(200px, 270px);
		min-width: 0;
		height: 100%;
		background: var(--surface-pane);
		color: var(--ink);
		font: 400 var(--t-body) var(--font-ui);
	}
	.document-view.no-list {
		grid-template-columns: minmax(0, 1fr) minmax(200px, 270px);
	}
	.document-view.no-panel {
		grid-template-columns: minmax(150px, 210px) minmax(0, 1fr);
	}
	.document-view.no-list.no-panel {
		grid-template-columns: minmax(0, 1fr);
	}
	.doc-list {
		display: grid;
		align-content: start;
		border-right: 1px solid var(--border);
	}
	.doc-list-head {
		margin: 0;
		padding: 12px 14px;
		border-bottom: 1px solid var(--border);
		font-weight: 650;
		font-size: var(--t-head);
	}
	.doc-entry {
		display: grid;
		gap: 2px;
		padding: 10px 14px;
		border-bottom: 1px solid var(--border);
	}
	.doc-entry.active {
		box-shadow: inset 3px 0 0 var(--intent-conversation);
		background: var(--surface-alt);
	}
	.doc-entry strong {
		font-weight: 600;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.doc-entry span {
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.doc-main {
		display: grid;
		grid-template-rows: auto auto 1fr;
		min-width: 0;
	}
	.doc-head {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
		padding: 10px 16px;
		border-bottom: 1px solid var(--border);
	}
	.doc-title {
		font-size: var(--t-head);
		font-weight: 650;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.doc-kind {
		padding: 3px 8px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
		font-size: var(--t-label);
	}
	.doc-sync {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		font: 500 var(--t-label) var(--font-mono);
		color: var(--state-success);
		white-space: nowrap;
	}
	.doc-sync.pending {
		color: var(--text-tertiary);
	}
	.doc-people {
		display: flex;
		margin-left: auto;
	}
	.doc-face {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		margin-left: -5px;
		border: 2px solid var(--surface-pane);
		border-radius: 50%;
		background: var(--c);
		color: #fff;
		font: 650 var(--t-label) var(--font-ui);
	}
	.doc-tools {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 8px 16px;
		border-bottom: 1px solid var(--border);
		color: var(--text-secondary);
	}
	.sep {
		width: 1px;
		height: 14px;
		background: var(--border-strong);
	}
	.doc-comment-tool {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		margin-left: auto;
		font-size: var(--t-label);
		font-weight: 600;
	}
	.doc-page {
		min-height: 0;
		overflow: auto;
		padding: 22px clamp(16px, 6%, 64px) 28px;
		background: var(--surface-raised);
	}
	.doc-block {
		position: relative;
		margin: 0 0 10px;
		line-height: 1.55;
	}
	.doc-block.h1 {
		margin: 6px 0 8px;
		font-size: var(--t-title);
		font-weight: 680;
		letter-spacing: -0.02em;
		line-height: 1.15;
	}
	.doc-block.h2 {
		margin: 18px 0 6px;
		font-size: var(--t-head);
		font-weight: 680;
		letter-spacing: -0.01em;
	}
	.doc-block.li {
		padding-left: 16px;
	}
	.doc-block.li::before {
		content: '';
		position: absolute;
		left: 3px;
		top: 0.62em;
		width: 5px;
		height: 5px;
		border-radius: 50%;
		background: var(--text-secondary);
	}
	.doc-block.quote {
		padding-left: 12px;
		border-left: 3px solid var(--border-strong);
		color: var(--text-secondary);
	}
	mark {
		padding: 1px 0;
		border-radius: 2px;
		background: color-mix(in srgb, var(--c) 26%, transparent);
		box-shadow: inset 0 -2px 0 color-mix(in srgb, var(--c) 70%, transparent);
		color: inherit;
		transition: background-color var(--motion-state) var(--ease-standard);
	}
	.caret {
		position: relative;
		display: inline-block;
		width: 0;
		height: 1.1em;
		vertical-align: text-bottom;
	}
	.caret i {
		position: absolute;
		left: 0;
		top: 0;
		bottom: 0;
		width: 2px;
		background: var(--c);
		animation: caret-blink 1s steps(1) infinite;
	}
	.caret em {
		position: absolute;
		left: 0;
		bottom: 100%;
		display: inline-flex;
		align-items: center;
		gap: 4px;
		padding: 1px 4px 1px 5px;
		border-radius: 3px 3px 3px 0;
		background: var(--c);
		color: #fff;
		font: 600 var(--t-label)/1.4 var(--font-ui);
		font-style: normal;
		white-space: nowrap;
		z-index: 1;
	}
	@keyframes caret-blink {
		50% {
			opacity: 0.25;
		}
	}
	.doc-panel {
		display: grid;
		grid-template-rows: auto auto 1fr;
		min-width: 0;
		min-height: 0;
		border-left: 1px solid var(--border);
	}
	.doc-tabs {
		display: flex;
		gap: 2px;
		padding: 6px;
		border-bottom: 1px solid var(--border);
	}
	.doc-tabs span {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		padding: 5px 8px;
		border-radius: var(--radius-control);
		font-size: var(--t-label);
		font-weight: 600;
		color: var(--text-tertiary);
	}
	.doc-tabs span.active {
		color: var(--intent-direction);
		box-shadow: inset 0 -2px 0 var(--intent-direction);
	}
	.doc-tabs b {
		padding: 0 5px;
		border-radius: 999px;
		background: var(--intent-direction-soft);
		font-weight: 600;
	}
	.doc-panel-head {
		display: grid;
		gap: 2px;
		margin: 0;
		padding: 10px 12px;
		border-bottom: 1px solid var(--border);
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.doc-panel-head strong {
		font-size: var(--t-body);
		color: var(--ink);
	}
	.doc-comments {
		display: grid;
		align-content: start;
		gap: 0;
		margin: 0;
		padding: 0;
		overflow: auto;
		list-style: none;
	}
	.doc-comments li {
		display: grid;
		gap: 4px;
		padding: 10px 12px;
		border-bottom: 1px solid var(--border);
		animation: comment-in var(--motion-disclosure) var(--ease-standard) both;
	}
	@keyframes comment-in {
		from {
			opacity: 0;
			transform: translateY(6px);
		}
	}
	.doc-comments li.resolved {
		opacity: 0.6;
	}
	.doc-comments p {
		margin: 0;
	}
	.who {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.who time {
		margin-left: auto;
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	.anchor {
		padding-left: 8px;
		border-left: 2px solid var(--border-strong);
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.text {
		line-height: 1.45;
	}
	.state {
		font: 500 var(--t-label) var(--font-mono);
		color: var(--state-success);
	}
	.empty {
		color: var(--text-tertiary);
	}
	@media (max-width: 860px) {
		.document-view,
		.document-view.no-list,
		.document-view.no-panel {
			grid-template-columns: minmax(0, 1fr);
		}
		.doc-list,
		.doc-panel {
			display: none;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.caret i,
		.doc-comments li {
			animation: none;
		}
	}
</style>
