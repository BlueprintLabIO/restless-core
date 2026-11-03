<script lang="ts">
	/* One picture in an agent's Markdown: a message, an Inbox item, a document. It always takes the
	 * same fixed frame, loading, shown or missing, so nothing around it moves (no jank). A picture
	 * the owner's view cannot open says why in that frame and offers the Library, never a blank box. */
	import ImageOff from '@lucide/svelte/icons/image-off';
	import type { CompanyImage } from '$lib/model/company-images';

	let {
		image,
		alt,
		companyRoot,
		onreference
	}: {
		image: CompanyImage;
		alt: string;
		/** `/<company>`, for the Library link; empty outside a company. */
		companyRoot: string;
		onreference?: (path: string, label: string) => void;
	} = $props();

	/* The address that failed to load; a new address (a redrawn message) tries again. */
	let failedSrc = $state('');
	const src = $derived(image.kind === 'unavailable' ? '' : image.src);
	const failed = $derived(!!src && failedSrc === src);
	const path = $derived(image.kind === 'web' ? undefined : image.path);
	const name = $derived(path?.split('/').pop() ?? '');
	const label = $derived(alt.trim() || name || 'Picture');
	const libraryHref = $derived(
		path?.startsWith('/company/') && companyRoot
			? `${companyRoot}/library?path=${encodeURIComponent(path)}`
			: undefined
	);
	const reason = $derived(
		image.kind === 'unavailable'
			? image.reason
			: image.kind === 'web'
				? 'This picture could not be loaded from its website.'
				: 'This picture could not be read from the company computer.'
	);
	function open(event: MouseEvent) {
		if (!onreference || !path || event.metaKey || event.ctrlKey || event.shiftKey) return;
		event.preventDefault();
		onreference(path, name || label);
	}
</script>

<span
	class="md-figure"
	data-image-state={image.kind === 'unavailable' || failed ? 'missing' : 'shown'}
>
	{#if image.kind !== 'unavailable' && !failed}
		{#if libraryHref}
			<a class="md-figure-frame" href={libraryHref} title={label} onclick={open}
				><img
					{src}
					alt={label}
					loading="lazy"
					decoding="async"
					onerror={() => (failedSrc = src)}
				/></a
			>
		{:else}
			<span class="md-figure-frame" title={label}
				><img
					{src}
					alt={label}
					loading="lazy"
					decoding="async"
					referrerpolicy="no-referrer"
					onerror={() => (failedSrc = src)}
				/></span
			>
		{/if}
	{:else}
		<span class="md-figure-frame missing" role="img" aria-label={`${label}. ${reason}`}>
			<ImageOff size={18} strokeWidth={1.8} aria-hidden="true" />
			<span class="md-figure-name" title={path ?? label}>{label}</span>
			<span class="md-figure-why">{reason}</span>
			{#if libraryHref}<a class="md-figure-open" href={libraryHref} onclick={open}
					>Open in Library</a
				>{/if}
		</span>
	{/if}
</span>

<style>
	/* A fixed frame, the same in every state: the picture fits inside it rather than sizing it. */
	.md-figure {
		display: block;
		width: 100%;
		max-width: 640px;
		margin: 0.4em 0 0.7em;
	}
	.md-figure-frame {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: 100%;
		aspect-ratio: 16 / 10;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--surface-raised);
		text-decoration: none;
	}
	a.md-figure-frame:hover {
		border-color: var(--border-strong);
	}
	/* Laid out by the frame, never by its own size: a square picture fits inside, uncropped. Two
	 * classes deep so Markdown's own `img { height: auto }` cannot win on stylesheet order. */
	.md-figure .md-figure-frame img {
		position: absolute;
		inset: 0;
		display: block;
		width: 100%;
		height: 100%;
		max-width: none;
		object-fit: contain;
	}
	.missing {
		flex-direction: column;
		gap: 4px;
		padding: 12px 16px;
		color: var(--text-tertiary);
		text-align: center;
		font-size: var(--t-label);
		line-height: 1.4;
	}
	.md-figure-name {
		max-width: 100%;
		overflow: hidden;
		color: var(--text-secondary);
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.md-figure-why {
		max-width: 36ch;
	}
	.md-figure-open {
		margin-top: 2px;
		color: var(--ink);
	}
</style>
