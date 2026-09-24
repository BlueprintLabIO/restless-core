<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { getDocuments, type DocumentListCursor } from '$lib/model/documents';

	// Compatibility for the Aris Runtime knowledge link shared before the
	// document was available in the owner cockpit. Runtime files are not served
	// by the SPA; this maps the filename to its imported, company-scoped copy.
	let destination = $state('/aris/work/documents');
	let status = $state('Finding the matching Aris document…');

	function slug(value: string): string {
		return value
			.toLocaleLowerCase()
			.replace(/\.md$/i, '')
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-|-$/g, '');
	}

	async function resolveLegacyFile(): Promise<void> {
		const basename = (page.params.filePath ?? '').split('/').pop() ?? '';
		const wanted = slug(basename);
		// Legacy root links do not include a company handle. The shared Aris
		// knowledge path names its company in the filename.
		if (!wanted.startsWith('aris-')) {
			status = 'This legacy file link has no matching in-app document.';
			return;
		}
		let cursor: DocumentListCursor | null = null;
		try {
			do {
				const result = await getDocuments('aris', cursor, 30, true);
				const match = result.items.find((item) => slug(item.document.title) === wanted);
				if (match) {
					destination = `/aris/work/documents?document=${encodeURIComponent(match.document.id)}`;
					status = 'Opening the matching Aris document…';
					void goto(destination, { replaceState: true });
					return;
				}
				cursor = result.next_cursor;
			} while (cursor);
			status = 'No matching native document exists. Open Aris Documents to review the available files.';
		} catch {
			status = 'Aris Documents could not be reached. Try opening the Documents workspace directly.';
		}
	}

	onMount(() => void resolveLegacyFile());
</script>

<svelte:head><title>Opening Aris sales leads — Restless</title></svelte:head>

<main aria-live="polite">
	<p>{status}</p>
	<a href={destination}>Open Aris Documents</a>
</main>
