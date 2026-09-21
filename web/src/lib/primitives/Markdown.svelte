<script lang="ts">
	import { Streamdown } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';

	let { text, streaming = false }: { text: string; streaming?: boolean } = $props();
	const origin = typeof window === 'undefined' ? undefined : window.location.origin;
	// No raw HTML or model-supplied components: Markdown becomes Svelte nodes,
	// with Streamdown's URL checks at the link/image boundary.
	const theme = {
		code: { header: 'md-code-header', buttons: 'md-code-buttons', line: 'md-code-line' },
		components: { button: 'md-code-button' }
	};
</script>

<div class="markdown">
	<Streamdown
		class="md"
		content={text ?? ''}
		static={!streaming}
		parseIncompleteMarkdown={streaming}
		renderHtml={false}
		defaultOrigin={origin}
		allowedImagePrefixes={['http://', 'https://']}
		animation={{ enabled: false }}
		controls={{ code: { copy: true, download: false }, table: false, mermaid: false }}
		components={{ code: Code }}
		{theme}
	/>
</div>

<style>
	.markdown :global([data-streamdown-code]),
	.markdown :global([data-streamdown-table]) {
		min-width: 0;
		max-width: 100%;
	}
	.markdown :global(.md-code-header) {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.markdown :global(.md-code-buttons) {
		display: flex;
	}
	.markdown :global(.md-code-button) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		padding: 5px;
		border: 0;
		border-radius: var(--radius-control);
		color: inherit;
		background: transparent;
		cursor: pointer;
	}
	.markdown :global(.md-code-button:hover) {
		background: var(--surface-alt);
	}
	.markdown :global(.md-code-button:focus-visible) {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.markdown :global(.md-code-button svg) {
		width: 14px;
		height: 14px;
	}
	.markdown :global(.md-code-line) {
		display: block;
	}

	.markdown :global(img),
	.markdown :global(video),
	.markdown :global(svg) {
		max-width: 100%;
		height: auto;
	}
	.markdown {
		min-width: 0;
		overflow-wrap: anywhere;
	}
	.markdown :global(.md > *:first-child) {
		margin-top: 0;
	}
	.markdown :global(.md > *:last-child) {
		margin-bottom: 0;
	}
	.markdown :global(p) {
		margin: 0 0 0.6em;
	}
	.markdown :global(h1),
	.markdown :global(h2),
	.markdown :global(h3),
	.markdown :global(h4),
	.markdown :global(h5),
	.markdown :global(h6) {
		margin: 1em 0 0.4em;
		font-weight: 650;
		line-height: 1.3;
	}
	.markdown :global(h1),
	.markdown :global(h2) {
		font-size: 1.08em;
	}
	.markdown :global(h3),
	.markdown :global(h4),
	.markdown :global(h5),
	.markdown :global(h6) {
		font-size: 1em;
	}
	.markdown :global(ul),
	.markdown :global(ol) {
		margin: 0 0 0.6em;
		padding-left: 1.3em;
	}
	.markdown :global(li) {
		margin: 0.15em 0;
	}
	.markdown :global(li::marker) {
		opacity: 0.55;
	}
	.markdown :global(a) {
		color: inherit;
		text-decoration: underline;
		text-underline-offset: 2px;
	}
	.markdown :global(blockquote) {
		margin: 0 0 0.6em;
		padding-left: 0.8em;
		border-left: 2px solid currentColor;
		opacity: 0.85;
	}
	.markdown :global(hr) {
		margin: 0.9em 0;
		border: 0;
		border-top: 1px solid currentColor;
		opacity: 0.2;
	}
	.markdown :global(code) {
		padding: 0.1em 0.32em;
		border-radius: 4px;
		background: color-mix(in srgb, currentColor 10%, transparent);
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.92em;
	}
	.markdown :global(pre) {
		margin: 0 0 0.6em;
		padding: 0.6em 0.75em;
		border-radius: 6px;
		background: color-mix(in srgb, currentColor 8%, transparent);
		overflow-x: auto;
	}
	.markdown :global(pre code) {
		padding: 0;
		background: transparent;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.markdown :global(table) {
		width: 100%;
		table-layout: fixed;
		border-collapse: collapse;
		margin: 0 0 0.6em;
	}
	.markdown :global(th),
	.markdown :global(td) {
		padding: 0.35em 0.5em;
		border: 1px solid color-mix(in srgb, currentColor 18%, transparent);
		text-align: left;
	}
</style>
