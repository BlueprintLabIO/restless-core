<script lang="ts">
	/* Renders a message's attached files: raster images inline (an uploads endpoint
	 * is expected to serve those inline), everything else as a download card. */

	import type { MessageAttachment } from '$lib/model/view';

	let {
		attachments,
		hrefFor
	}: {
		attachments: MessageAttachment[];
		/** Where this company-scoped attachment's bytes live. */
		hrefFor?: (attachment: MessageAttachment) => string;
	} = $props();

	const INLINE_IMAGE_TYPES = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp']);

	function hrefOf(attachment: MessageAttachment): string {
		return hrefFor ? hrefFor(attachment) : '';
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
</script>

{#if attachments.length > 0}
	<div class="attach-list">
		{#each attachments as attachment (attachment.uploadId)}
			{#if INLINE_IMAGE_TYPES.has(attachment.mediaType)}
				<a
					class="attach-img"
					href={hrefOf(attachment)}
					target="_blank"
					rel="noopener"
					title="{attachment.name} · {fmtSize(attachment.sizeBytes)}"
				>
					<img src={hrefOf(attachment)} alt={attachment.name} loading="lazy" />
				</a>
			{:else}
				<a class="attach-file" href={hrefOf(attachment)}>
					<span class="a-kind">{extOf(attachment.name)}</span>
					<span class="af-main">
						<span class="a-name">{attachment.name}</span>
						<span class="a-meta">{attachment.mediaType} · {fmtSize(attachment.sizeBytes)}</span>
					</span>
				</a>
			{/if}
		{/each}
	</div>
{/if}
