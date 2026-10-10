<script lang="ts">
	/* A company file an agent pointed to, opened in place. The daemon serves it
	 * only because that exact message names it; a small text file renders here,
	 * a Word or Excel file is read here, and other presentable files open on the
	 * isolated review origin. Every file can be downloaded or opened in the
	 * Library. */
	import X from '@lucide/svelte/icons/x';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import Download from '@lucide/svelte/icons/download';
	import LibraryIcon from '@lucide/svelte/icons/library';
	import TextPreview from './TextPreview.svelte';
	import OfficePreview from '$lib/components/OfficePreview.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { textFormat, type TextFormat } from '$lib/model/file-text';
	import {
		downloadHref,
		downloads,
		libraryFileBytes,
		libraryFileHref,
		officeFormat
	} from '$lib/model/library-files';

	let { companyId }: { companyId: string } = $props();
	let dialog: HTMLDialogElement;
	let label = $state('');
	let path = $state('');
	let text = $state<{ body: string; format: TextFormat } | null>(null);
	let frame = $state<string | null>(null);
	let failure = $state('');
	let loading = $state(false);

	const isPdf = $derived(/\.pdf$/i.test(path));
	const office = $derived(officeFormat(path));
	const isPowerPoint = $derived(/\.pptx?$/i.test(path));
	/* Shown by the browser itself, so it may be framed or opened in a tab. */
	const viewable = $derived(!downloads({ uri: path }));
	const savedHref = $derived(frame ? downloadHref(frame) : '');

	export async function open(messageId: string, filePath: string, name: string) {
		label = name;
		path = filePath;
		text = null;
		frame = null;
		failure = '';
		loading = true;
		dialog.showModal();
		try {
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/messages/${encodeURIComponent(messageId)}/reference?path=${encodeURIComponent(filePath)}`
			);
			const body = await response.json();
			if (!response.ok)
				throw Object.assign(new Error(body.message ?? 'The file could not be opened.'), {
					status: response.status
				});
			frame = typeof body.url === 'string' ? body.url : null;
			if (body.kind === 'text')
				text = {
					body: body.markdown,
					format: (body.format as TextFormat | undefined) ?? textFormat(filePath) ?? 'markdown'
				};
		} catch (cause) {
			failure = failureSentence(cause, 'The file could not be opened.');
		} finally {
			loading = false;
		}
	}
</script>

<dialog bind:this={dialog} aria-label={label} class="reference-preview">
	<header>
		<strong title={path}>{label}</strong>
		{#if savedHref}<a class="icon" href={savedHref} title={`Download ${label}`} aria-label="Download"
				><Download size={15} aria-hidden="true" /></a
			>{/if}
		{#if frame && viewable}<a
				class="icon"
				href={frame}
				target="_blank"
				rel="noreferrer"
				title="Open in a new tab"
				aria-label="Open in a new tab"><ExternalLink size={15} aria-hidden="true" /></a
			>{/if}
		{#if path}<a
				class="icon"
				href={libraryFileHref(companyId, { uri: path })}
				title="Open in the Library"
				aria-label="Open in the Library"
				onclick={() => dialog.close()}><LibraryIcon size={15} aria-hidden="true" /></a
			>{/if}
		<button
			class="icon"
			type="button"
			aria-label="Close"
			title="Close"
			onclick={() => dialog.close()}><X size={16} strokeWidth={2} /></button
		>
	</header>
	<div class="body" class:framed={!!frame && !text}>
		{#if loading}<p class="quiet">Opening…</p>
		{:else if failure}<p class="failure" role="alert">{failure}</p>
		{:else if text}<TextPreview text={text.body} format={text.format} />
		{:else if frame && office}
			<OfficePreview
				format={office}
				name={label}
				source={path}
				load={() => libraryFileBytes(companyId, path)}
				downloadHref={savedHref}
			/>
		{:else if frame && !viewable}
			<div class="download">
				<p>
					{isPowerPoint
						? 'Browsers cannot show PowerPoint files. Download it to present it in PowerPoint or Keynote.'
						: 'Browsers cannot show this kind of file. Download it to open it on your computer.'}
				</p>
				<a class="btn small primary" href={savedHref}
					><Download size={14} strokeWidth={1.9} aria-hidden="true" />Download {label}</a
				>
			</div>
		{:else if frame && isPdf}
			<!-- The browser's PDF reader will not run in a sandboxed frame. The file is already on its
			     own read-only origin, apart from the cockpit, and a PDF runs no page script. -->
			<iframe src={`${frame.split('#')[0]}#view=FitH`} title={label} referrerpolicy="no-referrer"
			></iframe>
		{:else if frame}<iframe
				src={frame}
				title={label}
				sandbox="allow-scripts allow-same-origin allow-downloads"
				referrerpolicy="no-referrer"
			></iframe>{/if}
	</div>
</dialog>

<style>
	.reference-preview {
		width: min(860px, calc(100vw - 32px));
		height: min(80vh, 900px);
		padding: 0;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		color: var(--ink);
		box-shadow: var(--shadow-soft);
	}
	.reference-preview[open] {
		display: flex;
		flex-direction: column;
	}
	.reference-preview::backdrop {
		background: color-mix(in srgb, var(--ink) 24%, transparent);
	}
	header {
		display: flex;
		align-items: center;
		gap: 6px;
		min-height: 48px;
		padding: 6px 8px 6px 16px;
		border-bottom: 1px solid var(--border);
	}
	strong {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		font-size: var(--t-body);
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.icon {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.icon:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.body {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}
	.body.framed {
		overflow: hidden;
	}
	iframe {
		width: 100%;
		height: 100%;
		border: 0;
		background: white;
	}
	.download {
		display: grid;
		place-content: center;
		justify-items: center;
		gap: 12px;
		height: 100%;
		padding: 24px;
		color: var(--text-secondary);
		text-align: center;
	}
	.download p {
		max-width: 420px;
		margin: 0;
	}
	.download .btn {
		gap: 6px;
		text-decoration: none;
	}
	.quiet,
	.failure {
		margin: 24px;
		color: var(--text-tertiary);
	}
	.failure {
		color: var(--state-danger);
	}
</style>
