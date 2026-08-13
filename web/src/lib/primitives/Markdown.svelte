<script lang="ts">
	/**
	 * Renders the token tree from `$lib/primitives/markdown` — an employee's reply, formatted.
	 *
	 * Every value below goes through ordinary Svelte interpolation, never `{@html}`, so a
	 * script tag written in a reply is displayed as those characters and nothing else.
	 * `href` is already restricted to http/https/mailto by the parser; the extra `rel`
	 * here is belt-and-braces for the tab this opens.
	 */
	import type { BlockToken, InlineToken } from '$lib/primitives/markdown';
	import { parseMarkdown } from '$lib/primitives/markdown';

	let { text }: { text: string } = $props();

	const blocks = $derived<BlockToken[]>(parseMarkdown(text));
</script>

<!-- Every href here is external by construction: the parser admits only http, https and
     mailto, and refuses everything else back to literal text. resolve() is for internal
     routes and would be wrong on all of them. -->
<!-- eslint-disable svelte/no-navigation-without-resolve -->

{#snippet inline(tokens: InlineToken[])}
	{#each tokens as token, index (index)}
		{#if token.kind === 'text'}{token.value}{:else if token.kind === 'code'}<code class="md-c"
				>{token.value}</code
			>{:else if token.kind === 'strong'}<strong>{@render inline(token.children)}</strong
			>{:else if token.kind === 'emphasis'}<em>{@render inline(token.children)}</em
			>{:else if token.kind === 'link'}<a
				href={token.href}
				target="_blank"
				rel="noopener noreferrer">{@render inline(token.children)}</a
			>{:else if token.kind === 'break'}<br />{/if}
	{/each}
{/snippet}

{#snippet body(tokens: BlockToken[])}
	{#each tokens as block, index (index)}
		{#if block.kind === 'paragraph'}
			<p>{@render inline(block.children)}</p>
		{:else if block.kind === 'heading'}
			{#if block.level === 1}
				<h1>{@render inline(block.children)}</h1>
			{:else if block.level === 2}
				<h2>{@render inline(block.children)}</h2>
			{:else if block.level === 3}
				<h3>{@render inline(block.children)}</h3>
			{:else if block.level === 4}
				<h4>{@render inline(block.children)}</h4>
			{:else if block.level === 5}
				<h5>{@render inline(block.children)}</h5>
			{:else}
				<h6>{@render inline(block.children)}</h6>
			{/if}
		{:else if block.kind === 'code'}
			<pre class="md-pre"><code>{block.value}</code></pre>
		{:else if block.kind === 'list'}
			{#if block.ordered}
				<ol start={block.start}>
					{#each block.items as item, position (position)}
						<li>{@render inline(item)}</li>
					{/each}
				</ol>
			{:else}
				<ul>
					{#each block.items as item, position (position)}
						<li>{@render inline(item)}</li>
					{/each}
				</ul>
			{/if}
		{:else if block.kind === 'quote'}
			<blockquote>{@render body(block.children)}</blockquote>
		{:else if block.kind === 'rule'}
			<hr />
		{/if}
	{/each}
{/snippet}

<div class="md">{@render body(blocks)}</div>

<style>
	/* Structure carried by weight and spacing, not by size — a reply must not out-shout
	   the surface it sits on. */
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
	.md :global(code.md-c) {
		padding: 0.1em 0.32em;
		border-radius: 4px;
		background: color-mix(in srgb, currentColor 10%, transparent);
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.92em;
	}
	/* A long block scrolls inside the bubble rather than widening the pane. */
	.md :global(pre.md-pre) {
		margin: 0 0 0.6em;
		padding: 0.6em 0.75em;
		border-radius: 6px;
		background: color-mix(in srgb, currentColor 8%, transparent);
		overflow-x: auto;
	}
	.md :global(pre.md-pre code) {
		font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
		font-size: 0.9em;
		white-space: pre;
	}
</style>
