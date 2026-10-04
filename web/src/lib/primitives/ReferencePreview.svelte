<script lang="ts">
	/* A company file an agent pointed to, opened in place. The daemon serves it
	 * only because that exact message names it; text renders here, and other
	 * presentable files open on the isolated review origin. */
	import X from '@lucide/svelte/icons/x';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import Markdown from './Markdown.svelte';
	import { failureSentence } from '$lib/model/failure';

	let { companyId }: { companyId: string } = $props();
	let dialog: HTMLDialogElement;
	let label = $state('');
	let path = $state('');
	let markdown = $state<string | null>(null);
	let frame = $state<string | null>(null);
	let failure = $state('');
	let loading = $state(false);

	export async function open(messageId: string, filePath: string, name: string) {
		label = name;
		path = filePath;
		markdown = null;
		frame = null;
		failure = '';
		loading = true;
		dialog.showModal();
		try {
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/messages/${encodeURIComponent(messageId)}/reference?path=${encodeURIComponent(filePath)}`
			);
			const body = await response.json();
			if (!response.ok)
				throw Object.assign(new Error(body.message ?? 'The file could not be opened.'), {
					status: response.status
				});
			if (body.kind === 'text') markdown = body.markdown;
			else frame = body.url;
		} catch (cause) {
			failure = failureSentence(cause, 'The file could not be opened.');
		} finally {
			loading = false;
		}
	}
</script>

<dialog bind:this={dialog} aria-label={label} class="reference-preview">
	<header>
		<strong title={path}>{label}</strong>
		{#if frame}<a
				class="icon"
				href={frame}
				target="_blank"
				rel="noreferrer"
				title="Open in a new tab"><ExternalLink size={15} aria-hidden="true" /></a
			>{/if}
		<button
			class="icon"
			type="button"
			aria-label="Close"
			title="Close"
			onclick={() => dialog.close()}><X size={16} strokeWidth={2} /></button
		>
	</header>
	<div class="body" class:framed={!!frame}>
		{#if loading}<p class="quiet">Opening…</p>
		{:else if failure}<p class="failure" role="alert">{failure}</p>
		{:else if markdown !== null}<div class="paper"><Markdown text={markdown} /></div>
		{:else if frame}<iframe src={frame} title={label} sandbox="allow-scripts allow-same-origin"
			></iframe>{/if}
	</div>
</dialog>

<style>
	.reference-preview {
		width: min(860px, calc(100vw - 32px));
		height: min(80vh, 900px);
		padding: 0;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		color: var(--ink);
		box-shadow: var(--shadow-soft);
	}
	.reference-preview[open] {
		display: flex;
		flex-direction: column;
	}
	.reference-preview::backdrop {
		background: color-mix(in srgb, var(--ink) 24%, transparent);
	}
	header {
		display: flex;
		align-items: center;
		gap: 6px;
		min-height: 48px;
		padding: 6px 8px 6px 16px;
		border-bottom: 1px solid var(--border);
	}
	strong {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		font-size: var(--t-body);
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.icon {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.icon:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.body {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}
	.body.framed {
		overflow: hidden;
	}
	.paper {
		max-width: 72ch;
		margin: 0 auto;
		padding: 24px 28px 48px;
		color: var(--text-secondary);
		line-height: 1.6;
	}
	.paper :global(:is(strong, h1, h2, h3, h4)) {
		color: var(--ink);
	}
	iframe {
		width: 100%;
		height: 100%;
		border: 0;
		background: white;
	}
	.quiet,
	.failure {
		margin: 24px;
		color: var(--text-tertiary);
	}
	.failure {
		color: var(--state-danger);
	}
</style>
