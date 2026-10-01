<script lang="ts">
	import type { Snippet } from 'svelte';

	/* The folio returns one outcome with its evidence and one bounded judgement.
	 * It only lays those out; what sits in each slot (markdown, a live decision
	 * control, an info tip) is supplied by the caller, which is what lets the same
	 * view render in the cockpit and on a public page. */
	let {
		title,
		whatHappened,
		whyItMatters,
		uncertainty,
		category = 'decision',
		headingLevel = 1,
		context,
		recommendation,
		decision,
		detailsCount = 0,
		details
	}: {
		title: string;
		whatHappened: string;
		whyItMatters: string;
		uncertainty?: string;
		category?: string;
		/** The folio is the page's h1 in the cockpit; on a page with its own h1 it is an h3. */
		headingLevel?: 1 | 2 | 3;
		/** Sits beside the title: an info tip, a deadline. */
		context?: Snippet;
		/** Recommendation body. Omit when it would repeat the text above. */
		recommendation?: Snippet;
		/** The judgement itself: hold-to-approve, reply, review. */
		decision?: Snippet;
		detailsCount?: number;
		/** Supporting evidence, revealed on request. */
		details?: Snippet;
	} = $props();
</script>

<article class="owner-folio category-{category}">
	<header class="folio-opening">
		<div class="folio-heading">
			<svelte:element this={`h${headingLevel}`}>{title}</svelte:element>
			{#if context}<div class="folio-context">{@render context()}</div>{/if}
		</div>
		<div class="folio-context-copy">
			<p>{whatHappened}</p>
			<p>{whyItMatters}</p>
		</div>
		{#if uncertainty}
			<p class="folio-uncertainty"><strong>Uncertain:</strong> {uncertainty}</p>
		{/if}
	</header>

	{#if recommendation}
		<section class="folio-recommendation" aria-label="Recommendation">
			<strong>Recommended</strong>
			{@render recommendation()}
		</section>
	{/if}

	{@render decision?.()}

	{#if details}
		<details class="folio-details">
			<summary title="Prepared by, supporting evidence, and source references">
				<span class="evidence-chevron" aria-hidden="true">›</span>
				<span>Details</span>
				{#if detailsCount}<small>· {detailsCount} item{detailsCount === 1 ? '' : 's'}</small>{/if}
			</summary>
			<div class="folio-evidence-body">{@render details()}</div>
		</details>
	{/if}
</article>

<style>
	.owner-folio {
		container-type: inline-size;
		width: min(760px, calc(100% - 40px));
		margin: 20px auto;
		padding: clamp(20px, 3vw, 32px);
		background: var(--surface-pane);
	}
	.folio-opening {
		padding: 0;
	}
	.folio-heading {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: var(--space-4);
	}
	.folio-context {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: var(--space-2);
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.folio-context :global(time) {
		max-width: 18ch;
	}
	.folio-heading > :global(:is(h1, h2, h3)) {
		max-width: 760px;
		margin: 0;
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.16;
		letter-spacing: -0.03em;
		text-wrap: balance;
	}
	.folio-context-copy {
		max-width: 72ch;
		margin-top: var(--space-4);
		color: var(--text-secondary);
		font-size: var(--t-head);
		line-height: 1.5;
	}
	.folio-context-copy p {
		margin: 0;
	}
	.folio-context-copy p + p {
		margin-top: var(--space-2);
	}
	.folio-uncertainty {
		margin: var(--space-3) 0 0;
		font-size: var(--t-body);
		line-height: 1.45;
		color: var(--text-secondary);
	}
	.folio-uncertainty strong {
		color: var(--intent-authority);
		font-weight: 600;
	}
	.folio-recommendation {
		margin: var(--space-4) 0;
		padding: var(--space-3) 0;
		border-block: 1px solid var(--border);
		color: var(--ink);
	}
	.folio-recommendation > strong {
		color: var(--intent-feedback);
		font-size: var(--t-body);
	}
	.folio-details {
		margin-top: var(--space-3);
		border-top: 1px solid var(--border);
	}
	.folio-details summary {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding: 10px 0;
		cursor: pointer;
		list-style: none;
		font: 500 var(--t-body) var(--font-ui);
		color: var(--text-secondary);
	}
	.folio-details summary::-webkit-details-marker {
		display: none;
	}
	.folio-details summary:hover {
		color: var(--ink);
	}
	.folio-details summary:focus-visible {
		outline: 2px solid var(--intent-feedback);
		outline-offset: 2px;
	}
	.folio-details summary small {
		font: inherit;
		font-weight: 400;
		color: var(--text-tertiary);
	}
	.evidence-chevron {
		width: var(--space-3);
		flex: 0 0 var(--space-3);
		font-size: var(--t-head);
		line-height: 1;
		color: var(--text-tertiary);
		transform-origin: center;
		transition: transform 120ms ease;
	}
	.folio-details[open] .evidence-chevron {
		transform: rotate(90deg);
	}
	.folio-evidence-body {
		padding: 2px 0 12px;
	}

	@media (max-width: 760px) {
		.owner-folio {
			width: calc(100% - 24px);
			margin-block: 12px;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.evidence-chevron {
			transition: none;
		}
	}
</style>
