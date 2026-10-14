<script lang="ts">
	/* The first look round a new company, straight after meeting Exec: a few calm steps, each
	 * lighting up the part of the screen it describes. It teaches the app's own places, so the
	 * words are the app's, not Exec's. Skippable with Escape, steps back with Back, takes focus
	 * so the keyboard can follow it, and replays from ⌘K ("Show the tour again"). */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onDestroy, tick } from 'svelte';

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
			place: 'people',
			title: 'People',
			text: 'Everyone in the company, people and agents. Talk to anyone, or see what they’re looking after.'
		},
		{
			place: 'apps',
			title: 'Apps',
			text: 'Give the company new abilities. Just say what it should be able to do.'
		},
		{
			place: 'company',
			title: 'Company',
			text: 'Who the company is and how it runs: the charter, brand and voice, spending limits and the keys it holds.'
		},
		{
			place: 'exec',
			title: 'And I’m right here',
			text: 'Ask me anything, or tell me what you want. I’ll take it from there.'
		}
	] as const;

	let step = $state(0);
	const current = $derived(STEPS[step]);
	let card = $state<HTMLDivElement>();

	$effect(() => {
		document.documentElement.dataset.tour = current.place;
	});
	/* Each step takes focus at its main button, so Enter moves on and a screen reader hears it. */
	$effect(() => {
		void step;
		void tick().then(() => card?.querySelector<HTMLButtonElement>('.next')?.focus());
	});
	onDestroy(() => {
		delete document.documentElement.dataset.tour;
	});

	function finish() {
		delete document.documentElement.dataset.tour;
		const url = new URL(page.url);
		url.searchParams.delete('tour');
		void goto(`${url.pathname}${url.search}`, {
			replaceState: true,
			noScroll: true,
			keepFocus: true
		});
	}
</script>

<svelte:window
	onkeydown={(event) => {
		if (event.key === 'Escape') finish();
	}}
/>

<div
	class="tour bridge-tokens"
	role="dialog"
	aria-label="A quick look around"
	aria-live="polite"
	bind:this={card}
>
	<span class="count">{step + 1} of {STEPS.length}</span>
	{#key step}
		<div class="words">
			<strong>{current.title}</strong>
			<p>{current.text}</p>
		</div>
	{/key}
	<div class="actions">
		<button type="button" class="skip" onclick={finish} title="Esc">Skip</button>
		{#if step > 0}<button type="button" class="back" onclick={() => (step -= 1)}>Back</button>{/if}
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
		margin-right: auto;
		border: 0;
		background: transparent;
		color: var(--text-tertiary);
	}
	.back {
		border: 1px solid var(--border);
		background: transparent;
		color: var(--ink);
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
	:global(html[data-tour='people'] .tb-tab[data-surface='people']),
	:global(html[data-tour='apps'] .tb-tab[data-surface='apps']),
	:global(html[data-tour='company'] .tb-tab[data-surface='company']) {
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--intent-conversation) 55%, transparent);
		border-radius: var(--radius-control);
		transition: box-shadow 400ms var(--ease-out, ease-out);
	}
	:global(html[data-tour='exec'] #bridge-exrail) {
		box-shadow: inset 0 0 0 2px color-mix(in srgb, var(--intent-conversation) 45%, transparent);
	}
	/* Where the chat floats over the page it stays closed for the tour, so the last step lights the
	 * button that opens it, and the card sits above the dock rather than over the tabs it names. */
	@media (max-width: 980px) {
		:global(html[data-tour='exec'] .tb-exec) {
			box-shadow: 0 0 0 2px color-mix(in srgb, var(--intent-conversation) 55%, transparent);
			transition: box-shadow 400ms var(--ease-out, ease-out);
		}
		/* Opened from that step, the chat takes the screen; the card waits until it closes rather
		 * than sitting over the composer. */
		:global(html:has(#bridge-exrail.open:not(.wide))) .tour {
			display: none;
		}
	}
	@media (max-width: 760px) {
		.tour {
			bottom: calc(var(--dock-h) + env(safe-area-inset-bottom) + 12px);
			left: 12px;
			width: calc(100vw - 24px);
		}
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
