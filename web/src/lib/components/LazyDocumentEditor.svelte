<script lang="ts">
	/* The collaborative editor (TipTap, ProseMirror, Yjs) is the heaviest code in
	 * the cockpit. Surfaces that only sometimes show a document load it when one
	 * opens, so Attention and People do not pay for it on every visit. */
	import type { ComponentProps } from 'svelte';
	import type DocumentEditor from './DocumentEditor.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';

	let props: ComponentProps<typeof DocumentEditor> = $props();
	const editor = import('./DocumentEditor.svelte');
</script>

{#await editor}
	<Skeleton label="Opening document" variant="page" count={6} />
{:then { default: Editor }}
	<Editor {...props} />
{:catch}
	<p class="editor-failed" role="alert">This document could not open. Reload to try again.</p>
{/await}

<style>
	.editor-failed {
		margin: 0;
		padding: var(--space-4);
		color: var(--text-secondary);
	}
</style>
