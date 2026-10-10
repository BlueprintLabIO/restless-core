<script lang="ts">
	/* One file Work produced, opened in place (S64-T6). The gateway serves it read-only on its own
	 * origin; this frames it the way its kind is meant to be seen: a site live, an image whole, a
	 * recording in a player, a PDF in the browser's reader, and a deck as a slideshow.
	 *
	 * A deck plays full screen. An HTML deck follows the deck contract Staff are taught
	 * (`presentation-deck` skill): one <section> per slide, and it answers {type:'restless:deck',
	 * action:'next'|'prev'|'first'} messages and reports {type:'restless:deck', slide, total}. A PDF
	 * deck reads in the browser's own reader, page after page; its reader ignores a changed #page
	 * once loaded, so the cockpit does not pretend to step it. */
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Play from '@lucide/svelte/icons/play';
	import Download from '@lucide/svelte/icons/download';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import X from '@lucide/svelte/icons/x';
	import {
		downloads,
		LibraryOpenError,
		openLibraryFile,
		type LibraryFile
	} from '$lib/model/library-files';
	import { failureSentence } from '$lib/model/failure';
	import Pending from '$lib/ui/feedback/Pending.svelte';

	let {
		companyId,
		file,
		owner,
		backHref
	}: { companyId: string; file: LibraryFile; owner: string; backHref: string } = $props();

	let url = $state('');
	let failure = $state('');
	let failureDetail = $state('');
	let loading = $state(true);
	let attempt = $state(0);
	$effect(() => {
		const target = file.id;
		void attempt;
		url = '';
		failure = '';
		failureDetail = '';
		loading = true;
		slide = 1;
		total = 0;
		void openLibraryFile(companyId, target)
			.then((next) => {
				if (target === file.id) url = next;
			})
			.catch((cause) => {
				if (target !== file.id) return;
				if (cause instanceof LibraryOpenError) {
					failure = cause.sentence;
					failureDetail = cause.detail === cause.sentence ? '' : cause.detail;
				} else failure = failureSentence(cause, 'This file could not be opened.');
			})
			.finally(() => {
				if (target === file.id) loading = false;
			});
	});

	const isMedia = $derived(file.kind === 'media');
	const isVideo = $derived(/\.(mp4|m4v|webm|ogv)$/i.test(file.uri));
	const isPdf = $derived(/\.pdf$/i.test(file.uri.split(/[?#]/)[0]));
	const isDownload = $derived(downloads(file));

	/* ---------- the slideshow ---------- */
	let stage = $state<HTMLElement | null>(null);
	let frame = $state<HTMLIFrameElement | null>(null);
	let presenting = $state(false);
	let slide = $state(1);
	let total = $state(0);
	/* The file is on its own review origin, or on this host under /review/ in an opaque (sandboxed)
	 * origin, whose messages arrive from origin "null". Trust the sending frame, not the origin;
	 * an opaque origin can only be addressed as "*", and these messages carry nothing private. */
	const origin = $derived(url ? new URL(url, location.href).origin : '');
	const target = $derived(!origin || origin === location.origin ? '*' : origin);
	const pdfSrc = $derived(url && isPdf ? `${url.split('#')[0]}#view=FitH` : '');

	function step(action: 'next' | 'prev' | 'first') {
		if (isPdf) return;
		frame?.contentWindow?.postMessage({ type: 'restless:deck', action }, target);
	}
	function onmessage(event: MessageEvent) {
		if (!frame || event.source !== frame.contentWindow) return;
		const data = event.data as { type?: string; slide?: number; total?: number } | null;
		if (data?.type !== 'restless:deck') return;
		if (typeof data.slide === 'number') slide = data.slide;
		if (typeof data.total === 'number') total = data.total;
	}
	/* Focus stays with the cockpit, which steps the deck by message: a focused cross-origin frame
	 * would swallow Escape. Where the browser cannot go full screen (iPhone Safari), the stage
	 * fills the window instead. */
	let windowed = $state(false);
	async function present() {
		presenting = true;
		try {
			if (!stage?.requestFullscreen) throw new Error('no fullscreen');
			await stage.requestFullscreen();
			windowed = false;
		} catch {
			windowed = true;
		}
	}
	async function stop() {
		presenting = false;
		windowed = false;
		if (document.fullscreenElement) await document.exitFullscreen().catch(() => {});
	}
	function onkeydown(event: KeyboardEvent) {
		if (file.kind !== 'deck' || !url) return;
		if (isPdf) {
			if (event.key === 'Escape' && presenting) void stop();
			return;
		}
		if (
			event.target instanceof Element &&
			event.target.closest('input, textarea, select, [contenteditable="true"]')
		)
			return;
		if (['ArrowRight', 'PageDown', ' '].includes(event.key)) {
			event.preventDefault();
			step('next');
		} else if (['ArrowLeft', 'PageUp'].includes(event.key)) {
			event.preventDefault();
			step('prev');
		} else if (event.key === 'Home') step('first');
		else if (event.key === 'Escape' && presenting) void stop();
	}
</script>

<svelte:window {onmessage} {onkeydown} />
<svelte:document onfullscreenchange={() => (presenting = !!document.fullscreenElement)} />

<div class="viewer">
	<header class="viewer-head">
		<a class="back" href={backHref} title="Back to the Library" aria-label="Back to the Library"
			><ArrowLeft size={16} strokeWidth={1.9} /></a
		>
		<div class="viewer-title">
			<h1 title={file.label}>{file.label}</h1>
			<small title={file.uri}>{owner} · {file.name}</small>
		</div>
		<span class="spacer"></span>
		{#if file.kind === 'deck' && url && !isDownload}
			<button class="btn small primary" type="button" onclick={() => void present()}
				><Play size={14} strokeWidth={2} aria-hidden="true" />Present</button
			>
		{/if}
		{#if url}
			<a class="btn small" href={url} target="_blank" rel="noopener noreferrer"
				>{#if isDownload}<Download
						size={14}
						strokeWidth={1.9}
						aria-hidden="true"
					/>Download{:else}<ExternalLink size={14} strokeWidth={1.9} aria-hidden="true" />Open in a
					tab{/if}</a
			>
		{/if}
	</header>

	<div class="viewer-body" class:deck={file.kind === 'deck'}>
		{#if loading}
			<div class="center"><Pending label="Opening the file" /></div>
		{:else if failure}
			<div class="center">
				<p class="failure">{failure}</p>
				<div class="failure-actions">
					<button class="btn small primary" type="button" onclick={() => (attempt += 1)}
						>Try again</button
					>
					<a class="btn small" href={backHref}>Back to the Library</a>
				</div>
				{#if failureDetail}<details class="failure-detail">
						<summary>Details</summary>
						<code>{failureDetail}</code>
					</details>{/if}
			</div>
		{:else if isDownload}
			<div class="center">
				<p>Browsers cannot show this kind of file. Download it to open it on your computer.</p>
				<a class="btn small primary" href={url}
					><Download size={14} strokeWidth={1.9} aria-hidden="true" />Download {file.name}</a
				>
			</div>
		{:else if file.kind === 'image'}
			<div class="image"><img src={url} alt={file.label} /></div>
		{:else if isMedia}
			<div class="center">
				{#if isVideo}<!-- svelte-ignore a11y_media_has_caption --><video
						src={url}
						controls
						preload="metadata"
					></video>{:else}<audio src={url} controls preload="metadata"></audio>{/if}
			</div>
		{:else if file.kind === 'deck'}
			<div class="stage" class:presenting class:windowed={presenting && windowed} bind:this={stage}>
				{#if isPdf}
					<!-- The browser's PDF reader will not run in a sandboxed frame. The file is already on
					     its own read-only origin, apart from the cockpit, and a PDF runs no page script. -->
					<iframe bind:this={frame} title={file.label} src={pdfSrc} referrerpolicy="no-referrer"
					></iframe>
				{:else}
					<iframe
						bind:this={frame}
						title={file.label}
						src={url}
						sandbox="allow-scripts allow-same-origin allow-forms"
						referrerpolicy="no-referrer"
					></iframe>
				{/if}
				{#if !isPdf || presenting}<div class="deck-bar">
						{#if !isPdf}<button
								type="button"
								onclick={() => step('prev')}
								aria-label="Previous slide"><ChevronLeft size={18} strokeWidth={2} /></button
							>
							<span class="deck-count" aria-live="polite"
								>{total ? `${slide} / ${total}` : `Slide ${slide}`}</span
							>
							<button type="button" onclick={() => step('next')} aria-label="Next slide"
								><ChevronRight size={18} strokeWidth={2} /></button
							>{/if}
						{#if presenting}<button
								type="button"
								onclick={() => void stop()}
								aria-label="Stop presenting"><X size={16} strokeWidth={2} /></button
							>{/if}
					</div>{/if}
			</div>
		{:else if isPdf}
			<iframe title={file.label} src={url} referrerpolicy="no-referrer"></iframe>
		{:else}
			<iframe
				title={file.label}
				src={url}
				sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
				referrerpolicy="no-referrer"
			></iframe>
		{/if}
	</div>
</div>

<style>
	.viewer {
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		width: 100%;
		min-height: 0;
	}
	.viewer-head {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 58px;
		padding: 8px 12px;
		border-bottom: 1px solid var(--border);
	}
	.back {
		display: inline-grid;
		place-items: center;
		width: 30px;
		height: 30px;
		border-radius: var(--radius-pane);
		color: var(--text-secondary);
	}
	.back:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.viewer-title {
		display: grid;
		min-width: 0;
	}
	.viewer-title h1 {
		margin: 0;
		overflow: hidden;
		font-size: var(--t-head);
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.viewer-title small {
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-body);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.spacer {
		flex: 1;
	}
	.viewer-head .btn {
		gap: 6px;
		text-decoration: none;
	}
	.viewer-body {
		position: relative;
		min-height: 0;
		background: var(--surface-alt);
	}
	.viewer-body > iframe {
		display: block;
		width: 100%;
		height: 100%;
		border: 0;
		background: #fff;
	}
	.center {
		display: grid;
		place-content: center;
		justify-items: center;
		gap: 12px;
		height: 100%;
		padding: 24px;
		color: var(--text-secondary);
		text-align: center;
	}
	.center p {
		max-width: 420px;
		margin: 0;
	}
	.failure {
		color: var(--ink);
		font-weight: 500;
	}
	.failure-actions {
		display: flex;
		gap: 8px;
	}
	.failure-detail {
		max-width: 480px;
		color: var(--text-tertiary);
		text-align: left;
	}
	.failure-detail summary {
		cursor: pointer;
		text-align: center;
	}
	.failure-detail code {
		display: block;
		margin-top: 6px;
		font-size: var(--t-label);
		overflow-wrap: anywhere;
	}
	.center video {
		max-width: 100%;
		max-height: 70vh;
		border-radius: var(--radius-lg);
		background: #000;
	}
	.image {
		display: grid;
		place-items: center;
		height: 100%;
		padding: 24px;
		overflow: auto;
	}
	.image img {
		max-width: 100%;
		max-height: 100%;
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-lift);
		background: repeating-conic-gradient(#eef0f4 0 25%, #fff 0 50%) 0 0 / 16px 16px;
	}
	.stage {
		position: relative;
		display: grid;
		grid-template-rows: minmax(0, 1fr) auto;
		height: 100%;
		background: #1b1f27;
	}
	.stage iframe {
		width: 100%;
		height: 100%;
		border: 0;
		background: #fff;
	}
	.stage.windowed {
		position: fixed;
		inset: 0;
		z-index: var(--z-overlay);
	}
	.stage.presenting {
		background: #000;
	}
	.deck-bar {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		height: 48px;
		color: #e8ebf2;
	}
	.deck-bar button {
		display: inline-grid;
		place-items: center;
		width: 34px;
		height: 34px;
		border: 1px solid rgba(255, 255, 255, 0.16);
		border-radius: var(--radius-pane);
		background: rgba(255, 255, 255, 0.06);
		color: inherit;
		cursor: pointer;
	}
	.deck-bar button:hover {
		background: rgba(255, 255, 255, 0.14);
	}
	.deck-count {
		min-width: 72px;
		font-variant-numeric: tabular-nums;
		text-align: center;
	}
</style>
