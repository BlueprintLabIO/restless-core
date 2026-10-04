<script lang="ts">
	import type { EditorStatus } from '$lib/model/sheet-editor';
	import { Notice } from '$lib/ui/page';
	let {
		company,
		sheet,
		onstatus
	}: { company: string; sheet: string; onstatus?: (value: EditorStatus) => void } = $props();
	let host: HTMLDivElement;
	let editorState = $state<EditorStatus>({ state: 'connecting', access: 'read' });
	let retry = $state(0);
	$effect(() => {
		const targetCompany = company,
			targetSheet = sheet;
		retry;
		let disposed = false,
			editor: { destroy(): void } | undefined;
		if (!host) return;
		void import('$lib/model/sheet-editor')
			.then((module) => {
				if (disposed) return;
				editor = module.mountSheet(host, targetCompany, targetSheet, (value: EditorStatus) => {
					if (!disposed) {
						editorState = value;
						onstatus?.(value);
					}
				});
			})
			.catch((error) => {
				if (!disposed) editorState = { state: 'error', access: 'read', message: error.message };
			});
		return () => {
			disposed = true;
			editor?.destroy();
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
