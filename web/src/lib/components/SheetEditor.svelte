<script lang="ts">
	import { toastWhile } from '$lib/ui/feedback/toast.svelte';
	import type { EditorStatus, SheetMark, SheetSelection } from '$lib/model/sheet-editor';
	import type { PageFace } from '$lib/model/page-faces';
	let {
		company,
		sheet,
		onstatus,
		onfaces,
		onselection,
		oncomment,
		marks = []
	}: {
		company: string;
		sheet: string;
		onstatus?: (value: EditorStatus) => void;
		onfaces?: (value: PageFace[]) => void;
		/** The selected cell, each time it changes. */
		onselection?: (value: SheetSelection) => void;
		/** The person asked to comment on the selected cell, from the grid's menu or Ctrl+Alt+M. */
		oncomment?: () => void;
		/** Cells that carry a comment thread, marked in the grid. */
		marks?: readonly SheetMark[];
	} = $props();
	let host: HTMLDivElement | undefined = $state();
	let editorState = $state<EditorStatus>({ state: 'connecting', access: 'read' });
	let notice = $state<EditorStatus['notice']>();
	let retry = $state(0);
	let editor:
		| {
				getModel(): any;
				select(worksheet: string, cell: string): boolean;
				mark(value: readonly SheetMark[]): void;
				print(title: string): boolean;
				destroy(): void;
		  }
		| undefined = $state.raw();
	/** The live workbook model, for downloading it; undefined until the sheet has opened. */
	export function getModel(): any {
		return editor?.getModel();
	}
	/** Print the selection or the tab's used cells; false when there is nothing to print. */
	export function print(title: string): boolean {
		return editor?.print(title) ?? false;
	}
	/** Show a cell, on its tab; false when the sheet is not open or the tab is gone. */
	export function select(worksheet: string, cell: string): boolean {
		return editor?.select(worksheet, cell) ?? false;
	}
	$effect(() => {
		editor?.mark(marks);
	});
	$effect(() => {
		const targetCompany = company,
			targetSheet = sheet;
		retry;
		let disposed = false;
		notice = undefined;
		const surface = host;
		if (!surface) return;
		void import('$lib/model/sheet-editor')
			.then((module) => {
				if (disposed) return;
				editor = module.mountSheet(
					surface,
					targetCompany,
					targetSheet,
					(value: EditorStatus) => {
						if (!disposed) {
							editorState = value;
							if (value.notice) notice = value.notice;
							onstatus?.(value);
						}
					},
					(value) => {
						if (!disposed) onfaces?.(value);
					},
					(value) => {
						if (!disposed) onselection?.(value);
					},
					() => {
						if (!disposed) oncomment?.();
					}
				);
			})
			.catch((error) => {
				if (!disposed) editorState = { state: 'error', access: 'read', message: error.message };
			});
		return () => {
			disposed = true;
			onfaces?.([]);
			editor?.destroy();
			editor = undefined;
		};
	});
	/* The spreadsheet engine draws a few controls without names and with a tab order of its own.
	 * Name them, and keep the page's tab order, however often it draws them again. */
	const UNNAMED: [string, string][] = [
		['.o-font-size', 'Font size'],
		['.o-grid-add-rows-input', 'Rows to add'],
		['.o-grid-composer .o-composer', 'Cell']
	];
	/* Said once, where the keyboard lands in the grid: how to leave it. */
	const LEAVE_HINT = 'Escape, then Tab, leaves the grid. Ctrl+Alt+M comments on the cell.';
	$effect(() => {
		const root = host;
		if (!root) return;
		let frame = 0;
		const repair = () => {
			frame = 0;
			for (const [selector, name] of UNNAMED)
				for (const control of root.querySelectorAll<HTMLElement>(selector))
					if (!control.getAttribute('aria-label')) control.setAttribute('aria-label', name);
			for (const cell of root.querySelectorAll<HTMLElement>('.o-grid-composer .o-composer'))
				if (!cell.getAttribute('aria-description'))
					cell.setAttribute('aria-description', LEAVE_HINT);
			for (const control of root.querySelectorAll<HTMLElement>('[tabindex]'))
				if (Number(control.getAttribute('tabindex')) > 0) control.setAttribute('tabindex', '0');
		};
		const observer = new MutationObserver(() => {
			frame ||= requestAnimationFrame(repair);
		});
		observer.observe(root, {
			subtree: true,
			childList: true,
			attributes: true,
			attributeFilter: ['tabindex']
		});
		repair();
		return () => {
			observer.disconnect();
			cancelAnimationFrame(frame);
		};
	});
	/* Sync trouble and what the editor had to say are toasts, never banners that push the grid
	 * down under the cursor. */
	toastWhile(() =>
		editorState.state === 'error'
			? {
					text: 'The sheet stopped syncing',
					tone: 'failure',
					action: { label: 'Reload sheet', run: () => retry++ }
				}
			: editorState.state === 'offline'
				? {
						text: 'Reconnecting',
						tone: 'failure',
						dedupeKey: 'sheet-reconnecting'
					}
				: notice
					? {
							text: [notice.title + '.', notice.text, notice.details?.replace(/\n/g, ' ')]
								.filter(Boolean)
								.join(' '),
							tone: 'failure'
						}
					: null
	);
</script>

<div class="sheet-editor-shell">
	<div class="native-sheet-surface" bind:this={host} aria-label="Spreadsheet editor"></div>
</div>

<style>
	.sheet-editor-shell {
		display: flex;
		flex-direction: column;
		min-height: 0;
		height: 100%;
		width: 100%;
	}
	.native-sheet-surface {
		height: 100%;
		/* A phone held sideways is shorter than 320px of grid; the sheet takes what there is. */
		min-height: min(320px, 60dvh);
		flex: 1;
		background: white;
		color: #252525;
	}
	/* The engine draws its grid, headers and controls in fixed light colours. In the dark theme
	 * the whole editor is turned dark by inverting lightness while keeping hue, so a fill or a
	 * coloured figure keeps its colour family. Pictures, charts, colour swatches and the comment
	 * marks are turned back by the exact inverse, so they show their true colours. */
	:global(:root[data-theme='dark']) .native-sheet-surface {
		background: #131313;
		color-scheme: light;
	}
	:global(:root[data-theme='dark']) .native-sheet-surface :global(.sheet-owl-root) {
		filter: invert(1) hue-rotate(180deg) contrast(0.85);
	}
	:global(:root[data-theme='dark'])
		.native-sheet-surface
		:global(:is(.o-figure, .o-color-picker, .sheet-comment-marks)) {
		filter: contrast(1.1765) hue-rotate(180deg) invert(1);
	}
</style>
