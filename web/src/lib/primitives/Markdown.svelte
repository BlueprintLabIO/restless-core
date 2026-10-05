<script lang="ts" module>
	import type { Component } from 'svelte';

	/* The code highlighter is most of Markdown's weight and most messages have
	 * no code fence, so it loads the first time one does. Until then a fence
	 * renders as Streamdown's plain block. */
	type CodeComponent = Component<any>;
	let loadedCode: CodeComponent | null = null;
	let codeLoading: Promise<CodeComponent> | null = null;
	function loadCode(): Promise<CodeComponent> {
		codeLoading ??= import('svelte-streamdown/code').then(
			(module) => (loadedCode = module.default as CodeComponent)
		);
		return codeLoading;
	}
	const FENCE = /(^|\n)[ \t]{0,3}(```|~~~)/;
</script>

<script lang="ts">
	import { Streamdown } from 'svelte-streamdown';
	import FileText from '@lucide/svelte/icons/file-text';
	import Sheet from '@lucide/svelte/icons/sheet';
	import Workflow from '@lucide/svelte/icons/workflow';
	import UserRound from '@lucide/svelte/icons/user-round';
	import File from '@lucide/svelte/icons/file';
	import Target from '@lucide/svelte/icons/target';
	import { referencePreview } from '$lib/model/reference-preview';

	let {
		text,
		streaming = false,
		onreference
	}: {
		text: string;
		streaming?: boolean;
		/** Opens a company file an agent linked to; without it such links stay plain. */
		onreference?: (path: string, label: string) => void;
	} = $props();
	let Code = $state<CodeComponent | null>(loadedCode);
	$effect(() => {
		if (Code || !FENCE.test(text ?? '')) return;
		void loadCode().then((component) => (Code = component));
	});
	const origin = typeof window === 'undefined' ? undefined : window.location.origin;
	const preview = referencePreview();
	// No raw HTML or model-supplied components: Markdown becomes Svelte nodes,
	// with Streamdown's URL checks at the link/image boundary.
	const theme = {
		code: { header: 'md-code-header', buttons: 'md-code-buttons', line: 'md-code-line' },
		components: { button: 'md-code-button' }
	};

	/* What a link points at decides how it reads: a company file or a place in
	 * the cockpit is a chip naming the thing; anything else stays a link. */
	type Reference = {
		kind: 'file' | 'doc' | 'sheet' | 'work' | 'goal' | 'person';
		path?: string;
	};
	function referenceOf(href: string | undefined): Reference | null {
		if (!href) return null;
		let url: URL;
		try {
			url = new URL(href, origin ?? 'http://local');
		} catch {
			return null;
		}
		if (origin && url.origin !== origin) return null;
		if (url.pathname.startsWith('/company/'))
			return { kind: 'file', path: decodeURIComponent(url.pathname).replace(/:\d+$/, '') };
		if (/\/library\/documents$/.test(url.pathname)) return { kind: 'doc' };
		if (/\/library\/sheets$/.test(url.pathname)) return { kind: 'sheet' };
		if (/\/work\/[^/]+$/.test(url.pathname)) return { kind: 'work' };
		if (/\/work$/.test(url.pathname) && url.searchParams.has('goal')) return { kind: 'goal' };
		if (/\/people$/.test(url.pathname) && url.searchParams.has('person')) return { kind: 'person' };
		return null;
	}
	const ICONS = {
		file: FileText,
		doc: FileText,
		sheet: Sheet,
		work: Workflow,
		goal: Target,
		person: UserRound
	};

	/* A quote that opens with one of these words is a callout the owner can
	 * spot without reading the paragraph around it. */
	const CALLOUTS: [RegExp, string][] = [
		[/^\s*(\*\*)?(needs you|decision needed|your call)\b/i, 'needs'],
		[/^\s*(\*\*)?(blocked|blocker)\b/i, 'blocked'],
		[/^\s*(\*\*)?(done|shipped|ready)\b/i, 'done'],
		[/^\s*(\*\*)?(risk|warning|caution)\b/i, 'risk']
	];
	function calloutOf(raw: string): string {
		const body = raw.replace(/^\s*>\s?/gm, '');
		return CALLOUTS.find(([pattern]) => pattern.test(body))?.[1] ?? '';
	}
</script>

{#snippet link({
	href,
	children,
	token
}: {
	href?: string;
	children: import('svelte').Snippet;
	token: { href: string; title?: string | null };
})}
	{@const reference = referenceOf(token.href ?? href)}
	{#if reference?.kind === 'file' && reference.path && onreference}
		{@const Icon = File}
		<button
			type="button"
			class="md-ref file"
			title={`Open ${reference.path}`}
			onclick={() =>
				onreference(reference.path!, reference.path!.split('/').pop() ?? reference.path!)}
			><Icon size={13} strokeWidth={1.9} aria-hidden="true" />{@render children()}</button
		>
	{:else if reference && reference.kind !== 'file'}
		{@const Icon = ICONS[reference.kind]}
		{@const card = preview?.(token.href) ?? null}
		<span class="md-ref-wrap"
			><a
				class="md-ref {reference.kind}"
				href={token.href}
				title={card ? undefined : (token.title ?? undefined)}
				><Icon size={13} strokeWidth={1.9} aria-hidden="true" />{@render children()}</a
			>{#if card}<span class="md-ref-card" role="tooltip"
					><strong>{card.title}</strong>{#each card.lines as line (line)}<span>{line}</span
						>{/each}</span
				>{/if}</span
		>
	{:else}
		<a
			href={href ?? token.href}
			title={token.title ?? undefined}
			target={href && !href.startsWith('/') ? '_blank' : undefined}
			rel="noopener noreferrer">{@render children()}</a
		>
	{/if}
{/snippet}

{#snippet blockquote({
	children,
	token
}: {
	children: import('svelte').Snippet;
	token: { raw: string };
})}
	{@const callout = calloutOf(token.raw)}
	<blockquote class:md-callout={!!callout} data-callout={callout || undefined}>
		{@render children()}
	</blockquote>
{/snippet}

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
		components={Code ? { code: Code } : {}}
		{theme}
		{link}
		{blockquote}
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
	/* Hovering or focusing a chip shows what it is without leaving the message. */
	.markdown :global(.md-ref-wrap) {
		position: relative;
		display: inline;
	}
	.markdown :global(.md-ref-card) {
		position: absolute;
		z-index: var(--z-raised, 5);
		left: 0;
		bottom: calc(100% + 6px);
		display: none;
		width: max-content;
		max-width: 260px;
		padding: 8px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		box-shadow: var(--shadow-lift);
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-weight: 400;
		line-height: 1.45;
		white-space: normal;
		pointer-events: none;
	}
	.markdown :global(.md-ref-card strong) {
		display: block;
		color: var(--ink);
		font-weight: 600;
	}
	.markdown :global(.md-ref-card span) {
		display: block;
	}
	@media (hover: hover) {
		.markdown :global(.md-ref-wrap:hover .md-ref-card) {
			display: block;
		}
	}
	.markdown :global(.md-ref-wrap:focus-within .md-ref-card) {
		display: block;
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
		font-weight: 600;
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
	/* A reference reads as the thing it names, not as underlined prose. */
	.markdown :global(.md-ref) {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		max-width: 100%;
		margin: 0 1px;
		padding: 0 6px;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
		line-height: 1.55;
		text-decoration: none;
		vertical-align: baseline;
		cursor: pointer;
		transition:
			border-color var(--motion-state) var(--ease-standard),
			background-color var(--motion-state) var(--ease-standard);
	}
	.markdown :global(.md-ref:hover) {
		border-color: var(--border-strong);
		background: var(--surface-hover);
	}
	.markdown :global(.md-ref svg) {
		flex: none;
		color: var(--text-tertiary);
	}
	.markdown :global(blockquote) {
		margin: 0 0 0.6em;
		padding-left: 0.8em;
		border-left: 2px solid currentColor;
		opacity: 0.85;
	}
	.markdown :global(.md-callout) {
		padding: 8px 10px;
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--surface-raised);
		color: var(--ink);
		opacity: 1;
	}
	.markdown :global(.md-callout > :last-child) {
		margin-bottom: 0;
	}
	.markdown :global(.md-callout[data-callout='needs']) {
		border-color: color-mix(in srgb, var(--surface-attention) 40%, var(--border));
		background: color-mix(in srgb, var(--surface-attention) 8%, var(--surface-raised));
	}
	.markdown :global(.md-callout[data-callout='blocked']) {
		border-color: color-mix(in srgb, var(--state-danger) 36%, var(--border));
		background: color-mix(in srgb, var(--state-danger) 6%, var(--surface-raised));
	}
	.markdown :global(.md-callout[data-callout='done']) {
		border-color: color-mix(in srgb, var(--state-success) 36%, var(--border));
		background: color-mix(in srgb, var(--state-success) 6%, var(--surface-raised));
	}
	.markdown :global(.md-callout[data-callout='risk']) {
		border-color: color-mix(in srgb, var(--state-danger) 24%, var(--border));
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
