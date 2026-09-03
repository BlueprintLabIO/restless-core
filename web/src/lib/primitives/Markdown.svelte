<script lang="ts">
	import DOMPurify from 'isomorphic-dompurify';
	import { marked } from 'marked';

	let { text }: { text: string } = $props();

	/* Marked deliberately does not sanitize. Employee and owner text crosses a
	 * trust boundary, so sanitize its HTML before handing it to Svelte. Keep
	 * only ordinary web links; no javascript:, data:, inline handlers or raw
	 * model-supplied markup survives the boundary. */
	const html = $derived.by(() => {
		const rendered = String(marked.parse(text ?? '', { async: false, gfm: true, breaks: true }));
		return DOMPurify.sanitize(rendered, {
			ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto):|[^a-z]|[a-z+.-]+(?:[^a-z+.-:]|$))/i
		});
	});
</script>

<!-- `html` is sanitized above. This is intentionally the one rendering
     boundary rather than a bespoke partial Markdown parser per transcript. -->
<div class="md">{@html html}</div>

<style>
	.md {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.md :global(> *:first-child) {
		margin-top: 0;
	}
	.md :global(> *:last-child) {
		margin-bottom: 0;
	}
	.md :global(p) {
		margin: 0 0 0.6em;
	}
	.md :global(h1),
	.md :global(h2),
	.md :global(h3),
	.md :global(h4),
	.md :global(h5),
	.md :global(h6) {
		margin: 1em 0 0.4em;
		font-weight: 650;
		line-height: 1.3;
	}
	.md :global(h1),
	.md :global(h2) {
		font-size: 1.08em;
	}
	.md :global(h3),
	.md :global(h4),
	.md :global(h5),
	.md :global(h6) {
		font-size: 1em;
	}
	.md :global(ul),
	.md :global(ol) {
		margin: 0 0 0.6em;
		padding-left: 1.3em;
	}
	.md :global(li) {
		margin: 0.15em 0;
	}
	.md :global(li::marker) {
		opacity: 0.55;
	}
	.md :global(a) {
		color: inherit;
		text-decoration: underline;
		text-underline-offset: 2px;
	}
	.md :global(blockquote) {
		margin: 0 0 0.6em;
		padding-left: 0.8em;
		border-left: 2px solid currentColor;
		opacity: 0.85;
	}
	.md :global(hr) {
		margin: 0.9em 0;
		border: 0;
		border-top: 1px solid currentColor;
		opacity: 0.2;
	}
	.md :global(code) {
		padding: 0.1em 0.32em;
		border-radius: 4px;
		background: color-mix(in srgb, currentColor 10%, transparent);
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.92em;
	}
	.md :global(pre) {
		margin: 0 0 0.6em;
		padding: 0.6em 0.75em;
		border-radius: 6px;
		background: color-mix(in srgb, currentColor 8%, transparent);
		overflow-x: auto;
	}
	.md :global(pre code) {
		padding: 0;
		background: transparent;
		white-space: pre;
	}
	.md :global(table) {
		width: 100%;
		border-collapse: collapse;
		margin: 0 0 0.6em;
	}
	.md :global(th),
	.md :global(td) {
		padding: 0.35em 0.5em;
		border: 1px solid color-mix(in srgb, currentColor 18%, transparent);
		text-align: left;
	}
</style>
