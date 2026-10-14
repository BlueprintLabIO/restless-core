<script lang="ts">
	/* The editor's two block menus: "/" at the start of a line or after a space (insert or turn
	 * the block into another, keys stay in the page), and a block's own actions from its drag
	 * handle or Ctrl+/ (comment, copy its link, duplicate, turn into, delete). Both look and move
	 * like the cockpit's ActionMenu. */
	import { composing } from '$lib/primitives/composer-keys';
	import { tick, type Component } from 'svelte';
	import type { Editor } from '@tiptap/core';
	import { TextSelection } from '@tiptap/pm/state';
	import type { Node as ProseMirrorNode } from '@tiptap/pm/model';
	import Braces from '@lucide/svelte/icons/braces';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Copy from '@lucide/svelte/icons/copy';
	import Heading1 from '@lucide/svelte/icons/heading-1';
	import Heading2 from '@lucide/svelte/icons/heading-2';
	import Heading3 from '@lucide/svelte/icons/heading-3';
	import ImageIcon from '@lucide/svelte/icons/image';
	import Link from '@lucide/svelte/icons/link';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import List from '@lucide/svelte/icons/list';
	import ListChecks from '@lucide/svelte/icons/list-checks';
	import ListOrdered from '@lucide/svelte/icons/list-ordered';
	import Minus from '@lucide/svelte/icons/minus';
	import Pilcrow from '@lucide/svelte/icons/pilcrow';
	import Quote from '@lucide/svelte/icons/quote';
	import Repeat2 from '@lucide/svelte/icons/repeat-2';
	import Table2 from '@lucide/svelte/icons/table-2';
	import Trash from '@lucide/svelte/icons/trash';
	import {
		DOCUMENT_IMAGE_TYPES,
		documentTable,
		slashMenuKey,
		type SlashMenuState
	} from '$lib/model/document-editor-schema';

	interface Props {
		editor: Editor | null;
		slash: SlashMenuState | null;
		/** Image files chosen for a position: the page uploads and places them. */
		onimages: (files: File[], at: number) => void;
		/** Start a comment thread on a block. */
		oncomment?: (blockId: string) => void;
		/** Copy a link that opens the document at a block. */
		onlink?: (blockId: string) => void;
	}

	let { editor, slash, onimages, oncomment, onlink }: Props = $props();

	type Chain = ReturnType<Editor['chain']>;
	interface BlockChoice {
		id: string;
		label: string;
		icon: Component;
		/** Other words someone might type after "/" for it. */
		words: string;
		/** Turns the selected block into this kind; absent for something only inserted. */
		turn?: (chain: Chain) => Chain;
	}

	const KINDS: BlockChoice[] = [
		{
			id: 'text',
			label: 'Text',
			icon: Pilcrow,
			words: 'paragraph plain',
			turn: (c) => c.clearNodes()
		},
		{
			id: 'heading1',
			label: 'Heading 1',
			icon: Heading1,
			words: 'h1 title',
			turn: (c) => c.clearNodes().setHeading({ level: 1 })
		},
		{
			id: 'heading2',
			label: 'Heading 2',
			icon: Heading2,
			words: 'h2 subtitle',
			turn: (c) => c.clearNodes().setHeading({ level: 2 })
		},
		{
			id: 'heading3',
			label: 'Heading 3',
			icon: Heading3,
			words: 'h3',
			turn: (c) => c.clearNodes().setHeading({ level: 3 })
		},
		{
			id: 'bullets',
			label: 'Bulleted list',
			icon: List,
			words: 'ul unordered',
			turn: (c) => c.clearNodes().toggleBulletList()
		},
		{
			id: 'numbers',
			label: 'Numbered list',
			icon: ListOrdered,
			words: 'ol ordered',
			turn: (c) => c.clearNodes().toggleOrderedList()
		},
		{
			id: 'todo',
			label: 'To-do list',
			icon: ListChecks,
			words: 'todo checklist task checkbox',
			turn: (c) => c.clearNodes().toggleTaskList()
		},
		{
			id: 'quote',
			label: 'Quote',
			icon: Quote,
			words: 'blockquote citation',
			turn: (c) => c.clearNodes().toggleBlockquote()
		},
		{
			id: 'code',
			label: 'Code',
			icon: Braces,
			words: 'codeblock snippet',
			turn: (c) => c.clearNodes().toggleCodeBlock()
		}
	];
	const INSERTS: BlockChoice[] = [
		...KINDS,
		{ id: 'divider', label: 'Divider', icon: Minus, words: 'rule line hr separator' },
		{ id: 'table', label: 'Table', icon: Table2, words: 'grid' },
		{ id: 'image', label: 'Image', icon: ImageIcon, words: 'picture photo upload' }
	];

	/* ---------- "/" ---------- */

	let slashIndex = $state(0);
	let slashPanel = $state<HTMLDivElement>();
	let slashPlace = $state<{ left: number; top: number } | null>(null);
	let imageInput = $state<HTMLInputElement>();
	let imageAt = -1;

	/* "/h" finds Heading 1-3; "/todo" or "/check" finds To-do list. */
	const slashItems = $derived.by(() => {
		const query = (slash?.query ?? '').trim().toLowerCase();
		if (!query) return INSERTS;
		return INSERTS.filter(
			(item) =>
				item.label.toLowerCase().includes(query) ||
				`${item.label} ${item.words}`
					.toLowerCase()
					.split(/[\s-]+/u)
					.some((word) => word.startsWith(query))
		);
	});

	$effect(() => {
		void slash?.query;
		slashIndex = 0;
	});

	/* Under the slash, or above it when the page has no room below. It is shown before it is
	 * measured: a hidden panel has no height, and would open off the bottom of the page. */
	$effect(() => {
		const state = slash;
		const panel = slashPanel;
		const open = Boolean(state && editor && slashItems.length);
		if (!panel) return;
		if (!open) {
			if (panel.matches(':popover-open')) panel.hidePopover();
			slashPlace = null;
			return;
		}
		if (!panel.matches(':popover-open')) panel.showPopover();
		if (!state || !editor) return;
		let coords: { left: number; top: number; bottom: number };
		try {
			coords = editor.view.coordsAtPos(state.from);
		} catch {
			slashPlace = null;
			return;
		}
		const height = panel.offsetHeight;
		const width = panel.offsetWidth;
		slashPlace = {
			left: Math.max(8, Math.min(coords.left, innerWidth - width - 8)),
			top:
				coords.bottom + height + 8 > innerHeight
					? Math.max(8, coords.top - height - 4)
					: coords.bottom + 4
		};
	});

	$effect(() => {
		slashPanel?.querySelector(`[data-index="${slashIndex}"]`)?.scrollIntoView({ block: 'nearest' });
	});

	function chooseSlash(item: BlockChoice): void {
		const state = slash;
		if (!editor || !state) return;
		const range = { from: state.from, to: state.from + 1 + state.query.length };
		const chain = editor
			.chain()
			.focus()
			.command(({ tr }) => {
				tr.setMeta(slashMenuKey, null);
				return true;
			})
			.deleteRange(range);
		if (item.turn) item.turn(chain).run();
		else if (item.id === 'divider') chain.setHorizontalRule().run();
		else if (item.id === 'table') chain.insertContent(documentTable()).run();
		else if (item.id === 'image') {
			chain.run();
			pickImages(range.from);
		}
	}

	/** The "/" menu's keys, while it is open; true when it used the key. */
	export function slashKey(event: KeyboardEvent): boolean {
		const items = slashItems;
		if (!slash || !items.length || composing(event)) return false;
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			const step = event.key === 'ArrowDown' ? 1 : -1;
			slashIndex = (slashIndex + step + items.length) % items.length;
			return true;
		}
		if (event.key === 'Enter' || event.key === 'Tab') {
			const item = items[Math.min(slashIndex, items.length - 1)];
			if (item) chooseSlash(item);
			return true;
		}
		return false;
	}

	function pickImages(at: number): void {
		imageAt = at;
		if (imageInput) {
			imageInput.value = '';
			imageInput.click();
		}
	}

	/* ---------- a block's own actions ---------- */

	let blockOpen = $state(false);
	let blockView = $state<'main' | 'turn'>('main');
	let blockPanel = $state<HTMLDivElement>();
	let turnable = $state(false);
	let blockAnchor: DOMRect | null = null;
	/* Where the block was when its actions opened, and who it is: a co-editor may move it meanwhile. */
	let blockPos = -1;
	let blockId: unknown = null;

	/** The block whose actions are open, wherever it is now; null once it is gone. */
	function currentBlock(): { node: ProseMirrorNode; pos: number } | null {
		if (!editor || blockPos < 0) return null;
		const { doc } = editor.state;
		let found: { node: ProseMirrorNode; pos: number } | null = null;
		if (typeof blockId === 'string')
			doc.forEach((node, offset) => {
				if (!found && node.attrs.block_id === blockId) found = { node, pos: offset };
			});
		if (found) return found;
		const node = blockPos < doc.content.size ? doc.nodeAt(blockPos) : null;
		return node && blockId === null ? { node, pos: blockPos } : null;
	}

	/** Open a top-level block's actions beside an anchor (its drag handle, or the block itself). */
	export function openBlockMenu(target: { pos: number; anchor: DOMRect }): void {
		const node = editor?.isEditable ? editor.state.doc.nodeAt(target.pos) : null;
		if (!node) return;
		blockPos = target.pos;
		blockId = typeof node.attrs.block_id === 'string' ? node.attrs.block_id : null;
		turnable =
			node.isTextblock ||
			['bulletList', 'orderedList', 'taskList', 'blockquote'].includes(node.type.name);
		blockAnchor = target.anchor;
		blockView = 'main';
		blockOpen = true;
		void tick().then(() => {
			const panel = blockPanel;
			if (!panel || !blockAnchor) return;
			if (!panel.matches(':popover-open')) panel.showPopover();
			placeBlockMenu();
			requestAnimationFrame(placeBlockMenu);
			focusFirst();
		});
	}

	function placeBlockMenu(): void {
		const panel = blockPanel;
		const anchor = blockAnchor;
		if (!panel || !anchor) return;
		const width = panel.offsetWidth;
		const height = panel.offsetHeight;
		panel.style.left = `${Math.max(8, Math.min(anchor.left, innerWidth - width - 8))}px`;
		panel.style.top = `${Math.max(8, anchor.bottom + height + 8 > innerHeight ? anchor.top - height - 4 : anchor.bottom + 4)}px`;
	}

	function focusFirst(): void {
		void tick().then(() =>
			blockPanel?.querySelector<HTMLElement>('button:not(:disabled):not(.back)')?.focus()
		);
	}

	function closeBlockMenu(refocus: boolean): void {
		if (blockPanel?.matches(':popover-open')) blockPanel.hidePopover();
		blockOpen = false;
		blockPos = -1;
		blockAnchor = null;
		if (refocus) editor?.commands.focus();
	}

	/** Run something on the block whose actions are open, then close them. */
	function onBlock(action: (editor: Editor, node: ProseMirrorNode, pos: number) => void): void {
		const block = currentBlock();
		if (editor && block && editor.isEditable) action(editor, block.node, block.pos);
		closeBlockMenu(false);
	}

	function duplicate(instance: Editor, node: ProseMirrorNode, pos: number): void {
		/* The copy's blocks get identities of their own; the ids the original keeps stay unique. */
		const strip = (value: Record<string, unknown>): Record<string, unknown> => ({
			...value,
			...(value.attrs && typeof value.attrs === 'object'
				? { attrs: { ...(value.attrs as Record<string, unknown>), block_id: null } }
				: {}),
			...(Array.isArray(value.content)
				? { content: (value.content as Record<string, unknown>[]).map(strip) }
				: {})
		});
		const copy = instance.schema.nodeFromJSON(strip(node.toJSON()));
		const end = pos + node.nodeSize;
		const tr = instance.state.tr.insert(end, copy);
		tr.setSelection(TextSelection.near(tr.doc.resolve(end + 1)));
		instance.view.dispatch(tr.scrollIntoView());
		instance.view.focus();
	}

	const COMMENT_KEYS =
		typeof navigator !== 'undefined' && /Mac|iPhone|iPad/u.test(navigator.platform)
			? '⌘⇧M'
			: 'Ctrl+Shift+M';

	function withBlockId(node: ProseMirrorNode, use: (blockId: string) => void): void {
		const id = node.attrs.block_id;
		if (typeof id === 'string' && id) use(id);
	}

	function remove(instance: Editor, node: ProseMirrorNode, pos: number): void {
		const { state } = instance;
		const tr =
			state.doc.childCount === 1
				? state.tr.replaceWith(pos, pos + node.nodeSize, state.schema.nodes.paragraph.create())
				: state.tr.delete(pos, pos + node.nodeSize);
		tr.setSelection(TextSelection.near(tr.doc.resolve(Math.min(pos, tr.doc.content.size))));
		instance.view.dispatch(tr.scrollIntoView());
		instance.view.focus();
	}

	function turnInto(item: BlockChoice): void {
		onBlock((instance, node, pos) => {
			if (!item.turn) return;
			const { doc } = instance.state;
			const selection = TextSelection.between(
				doc.resolve(pos + 1),
				doc.resolve(pos + node.nodeSize - 1)
			);
			item
				.turn(
					instance
						.chain()
						.focus()
						.command(({ tr }) => {
							tr.setSelection(selection);
							return true;
						})
				)
				.run();
		});
	}

	function blockKeys(event: KeyboardEvent): void {
		const panel = blockPanel;
		if (!panel) return;
		if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			if (blockView === 'turn') {
				blockView = 'main';
				focusFirst();
			} else closeBlockMenu(true);
			return;
		}
		if (event.key === 'ArrowLeft' && blockView === 'turn') {
			event.preventDefault();
			blockView = 'main';
			focusFirst();
			return;
		}
		if (event.key === 'ArrowRight' && (event.target as Element).closest('[data-turn]')) {
			event.preventDefault();
			blockView = 'turn';
			focusFirst();
			return;
		}
		const actions = [...panel.querySelectorAll<HTMLElement>('button:not(:disabled)')];
		if (!actions.length || !['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
		event.preventDefault();
		const index = actions.indexOf(document.activeElement as HTMLElement);
		const next =
			event.key === 'Home'
				? 0
				: event.key === 'End'
					? actions.length - 1
					: (index + (event.key === 'ArrowDown' ? 1 : -1) + actions.length) % actions.length;
		actions[next]?.focus();
	}

	$effect(() => {
		if (!blockOpen) return;
		const away = (event: PointerEvent) => {
			if (!blockPanel?.contains(event.target as Node)) closeBlockMenu(false);
		};
		const reposition = () => placeBlockMenu();
		document.addEventListener('pointerdown', away, true);
		window.addEventListener('resize', reposition);
		return () => {
			document.removeEventListener('pointerdown', away, true);
			window.removeEventListener('resize', reposition);
		};
	});
</script>

<input
	bind:this={imageInput}
	class="image-input"
	type="file"
	accept={DOCUMENT_IMAGE_TYPES.join(',')}
	multiple
	tabindex="-1"
	aria-hidden="true"
	onchange={(event) => {
		const files = [...(event.currentTarget.files ?? [])];
		if (files.length && imageAt >= 0) onimages(files, imageAt);
		imageAt = -1;
	}}
/>

<div
	bind:this={slashPanel}
	popover="manual"
	class="block-menu slash-menu"
	role="listbox"
	aria-label="Blocks"
	style:left={slashPlace ? `${slashPlace.left}px` : undefined}
	style:top={slashPlace ? `${slashPlace.top}px` : undefined}
	style:visibility={slashPlace ? undefined : 'hidden'}
>
	{#each slashItems as item, index (item.id)}
		{@const Icon = item.icon}
		<button
			type="button"
			role="option"
			tabindex="-1"
			data-index={index}
			aria-selected={index === slashIndex}
			class:active={index === slashIndex}
			onmousedown={(event) => event.preventDefault()}
			onmouseenter={() => (slashIndex = index)}
			onclick={() => chooseSlash(item)}
			><Icon size={16} strokeWidth={1.8} aria-hidden="true" />{item.label}</button
		>
	{/each}
</div>

<div
	bind:this={blockPanel}
	popover="manual"
	class="block-menu"
	role="menu"
	tabindex="-1"
	aria-label={blockView === 'turn' ? 'Turn into' : 'Block'}
	onkeydown={blockKeys}
>
	{#if blockView === 'main'}
		{#if oncomment}
			<button
				type="button"
				role="menuitem"
				onclick={() => onBlock((_editor, node) => withBlockId(node, oncomment))}
				><MessageSquare size={16} strokeWidth={1.8} aria-hidden="true" />Comment<kbd
					>{COMMENT_KEYS}</kbd
				></button
			>
		{/if}
		{#if onlink}
			<button
				type="button"
				role="menuitem"
				onclick={() => onBlock((_editor, node) => withBlockId(node, onlink))}
				><Link size={16} strokeWidth={1.8} aria-hidden="true" />Copy link to block</button
			>
		{/if}
		<button type="button" role="menuitem" onclick={() => onBlock(duplicate)}
			><Copy size={16} strokeWidth={1.8} aria-hidden="true" />Duplicate</button
		>
		<button
			type="button"
			role="menuitem"
			data-turn
			disabled={!turnable}
			aria-haspopup="menu"
			title={turnable ? undefined : 'Only text, lists and quotes change kind'}
			onclick={() => {
				blockView = 'turn';
				focusFirst();
			}}
			><Repeat2 size={16} strokeWidth={1.8} aria-hidden="true" />Turn into<ChevronRight
				class="trailing"
				size={14}
				strokeWidth={1.8}
				aria-hidden="true"
			/></button
		>
		<button type="button" role="menuitem" class="danger" onclick={() => onBlock(remove)}
			><Trash size={16} strokeWidth={1.8} aria-hidden="true" />Delete</button
		>
	{:else}
		<button
			type="button"
			role="menuitem"
			class="back"
			title="Back to this block's actions"
			onclick={() => {
				blockView = 'main';
				focusFirst();
			}}><ChevronLeft size={16} strokeWidth={1.8} aria-hidden="true" />Turn into</button
		>
		{#each KINDS as item (item.id)}
			{@const Icon = item.icon}
			<button type="button" role="menuitem" onclick={() => turnInto(item)}
				><Icon size={16} strokeWidth={1.8} aria-hidden="true" />{item.label}</button
			>
		{/each}
	{/if}
</div>

<style>
	.image-input {
		display: none;
	}
	/* The cockpit's ActionMenu panel, placed by the editor rather than by a summary. */
	.block-menu {
		position: fixed;
		margin: 0;
		inset: auto;
		z-index: 30;
		display: grid;
		gap: 3px;
		min-width: 200px;
		max-width: min(320px, calc(100vw - 40px));
		padding: 6px;
		border: 1px solid var(--border-strong);
		border-radius: 8px;
		background: var(--surface-raised);
		box-shadow: 0 8px 28px #18243a18;
	}
	.block-menu:not(:popover-open) {
		display: none;
	}
	.slash-menu {
		max-height: min(340px, calc(100vh - 32px));
		overflow-y: auto;
		overscroll-behavior: contain;
	}
	.block-menu button {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		min-height: 34px;
		padding: 7px 10px;
		border: 0;
		border-radius: 4px;
		background: transparent;
		color: var(--ink);
		text-align: left;
		font: inherit;
		font-size: var(--t-body);
		cursor: pointer;
	}
	.block-menu button :global(svg) {
		flex: none;
		color: var(--text-secondary);
	}
	.block-menu button :global(.trailing) {
		margin-left: auto;
	}
	.block-menu button:hover:not(:disabled),
	.block-menu button.active {
		background: var(--surface-alt);
	}
	.block-menu button:focus-visible {
		outline: 2px solid var(--intent-feedback);
		outline-offset: -2px;
	}
	.block-menu button:disabled {
		opacity: 0.42;
		cursor: not-allowed;
	}
	.block-menu button.back {
		color: var(--text-secondary);
	}
	.block-menu kbd {
		margin-left: auto;
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-ui);
	}
	.block-menu button.danger,
	.block-menu button.danger :global(svg) {
		color: var(--state-danger);
	}
	@media (pointer: coarse) {
		.block-menu button {
			min-height: 44px;
		}
	}
</style>
