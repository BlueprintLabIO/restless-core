<script lang="ts">
	/* Any route that fails to load or does not exist lands here, in the cockpit's
	 * own frame rather than the framework's bare status text. The way back goes
	 * to the company the address named when there is one. */
	import { page } from '$app/state';
	import MatrixGlyph, { GLYPHS } from '$lib/ui/glyph/MatrixGlyph.svelte';

	const missing = $derived(page.status === 404);
	const companyId = $derived(page.params.companyId);
	const home = $derived(companyId ? `/${encodeURIComponent(companyId)}` : '/');
</script>

<svelte:head>
	<title>{missing ? 'Not found' : 'Something went wrong'}</title>
</svelte:head>

<div class="bridge-root error-root">
	<main class="error-state">
		<span class="error-mark" aria-hidden="true">
			<MatrixGlyph rows={missing ? GLYPHS.r : GLYPHS.alert} size={12} />
		</span>
		<h1>{missing ? 'This page does not exist' : 'This page could not open'}</h1>
		<p>
			{missing
				? 'The address may be mistyped, or what it pointed to has moved.'
				: (page.error?.message ?? 'Try again in a moment.')}
		</p>
		<div class="error-actions">
			{#if !missing}
				<button class="btn" type="button" onclick={() => location.reload()}>Try again</button>
			{/if}
			<a class="btn primary" href={home}>{companyId ? 'Back to company' : 'Back to companies'}</a>
		</div>
	</main>
</div>

<style>
	:global(.bridge-root).error-root {
		display: grid;
		grid-template-rows: none;
		min-height: 100dvh;
		place-items: center;
		padding: var(--space-6) var(--space-4);
		background: var(--bg-app);
	}
	.error-state {
		display: grid;
		justify-items: center;
		gap: var(--space-3);
		max-width: 420px;
		text-align: center;
		animation: error-in var(--motion-state) var(--ease-out) both;
	}
	.error-mark {
		display: grid;
		width: 44px;
		height: 44px;
		place-items: center;
		margin-bottom: var(--space-2);
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft), var(--bevel-subtle);
		color: var(--text-secondary);
	}
	h1 {
		margin: 0;
	}
	p {
		margin: 0;
		color: var(--text-secondary);
		text-wrap: pretty;
	}
	.error-actions {
		display: flex;
		gap: var(--space-2);
		margin-top: var(--space-3);
	}
	@keyframes error-in {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.error-state {
			animation: none;
		}
	}
</style>
