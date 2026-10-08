<script lang="ts">
	/* The first look round a new company, straight after meeting Exec: a few calm steps, each
	 * lighting up the part of the screen it describes. It teaches the app's own places, so the
	 * words are the app's, not Exec's. Skippable, and shown once. */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onDestroy } from 'svelte';

	let { companyId }: { companyId: string } = $props();

	const STEPS = [
		{
			place: 'attention',
			title: 'This is your Inbox',
			text: 'Anything that needs you lands here: a decision, a sign-in, something to look over. When it’s empty, you’re free.'
		},
		{
			place: 'work',
			title: 'Work',
			text: 'What the company is doing right now, and who’s looking after each piece.'
		},
		{
			place: 'library',
			title: 'Library',
			text: 'Documents and sheets the company writes. Open one to read it, comment, or edit together.'
		},
		{
			place: 'apps',
			title: 'Apps',
			text: 'Give the company new abilities. Just say what it should be able to do.'
		},
		{
			place: 'exec',
			title: 'And I’m right here',
			text: 'Ask me anything, or tell me what you want. I’ll take it from there.'
		}
	] as const;

	let step = $state(0);
	const current = $derived(STEPS[step]);

	$effect(() => {
		document.documentElement.dataset.tour = current.place;
	});
	onDestroy(() => {
		delete document.documentElement.dataset.tour;
	});

	function finish() {
		delete document.documentElement.dataset.tour;
		try {
			localStorage.setItem(`restless:tour:${companyId}`, 'done');
		} catch {
			// Shown once is a convenience; without storage the URL decides.
		}
		const url = new URL(page.url);
		url.searchParams.delete('tour');
		void goto(`${url.pathname}${url.search}`, {
			replaceState: true,
			noScroll: true,
			keepFocus: true
		});
	}
</script>

<div class="tour bridge-tokens" role="dialog" aria-label="A quick look around" aria-live="polite">
	<span class="count">{step + 1} of {STEPS.length}</span>
	{#key step}
		<div class="words">
			<strong>{current.title}</strong>
			<p>{current.text}</p>
		</div>
	{/key}
	<div class="actions">
		<button type="button" class="skip" onclick={finish}>Skip</button>
		{#if step < STEPS.length - 1}
			<button type="button" class="next" onclick={() => (step += 1)}>Next</button>
		{:else}
			<button type="button" class="next" onclick={finish}>Got it</button>
		{/if}
	</div>
</div>

<style>
	.tour {
		position: fixed;
		bottom: 24px;
		left: 24px;
		z-index: var(--z-popover, 80);
		display: grid;
		gap: 10px;
		width: min(340px, calc(100vw - 48px));
		padding: 16px 18px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-float);
		animation: tour-in 500ms var(--ease-out, ease-out) both;
	}
	.count {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.words {
		display: grid;
		gap: 4px;
		animation: tour-in 400ms var(--ease-out, ease-out) both;
	}
	strong {
		font-size: var(--t-head);
		font-weight: 500;
	}
	p {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.5;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
	.actions button {
		height: 32px;
		padding: 0 14px;
		border-radius: 999px;
		font: inherit;
		font-size: var(--t-body);
		cursor: pointer;
	}
	.skip {
		border: 0;
		background: transparent;
		color: var(--text-tertiary);
	}
	.next {
		border: 0;
		background: var(--ink);
		color: var(--surface-raised);
	}
	/* What the step describes lights up softly, wherever it is on screen. */
	:global(html[data-tour='attention'] .tb-tab[data-surface='attention']),
	:global(html[data-tour='work'] .tb-tab[data-surface='work']),
	:global(html[data-tour='library'] .tb-tab[data-surface='library']),
	:global(html[data-tour='apps'] .tb-tab[data-surface='apps']) {
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--intent-conversation) 55%, transparent);
		border-radius: var(--radius-control);
		transition: box-shadow 400ms var(--ease-out, ease-out);
	}
	:global(html[data-tour='exec'] #bridge-exrail) {
		box-shadow: inset 0 0 0 2px color-mix(in srgb, var(--intent-conversation) 45%, transparent);
	}
	@keyframes tour-in {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.tour,
		.words {
			animation: none;
		}
	}
</style>
