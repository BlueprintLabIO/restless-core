<script lang="ts">
	/* A Word document or Excel workbook, read in the cockpit (G17). Browsers render neither, so the
	 * bytes come to the cockpit and are rendered here: Word as sanitized HTML, Excel read-only in the
	 * same spreadsheet engine as native Sheets. Download stays beside it for the original. */
	import { untrack } from 'svelte';
	import Pending from '$lib/ui/feedback/Pending.svelte';
	import { failureSentence } from '$lib/model/failure';

	let {
		format,
		name,
		source,
		load,
		downloadHref = ''
	}: {
		format: 'docx' | 'xlsx';
		name: string;
		/** Which file this is (its path or URL); a different one reads again. */
		source: string;
		/** The file's bytes. */
		load: () => Promise<ArrayBuffer>;
		downloadHref?: string;
	} = $props();

	let html = $state('');
	let failure = $state('');
	let loading = $state(true);
	let workbook = $state<HTMLDivElement | null>(null);
	let attempt = $state(0);

	$effect(() => {
		const kind = format;
		void source;
		const read = untrack(() => load);
		void attempt;
		const host = workbook;
		if (kind === 'xlsx' && !host) return;
		let disposed = false;
		let teardown: { destroy(): void } | undefined;
		html = '';
		failure = '';
		loading = true;
		void (async () => {
			try {
				const [bytes, office] = await Promise.all([read(), import('$lib/model/office-preview')]);
				if (disposed) return;
				if (kind === 'docx') {
					const rendered = await office.docxHtml(bytes);
					if (!disposed) html = rendered.html || '<p>This document has no text.</p>';
				} else if (host) {
					const mounted = await office.mountWorkbook(host, bytes);
					if (disposed) mounted.destroy();
					else teardown = mounted;
				}
			} catch (cause) {
				if (!disposed)
					failure = failureSentence(
						cause,
						kind === 'docx'
							? 'This Word document could not be read here.'
							: 'This workbook could not be read here.'
					);
			} finally {
				if (!disposed) loading = false;
			}
		})();
		return () => {
			disposed = true;
			teardown?.destroy();
		};
	});
</script>

<div class="office-preview">
	{#if failure}
		<div class="office-center">
			<p class="office-failure">{failure}</p>
			<div class="office-actions">
				<button class="btn small" type="button" onclick={() => (attempt += 1)}>Try again</button>
				{#if downloadHref}<a class="btn small primary" href={downloadHref} download>Download {name}</a>{/if}
			</div>
		</div>
	{:else if loading && format === 'docx'}
		<div class="office-center"><Pending label="Reading the document" /></div>
	{/if}
	{#if format === 'docx' && html && !failure}
		<div class="office-scroll">
			<!-- Sanitized in office-preview.ts: no script, style, frame or network reference survives. -->
			<article class="office-paper">{@html html}</article>
		</div>
	{:else if format === 'xlsx'}
		{#if loading && !failure}<div class="office-center overlay">
				<Pending label="Opening the workbook" />
			</div>{/if}
		<div
			class="office-workbook"
			class:hidden={!!failure}
			bind:this={workbook}
			aria-label={`${name}, read only`}
		></div>
	{/if}
</div>

<style>
	.office-preview {
		position: relative;
		display: grid;
		height: 100%;
		min-height: 0;
	}
	.office-center {
		display: grid;
		place-content: center;
		justify-items: center;
		gap: 12px;
		height: 100%;
		padding: 24px;
		color: var(--text-secondary);
		text-align: center;
	}
	.office-center.overlay {
		position: absolute;
		inset: 0;
		z-index: 1;
		background: var(--surface-alt);
	}
	.office-failure {
		max-width: 420px;
		margin: 0;
		color: var(--ink);
		font-weight: 500;
	}
	.office-actions {
		display: flex;
		gap: 8px;
	}
	.office-scroll {
		min-height: 0;
		overflow: auto;
		padding: 24px 16px 56px;
	}
	.office-paper {
		max-width: 76ch;
		margin: 0 auto;
		padding: 40px 48px;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-pane, var(--surface));
		color: var(--ink);
		line-height: 1.6;
		overflow-wrap: anywhere;
	}
	.office-paper :global(:is(h1, h2, h3, h4)) {
		margin: 1.2em 0 0.45em;
		font-weight: 600;
		line-height: 1.3;
	}
	.office-paper :global(h1) {
		font-size: 1.35em;
	}
	.office-paper :global(h2) {
		font-size: 1.15em;
	}
	.office-paper :global(:is(h3, h4)) {
		font-size: 1em;
	}
	.office-paper :global(> :first-child) {
		margin-top: 0;
	}
	.office-paper :global(p) {
		margin: 0 0 0.75em;
	}
	.office-paper :global(:is(ul, ol)) {
		margin: 0 0 0.75em;
		padding-left: 1.4em;
	}
	.office-paper :global(img) {
		max-width: 100%;
		height: auto;
	}
	.office-paper :global(table) {
		width: 100%;
		margin: 0 0 1em;
		border-collapse: collapse;
	}
	.office-paper :global(:is(th, td)) {
		padding: 6px 8px;
		border: 1px solid var(--border-strong);
		text-align: left;
		vertical-align: top;
	}
	.office-paper :global(a) {
		color: inherit;
		text-decoration: underline;
		text-underline-offset: 2px;
	}
	.office-workbook {
		height: 100%;
		min-height: 320px;
		background: white;
		color: #252525;
	}
	.office-workbook.hidden {
		display: none;
	}
	@media (max-width: 760px) {
		.office-scroll {
			padding: 0;
		}
		.office-paper {
			padding: 20px 18px;
			border: 0;
			border-radius: 0;
		}
	}
</style>
