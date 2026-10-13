<script lang="ts">
	import type { EditorStatus, SheetMark, SheetSelection } from '$lib/model/sheet-editor';
	import type { PageFace } from '$lib/model/page-faces';
	import { Notice } from '$lib/ui/page';
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
				destroy(): void;
		  }
		| undefined = $state.raw();
	/** The live workbook model, for downloading it; undefined until the sheet has opened. */
	export function getModel(): any {
		return editor?.getModel();
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
</script>

<div class="sheet-editor-shell">
	{#if editorState.state === 'error'}
		<div class="sheet-notice">
			<Notice tone="danger" title="The sheet stopped syncing" details={editorState.message}>
				{#snippet actions()}<button class="btn small" type="button" onclick={() => retry++}
						>Reload sheet</button
					>{/snippet}
			</Notice>
		</div>
	{:else if editorState.state === 'offline'}
		<div class="sheet-notice">
			<Notice tone="warning" title="Reconnecting" details={editorState.message} />
		</div>
	{:else if notice}
		<div class="sheet-notice">
			<Notice tone="warning" title={notice.title} details={notice.details}>
				{notice.text}
				{#snippet actions()}<button
						class="btn small"
						type="button"
						onclick={() => (notice = undefined)}>Dismiss</button
					>{/snippet}
			</Notice>
		</div>
	{/if}
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
		min-height: 320px;
		flex: 1;
		background: white;
		color: #252525;
	}
	.sheet-notice {
		padding: 10px 12px;
	}
</style>
