<script lang="ts">
	/* Renders a message's attached files: raster images as thumbnails, everything else as a file
	 * card. Opening one shows it in place (an image whole, a PDF in the browser's reader, a Word or
	 * Excel file read by the cockpit, a note as text); anything else downloads. The gateway serves an
	 * image or PDF in place only when asked (`inline=1`) and only after checking its bytes. */
	import { readAttachmentBytes } from '$lib/model/company-actions';
	import Download from '@lucide/svelte/icons/download';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import X from '@lucide/svelte/icons/x';
	import type { MessageAttachment } from '$lib/model/view';
	import { textFormat, type TextFormat } from '$lib/model/file-text';
	import { officeFormat } from '$lib/model/library-files';
	import { failureSentence, type OwnerFailure } from '$lib/model/failure';
	import TextPreview from './TextPreview.svelte';
	import OfficePreview from '$lib/components/OfficePreview.svelte';

	let {
		attachments,
		hrefFor
	}: {
		attachments: MessageAttachment[];
		/** Where this company-scoped attachment's bytes live. */
		hrefFor?: (attachment: MessageAttachment) => string;
	} = $props();

	const INLINE_IMAGE_TYPES = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp']);
	const OFFICE_FILE_TYPES = new Map<string, string>([
		['doc', 'Word document'],
		['docx', 'Word document'],
		['xls', 'Excel workbook'],
		['xlsx', 'Excel workbook'],
		['ppt', 'PowerPoint presentation'],
		['pptx', 'PowerPoint presentation']
	]);
	/* A text attachment larger than this downloads rather than rendering in place. */
	const MAX_TEXT_PREVIEW_BYTES = 1024 * 1024;

	function hrefOf(attachment: MessageAttachment): string {
		return hrefFor ? hrefFor(attachment) : '';
	}
	function inlineHref(attachment: MessageAttachment): string {
		const href = hrefOf(attachment);
		return href ? `${href}${href.includes('?') ? '&' : '?'}inline=1` : '';
	}

	function fmtSize(bytes: number): string {
		if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
		return `${Math.max(1, Math.ceil(bytes / 1024))} KB`;
	}

	/* The kind chip is the file's extension — "file" tells you nothing. */
	function extOf(name: string): string {
		const match = /\.([a-z0-9]{1,8})$/i.exec(name);
		return match ? match[1].toLowerCase() : 'file';
	}

	function fileDescription(attachment: MessageAttachment): string {
		/* A kind a person recognises, not a media type: "PDF", "CSV file", "Word document". */
		const office = OFFICE_FILE_TYPES.get(extOf(attachment.name));
		if (office) return office;
		if (attachment.mediaType === 'application/pdf') return 'PDF';
		if (attachment.mediaType.startsWith('image/')) return 'Image';
		if (attachment.mediaType.startsWith('video/')) return 'Video';
		if (attachment.mediaType.startsWith('audio/')) return 'Audio';
		const ext = extOf(attachment.name);
		return ext !== 'file' ? `${ext.toUpperCase()} file` : 'File';
	}

	type Preview = 'image' | 'pdf' | 'office' | 'text';
	function previewOf(attachment: MessageAttachment): Preview | null {
		if (INLINE_IMAGE_TYPES.has(attachment.mediaType)) return 'image';
		/* The gateway shows a PDF in place only when it was stored as one. */
		if (attachment.mediaType === 'application/pdf') return 'pdf';
		if (officeFormat(attachment.name)) return 'office';
		if (textFormat(attachment.name) && attachment.sizeBytes <= MAX_TEXT_PREVIEW_BYTES)
			return 'text';
		return null;
	}

	let dialog = $state<HTMLDialogElement | null>(null);
	let previewing = $state<MessageAttachment | null>(null);
	let text = $state<{ body: string; format: TextFormat } | null>(null);
	let failure = $state('');
	const kind = $derived(previewing ? previewOf(previewing) : null);

	const readBytes = (attachment: MessageAttachment): Promise<Response> =>
		readAttachmentBytes(hrefOf(attachment) ?? '');

	/* An image that will not load says why, never a blank frame (S85-08): the bytes are gone, the
	 * company computer is stopped, or the read failed. The <img> carries no reason, so ask again. */
	let unshown = $state<Record<string, true>>({});
	function unreadable(cause: unknown): string {
		const failure = cause as OwnerFailure;
		if (failure?.status === 404) return 'This file is no longer available.';
		if (failure?.code === 'runtime' && failure.serverMessage) return failure.serverMessage;
		if (failure?.status && failure.status >= 500) return 'The image could not be shown. Try again.';
		return failureSentence(cause, 'The image could not be shown.');
	}
	function explain(attachment: MessageAttachment) {
		void readBytes(attachment)
			.then(() => {
				if (previewing === attachment) failure = 'The image could not be shown.';
			})
			.catch((cause) => {
				if (previewing === attachment) failure = unreadable(cause);
			});
	}

	function open(event: MouseEvent, attachment: MessageAttachment) {
		const preview = previewOf(attachment);
		if (
			!preview ||
			!hrefOf(attachment) ||
			!dialog ||
			event.button !== 0 ||
			event.metaKey ||
			event.ctrlKey ||
			event.shiftKey ||
			event.altKey
		)
			return;
		event.preventDefault();
		previewing = attachment;
		text = null;
		failure = '';
		dialog.showModal();
		if (preview !== 'text') return;
		const format = textFormat(attachment.name)!;
		void readBytes(attachment)
			.then((response) => response.text())
			.then((body) => {
				if (previewing === attachment) text = { body, format };
			})
			.catch((cause) => {
				if (previewing === attachment)
					failure = failureSentence(cause, 'The file could not be read.');
			});
	}
</script>

{#if attachments.length > 0}
	<div class="attach-list">
		{#each attachments as attachment (attachment.uploadId)}
			{#if INLINE_IMAGE_TYPES.has(attachment.mediaType)}
				<a
					class="attach-img"
					href={inlineHref(attachment)}
					target="_blank"
					rel="noopener"
					title="{attachment.name} · {fmtSize(attachment.sizeBytes)}"
					onclick={(event) => open(event, attachment)}
				>
					{#if unshown[attachment.uploadId]}<span class="attach-img-missing">Can’t show</span
						>{:else}<img
							src={inlineHref(attachment)}
							alt={attachment.name}
							loading="lazy"
							onerror={() => (unshown[attachment.uploadId] = true)}
						/>{/if}
				</a>
			{:else}
				<a
					class="attach-file"
					href={previewOf(attachment) === 'pdf' ? inlineHref(attachment) : hrefOf(attachment)}
					target={previewOf(attachment) === 'pdf' ? '_blank' : undefined}
					rel={previewOf(attachment) === 'pdf' ? 'noopener' : undefined}
					onclick={(event) => open(event, attachment)}
				>
					<span class="a-kind">{extOf(attachment.name)}</span>
					<span class="af-main">
						<span class="a-name">{attachment.name}</span>
						<span class="a-meta"
							>{fileDescription(attachment)} · {fmtSize(attachment.sizeBytes)}</span
						>
					</span>
				</a>
			{/if}
		{/each}
	</div>

	<dialog
		bind:this={dialog}
		class="attachment-preview"
		aria-label={previewing?.name ?? 'Attachment'}
		onclose={() => (previewing = null)}
	>
		{#if previewing}
			<header>
				<strong title={previewing.name}>{previewing.name}</strong>
				{#if (kind === 'image' && !failure) || kind === 'pdf'}<a
						class="icon"
						href={inlineHref(previewing)}
						target="_blank"
						rel="noopener"
						title="Open in a new tab"
						aria-label="Open in a new tab"><ExternalLink size={15} aria-hidden="true" /></a
					>{/if}
				{#if !(kind === 'image' && failure)}<a
						class="icon"
						href={hrefOf(previewing)}
						title={`Download ${previewing.name}`}
						aria-label="Download"><Download size={15} aria-hidden="true" /></a
					>{/if}
				<button
					class="icon"
					type="button"
					aria-label="Close"
					title="Close"
					onclick={() => dialog?.close()}><X size={16} strokeWidth={2} /></button
				>
			</header>
			<div class="body" class:framed={kind === 'pdf' || kind === 'office'}>
				{#if kind === 'image' && failure}
					<p class="failure" role="alert">{failure}</p>
				{:else if kind === 'image'}
					{@const shown = previewing}
					<div class="image">
						<img src={inlineHref(shown)} alt={shown.name} onerror={() => explain(shown)} />
					</div>
				{:else if kind === 'pdf'}
					<!-- The browser's PDF reader refuses a sandboxed frame; the gateway serves a PDF in place
				     only after checking it is one, and a PDF runs no page script. -->
					<iframe src={`${inlineHref(previewing)}#view=FitH`} title={previewing.name}></iframe>
				{:else if kind === 'office'}
					{@const shown = previewing}
					<OfficePreview
						format={officeFormat(shown.name) ?? 'docx'}
						name={shown.name}
						source={hrefOf(shown)}
						load={() => readBytes(shown).then((response) => response.arrayBuffer())}
						downloadHref={hrefOf(shown)}
					/>
				{:else if failure}
					<p class="failure" role="alert">{failure}</p>
				{:else if text}
					<TextPreview text={text.body} format={text.format} />
				{:else}
					<p class="quiet">Opening…</p>
				{/if}
			</div>
		{/if}
	</dialog>
{/if}

<style>
	.attachment-preview {
		width: min(920px, calc(100vw - 32px));
		height: min(84vh, 960px);
		padding: 0;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		color: var(--ink);
		box-shadow: var(--shadow-soft);
	}
	.attachment-preview[open] {
		display: flex;
		flex-direction: column;
	}
	.attachment-preview::backdrop {
		background: color-mix(in srgb, var(--ink) 24%, transparent);
	}
	.attachment-preview header {
		display: flex;
		align-items: center;
		gap: 6px;
		min-height: 48px;
		padding: 6px 8px 6px 16px;
		border-bottom: 1px solid var(--border);
	}
	.attachment-preview strong {
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
	.image {
		display: grid;
		place-items: center;
		min-height: 100%;
		padding: 20px;
		background: var(--surface-alt);
	}
	.image img {
		max-width: 100%;
		max-height: calc(84vh - 96px);
		border-radius: var(--radius-md);
		background: repeating-conic-gradient(#eef0f4 0 25%, #fff 0 50%) 0 0 / 16px 16px;
	}
	iframe {
		width: 100%;
		height: 100%;
		border: 0;
		background: white;
	}
	.attach-img-missing {
		display: grid;
		place-items: center;
		height: 100%;
		padding: 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-align: center;
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
