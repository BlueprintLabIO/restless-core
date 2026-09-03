<script lang="ts">
	import { Marked, Renderer, type Tokens } from 'marked';

	let { text }: { text: string } = $props();

	function escapeHtml(value: string): string {
		return value
			.replaceAll('&', '&amp;')
			.replaceAll('<', '&lt;')
			.replaceAll('>', '&gt;')
			.replaceAll('"', '&quot;')
			.replaceAll("'", '&#39;');
	}

	function safeHref(value: string, image = false): string | null {
		const href = value.trim();
		if (/^https?:\/\//i.test(href)) return href;
		if (!image && /^mailto:/i.test(href)) return href;
		if (/^(?:\/[^/]|\.\.?\/|#|\?)/.test(href)) return href;
		return null;
	}

	const renderer = new Renderer();
	renderer.html = ({ text }: Tokens.HTML | Tokens.Tag) => escapeHtml(text);
	renderer.link = function ({ href, title, tokens }: Tokens.Link) {
		const label = this.parser.parseInline(tokens);
		const safe = safeHref(href);
		if (!safe) return label;
		const titleAttribute = title ? ` title="${escapeHtml(title)}"` : '';
		return `<a href="${escapeHtml(safe)}"${titleAttribute}>${label}</a>`;
	};
	renderer.image = ({ href, title, text }: Tokens.Image) => {
		const safe = safeHref(href, true);
		if (!safe) return escapeHtml(text);
		const titleAttribute = title ? ` title="${escapeHtml(title)}"` : '';
		return `<img src="${escapeHtml(safe)}" alt="${escapeHtml(text)}"${titleAttribute}>`;
	};

	const markdown = new Marked({ async: false, gfm: true, breaks: true, renderer });

	/* Employee, owner and model text crosses a trust boundary. Raw HTML is
	 * escaped and every generated URL is allowlisted before Svelte receives it,
	 * so this renderer is identical in browsers and during server rendering. */
	const html = $derived.by(() => {
		return String(markdown.parse(text ?? ''));
	});
</script>

<!-- `html` is produced only by the escaped and protocol-allowlisted renderer above. -->
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
