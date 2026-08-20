<script lang="ts">
	import Markdown from '$lib/primitives/Markdown.svelte';
	import type { AttentionItem } from '$lib/model/view';

	let { item }: { item: AttentionItem } = $props();

	const categoryLabel = $derived(
		(
			{
				approval: 'Approval',
				review: 'Outcome review',
				decision: 'Decision',
				blocker: 'Blocked',
				opportunity: 'Opportunity',
				contradiction: 'Conflicting evidence',
				human_step: 'Your participation'
			} as Record<string, string>
		)[item.category] ?? item.category.replaceAll('_', ' ')
	);

	function when(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		return date.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<section class="attention-rail-card" aria-labelledby="attention-rail-title">
	<header class="attention-rail-opening">
		<div class="attention-rail-meta">
			<span>{categoryLabel}</span>
			{#if item.deadline}<strong>By {item.deadline}</strong>{/if}
		</div>
		<h2 id="attention-rail-title">{item.title}</h2>
		<div class="attention-rail-situation"><Markdown text={item.whatHappened} /></div>
		<p class="attention-rail-impact">{item.whyItMatters}</p>
		{#if item.uncertainty}
			<p class="attention-rail-uncertainty">
				<strong>What remains uncertain</strong>
				<span>{item.uncertainty}</span>
			</p>
		{/if}
	</header>

	<section class="attention-rail-recommendation" aria-label="Recommendation">
		<span>Recommendation</span>
		<div><Markdown text={item.recommendation} /></div>
	</section>

	<section class="attention-rail-move" aria-label="Your next move">
		<span>Your next move</span>
		<div><Markdown text={item.requestedAction} /></div>
		<small><strong>If you wait:</strong> {item.ifNoAction}</small>
	</section>

	<footer class="attention-rail-provenance">
		<span>Prepared by</span>
		<strong>{item.briefAuthor?.display ?? item.responsibleActor?.display ?? 'Source record'}</strong
		>
		{#if item.briefedAt}<time>{when(item.briefedAt)}</time>{/if}
	</footer>

	<details class="attention-rail-evidence">
		<summary>
			<span>Evidence</span>
			<small>{item.evidence.length} item{item.evidence.length === 1 ? '' : 's'}</small>
		</summary>
		<div class="attention-rail-evidence-body">
			{#each item.evidence as evidence (`${evidence.kind}:${evidence.label}`)}
				{#if evidence.content}
					<article>
						<strong>{evidence.label}</strong>
						<div><Markdown text={evidence.content} /></div>
					</article>
				{:else if evidence.uri}
					<a href={evidence.uri} target="_blank" rel="noreferrer">
						{evidence.label} <span aria-hidden="true">↗</span>
					</a>
				{/if}
			{/each}
		</div>
	</details>
</section>

<style>
	.attention-rail-card {
		min-height: 0;
		flex: 1 1 auto;
		overflow: auto;
		background:
			linear-gradient(
				145deg,
				color-mix(in srgb, var(--intent-feedback) 5%, white),
				transparent 34%
			),
			var(--surface-pane);
	}
	.attention-rail-opening {
		padding: 18px 16px 17px;
	}
	.attention-rail-meta {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		font: 600 var(--t-label) var(--font-mono);
		letter-spacing: var(--track-label);
		text-transform: uppercase;
		color: var(--intent-feedback);
	}
	.attention-rail-meta strong {
		padding: 3px 6px;
		border: 1px solid color-mix(in srgb, var(--intent-feedback) 24%, var(--border));
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--intent-feedback-soft) 62%, white);
		font-weight: 500;
	}
	h2 {
		margin: 11px 0 0;
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.12;
		letter-spacing: -0.025em;
	}
	.attention-rail-situation {
		margin-top: 14px;
		font-size: var(--t-head);
		font-weight: 500;
		line-height: 1.45;
		color: var(--text-secondary);
	}
	.attention-rail-impact {
		margin: 10px 0 0;
		font-size: var(--t-body);
		line-height: 1.55;
		color: var(--ink);
	}
	.attention-rail-uncertainty {
		display: grid;
		gap: 3px;
		margin: 13px 0 0;
		padding: 9px 10px;
		border-left: 3px solid var(--intent-authority);
		background: var(--intent-authority-soft);
		font-size: var(--t-body);
		line-height: 1.45;
		color: var(--text-secondary);
	}
	.attention-rail-uncertainty strong {
		color: var(--ink);
	}
	.attention-rail-recommendation,
	.attention-rail-move {
		padding: 15px 16px;
		border-top: 1px solid var(--border);
	}
	.attention-rail-recommendation {
		background: linear-gradient(
			100deg,
			color-mix(in srgb, var(--intent-feedback-soft) 82%, white),
			color-mix(in srgb, var(--intent-feedback-soft) 34%, white)
		);
	}
	.attention-rail-recommendation > span,
	.attention-rail-move > span,
	.attention-rail-provenance > span {
		display: block;
		margin-bottom: 6px;
		font: 600 var(--t-label) var(--font-mono);
		letter-spacing: var(--track-label);
		text-transform: uppercase;
		color: var(--text-tertiary);
	}
	.attention-rail-recommendation > span {
		color: var(--intent-feedback);
	}
	.attention-rail-recommendation > div,
	.attention-rail-move > div {
		font-size: var(--t-head);
		font-weight: 600;
		line-height: 1.45;
	}
	.attention-rail-move small {
		display: block;
		margin-top: 9px;
		font-size: var(--t-body);
		line-height: 1.45;
		color: var(--text-secondary);
	}
	.attention-rail-provenance {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		align-items: baseline;
		gap: 3px 8px;
		padding: 11px 16px;
		border-top: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-alt) 72%, white);
		font-size: var(--t-label);
	}
	.attention-rail-provenance > span {
		grid-column: 1 / -1;
		margin: 0;
	}
	.attention-rail-provenance time {
		color: var(--text-tertiary);
	}
	.attention-rail-evidence {
		border-top: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-alt) 62%, white);
	}
	.attention-rail-evidence summary {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 12px 16px;
		cursor: pointer;
		font: 500 var(--t-body) var(--font-mono);
		color: var(--text-secondary);
		list-style: none;
	}
	.attention-rail-evidence summary::-webkit-details-marker {
		display: none;
	}
	.attention-rail-evidence summary::after {
		content: '›';
		width: 14px;
		color: var(--text-tertiary);
		font-size: var(--t-head);
		line-height: 1;
		transform: rotate(0);
		transition: transform var(--motion-state) var(--ease-out);
	}
	.attention-rail-evidence[open] summary::after {
		transform: rotate(90deg);
	}
	.attention-rail-evidence summary:focus-visible {
		outline: 3px solid color-mix(in srgb, var(--intent-feedback) 28%, transparent);
		outline-offset: -3px;
	}
	.attention-rail-evidence summary small {
		font: var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	.attention-rail-evidence-body {
		display: grid;
		gap: 10px;
		padding: 12px 16px 16px;
		border-top: 1px solid var(--border);
		animation: bridge-disclosure-in var(--motion-disclosure) var(--ease-out) both;
	}
	.attention-rail-evidence-body article,
	.attention-rail-evidence-body a {
		display: grid;
		gap: 5px;
		padding: 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.45;
		text-decoration: none;
	}
	.attention-rail-evidence-body article > strong {
		color: var(--ink);
		font-size: var(--t-label);
		font-family: var(--font-mono);
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}
	.attention-rail-evidence-body a:hover {
		border-color: color-mix(in srgb, var(--intent-feedback) 34%, var(--border));
		color: var(--ink);
	}
</style>
