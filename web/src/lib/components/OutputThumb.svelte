<script lang="ts">
	/* One output of a piece of Work as a small picture (S88-04): an image as itself, a site as a
	 * scaled live preview, anything else as its kind and name. It opens in the Library's viewer. */
	import {
		fileLabel,
		LIBRARY_KIND_LABEL,
		libraryFileHref,
		libraryKind,
		openLibraryFile
	} from '$lib/model/library-files';

	let {
		companyId,
		artifact
	}: {
		companyId: string;
		artifact: { id: string; uri: string; kind: string; label: string };
	} = $props();

	const kind = $derived(libraryKind(artifact));
	const label = $derived(fileLabel(artifact.label, artifact.uri));
	let url = $state('');
	let failed = $state(false);
	$effect(() => {
		if (kind !== 'image' && kind !== 'site') return;
		let live = true;
		void openLibraryFile(companyId, { id: artifact.id, uri: artifact.uri })
			.then((opened) => {
				if (live) url = opened.url;
			})
			.catch(() => {
				if (live) failed = true;
			});
		return () => {
			live = false;
		};
	});
</script>

<a
	class="output-thumb"
	href={libraryFileHref(companyId, { id: artifact.id, uri: artifact.uri })}
	title={label}
>
	<span class="output-face" aria-hidden="true">
		{#if url && kind === 'image'}<img
				src={url}
				alt=""
				loading="lazy"
			/>{:else if url && kind === 'site'}<iframe
				src={url}
				title=""
				tabindex="-1"
				loading="lazy"
				referrerpolicy="no-referrer"
			></iframe>{:else}<span class="output-kind"
				>{kind ? LIBRARY_KIND_LABEL[kind].replace(/s$/, '') : 'File'}{failed
					? ' · unavailable'
					: ''}</span
			>{/if}
	</span>
	<span class="output-label">{label}</span>
</a>

<style>
	.output-thumb {
		display: grid;
		gap: 4px;
		min-width: 0;
		color: var(--text-secondary);
		text-decoration: none;
	}
	.output-face {
		position: relative;
		display: grid;
		place-items: center;
		aspect-ratio: 4 / 3;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-alt);
	}
	.output-thumb:hover .output-face {
		border-color: var(--border-strong);
	}
	img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	/* A live page, drawn at desktop width and scaled down; it takes no clicks. */
	iframe {
		position: absolute;
		top: 0;
		left: 0;
		width: 400%;
		height: 400%;
		border: 0;
		background: white;
		transform: scale(0.25);
		transform-origin: 0 0;
		pointer-events: none;
	}
	.output-kind {
		padding: 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-align: center;
	}
	.output-label {
		overflow: hidden;
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
