<script lang="ts">
	/* Company opens on its overview; every detail page sits under a quiet way back to it. Pages
	 * render through the shared Page frame, so this layout owns no headers or widths of its own. */
	import { page } from '$app/state';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';

	let { children } = $props();
	const companyId = $derived(page.params.companyId ?? 'aris');
	const overview = $derived(`/${companyId}/company`);
	const detail = $derived(page.url.pathname !== overview);
	const desktopFocus = $derived(
		page.url.pathname === `/${companyId}/company/computer` &&
			page.url.searchParams.get('focus') === 'desktop'
	);

	/* E edits where a page offers it. */
	function keyboard(event: KeyboardEvent) {
		if (
			(event.target as HTMLElement)?.closest(
				'input, textarea, select, [contenteditable], #bridge-exrail'
			) ||
			event.defaultPrevented ||
			document.querySelector('dialog[open]')
		)
			return;
		if (event.key === 'e' && !event.metaKey && !event.ctrlKey && !event.altKey) {
			const edit = [...document.querySelectorAll<HTMLButtonElement>('.company-main button')].find(
				(button) => /^(Edit|Write charter|Write identity)$/.test(button.textContent?.trim() ?? '')
			);
			if (edit) {
				event.preventDefault();
				edit.click();
			}
		}
	}
</script>

<svelte:window onkeydown={keyboard} />

{#if desktopFocus}
	<div class="company-focus">{@render children()}</div>
{:else}
	<main class="company-main">
		{#if detail}<a class="company-back" href={overview}
				><ArrowLeft size={14} strokeWidth={1.8} aria-hidden="true" />Company</a
			>{/if}
		{@render children()}
	</main>
{/if}

<style>
	.company-focus,
	.company-main {
		position: relative;
		display: flex;
		flex: 1 1 auto;
		width: 100%;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
	}
	.company-main {
		flex-direction: column;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
		box-shadow: var(--bevel), var(--shadow-soft);
	}
	.company-main > :global(:not(.company-back)) {
		flex: 1 1 auto;
		min-height: 0;
	}
	.company-back {
		position: absolute;
		top: 12px;
		left: 14px;
		z-index: 2;
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 8px;
		border-radius: var(--radius-control);
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	@media (max-width: 760px) {
		.company-back {
			position: static;
			align-self: flex-start;
			margin: 10px 0 -14px 10px;
		}
	}
	.company-back:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
</style>
