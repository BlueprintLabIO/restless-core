<script lang="ts">
	import type { Snippet } from 'svelte';
	import MatrixGlyph, { GLYPHS } from '../glyph/MatrixGlyph.svelte';
	import Skeleton from '../feedback/Skeleton.svelte';
	import {
		companyMarkRows,
		orderPortfolio,
		portfolioPrimary,
		portfolioSignal,
		type CompanyPortfolioEntry
	} from '../portfolio';

	/* The owner's companies, one card each, ordered by what needs them. Every host renders this
	 * from the same content-free card (company projection v2), so a local appliance and Cloud show
	 * the same thing for the same numbers. The host owns entry, authority and transport. */
	let {
		companies,
		loaded = true,
		title = 'Your companies',
		actions = null,
		before = null,
		feedback = null,
		empty = null,
		footer = null,
		rowActions = null,
		now = Date.now()
	}: {
		companies: CompanyPortfolioEntry[];
		loaded?: boolean;
		title?: string;
		/** Page actions beside the title, e.g. New company. */
		actions?: Snippet | null;
		before?: Snippet | null;
		feedback?: Snippet | null;
		empty?: Snippet | null;
		footer?: Snippet | null;
		/** The card's menu: secondary routes such as Archive, Compute or Service. */
		rowActions?: Snippet<[CompanyPortfolioEntry]> | null;
		/** The clock relative times are read against; injectable for fixtures. */
		now?: number;
	} = $props();

	const ordered = $derived(orderPortfolio(companies));

	function entryLabel(company: CompanyPortfolioEntry): string {
		if (company.entry?.label) return company.entry.label;
		const primary = portfolioPrimary(company);
		return `Open ${company.name}${primary ? '. ' + primary : ''}`;
	}
</script>

{#if !loaded}
	<main class="portfolio-main" aria-busy="true">
		<header class="portfolio-head"><h1>{title}</h1></header>
		{#if feedback}{@render feedback()}{:else}<div class="portfolio-cards">
				<Skeleton label="Loading companies" variant="list" count={3} />
			</div>{/if}
	</main>
{:else}
	<main class="portfolio-main">
		{#if before}{@render before()}{/if}
		<header class="portfolio-head">
			<h1>{title}</h1>
			{#if actions}<div class="portfolio-actions">{@render actions()}</div>{/if}
		</header>
		{#if feedback}<div class="portfolio-feedback">{@render feedback()}</div>{/if}
		{#if ordered.length}
			<ul class="portfolio-cards" aria-label="Companies">
				{#each ordered as company (company.id)}
					{@const primary = portfolioPrimary(company)}
					{@const signal = portfolioSignal(company, now)}
					<li
						class="portfolio-card"
						class:dormant={company.dormant}
						class:blocked={!!company.issue}
						data-company-id={company.id}
						data-tone={company.tone ?? 'waiting'}
					>
						{#if company.entry && 'href' in company.entry}
							<a class="portfolio-entry" href={company.entry.href} aria-label={entryLabel(company)}
							></a>
						{:else if company.entry && 'formAction' in company.entry}
							<form method="POST" action={company.entry.formAction} class="portfolio-entry-form">
								{#each Object.entries(company.entry.fields) as [name, value]}<input
										type="hidden"
										{name}
										{value}
									/>{/each}
								<button class="portfolio-entry" aria-label={entryLabel(company)}></button>
							</form>
						{/if}
						<span class="portfolio-mark" aria-hidden="true">
							<MatrixGlyph rows={companyMarkRows(company.id)} size={16} />
						</span>
						<span class="portfolio-copy">
							<span class="portfolio-title">
								<strong>{company.name}</strong>
								<span class="portfolio-state"><i aria-hidden="true"></i>{company.status}</span>
							</span>
							{#if signal}<span class="portfolio-signal" title={signal}>{signal}</span>{/if}
						</span>
						{#if primary}<span class="portfolio-primary" aria-hidden="true">{primary}</span>{/if}
						{#if rowActions}<div class="portfolio-menu">{@render rowActions(company)}</div>{/if}
					</li>
				{/each}
			</ul>
		{:else}
			<div class="portfolio-empty">
				<MatrixGlyph rows={GLYPHS.ring} size={14} />
				{#if empty}{@render empty()}{:else}<h2>Start your first company</h2>{/if}
			</div>
		{/if}
		{#if footer}<footer class="portfolio-footer">{@render footer()}</footer>{/if}
	</main>
{/if}

<style>
	.portfolio-main {
		width: min(880px, 100%);
		margin-inline: auto;
		padding: clamp(24px, 4vw, 48px) clamp(16px, 4vw, 32px) 56px;
	}
	.portfolio-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: 20px;
	}
	.portfolio-head h1 {
		margin: 0;
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.2;
		letter-spacing: -0.02em;
	}
	.portfolio-actions {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.portfolio-feedback {
		margin-bottom: 16px;
	}
	/* One quiet list, like Linear's: rows on hairlines, a wash under the pointer, no cards. */
	.portfolio-cards {
		display: grid;
		margin: 0;
		padding: 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		list-style: none;
	}
	.portfolio-card {
		position: relative;
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: 12px;
		min-height: 56px;
		padding: 8px 48px 8px 10px;
		border-radius: var(--radius-control);
		transition: background var(--motion-state) var(--ease-standard);
	}
	.portfolio-card + .portfolio-card::before {
		content: '';
		position: absolute;
		top: 0;
		right: 10px;
		left: 54px;
		height: 1px;
		background: var(--border);
	}
	.portfolio-card:hover {
		background: var(--wash-hover, var(--surface-alt));
	}
	.portfolio-card:hover::before,
	.portfolio-card:hover + .portfolio-card::before {
		background: transparent;
	}
	.portfolio-card.dormant {
		background: transparent;
	}
	.portfolio-card.dormant > :not(.portfolio-menu) {
		opacity: 0.6;
	}
	.portfolio-entry {
		position: absolute;
		inset: 0;
		z-index: 1;
		width: 100%;
		padding: 0;
		border: 0;
		border-radius: inherit;
		background: transparent;
		cursor: pointer;
	}
	.portfolio-entry:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.portfolio-entry-form {
		display: contents;
	}
	.portfolio-mark {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		color: var(--text-secondary);
	}
	.portfolio-card[data-tone='presence'] .portfolio-mark {
		color: var(--state-success);
	}
	.portfolio-card.blocked .portfolio-mark {
		background: color-mix(in srgb, var(--state-danger) 10%, transparent);
		color: var(--state-danger);
	}
	.portfolio-copy {
		display: grid;
		gap: 3px;
		min-width: 0;
	}
	.portfolio-title {
		display: flex;
		align-items: baseline;
		gap: 10px;
		min-width: 0;
	}
	.portfolio-title strong {
		overflow: hidden;
		font-size: var(--t-body);
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.portfolio-state {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.portfolio-state i {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-tertiary);
	}
	.portfolio-card[data-tone='presence'] .portfolio-state i {
		background: var(--state-success);
	}
	.portfolio-card[data-tone='unavailable'] .portfolio-state i,
	.portfolio-card.blocked .portfolio-state i {
		background: var(--state-danger);
	}
	.portfolio-signal {
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.portfolio-card.blocked .portfolio-signal {
		white-space: normal;
	}
	.portfolio-primary {
		flex: none;
		color: var(--intent-authority);
		font-size: var(--t-label);
		font-weight: 500;
		white-space: nowrap;
	}
	.portfolio-card.blocked .portfolio-primary {
		color: var(--state-danger);
	}
	/* The row's menu appears with the row, as Linear's do; a keyboard or an open menu keeps it. */
	.portfolio-menu {
		position: absolute;
		top: 50%;
		right: 8px;
		z-index: 2;
		transform: translateY(-50%);
		opacity: 0;
		transition: opacity var(--motion-state) var(--ease-standard);
	}
	.portfolio-card:hover .portfolio-menu,
	.portfolio-menu:focus-within,
	.portfolio-menu:has(:global([open])) {
		opacity: 1;
	}
	@media (hover: none) {
		.portfolio-menu {
			opacity: 1;
		}
	}
	.portfolio-empty {
		padding: 56px 16px;
		border: 1px dashed var(--border-strong);
		border-radius: var(--radius-lg);
		color: var(--text-tertiary);
		text-align: center;
	}
	.portfolio-empty :global(h2) {
		margin: 12px 0 6px;
		color: var(--ink);
		font-size: var(--t-head);
	}
	.portfolio-empty :global(p) {
		margin: 0;
		font-size: var(--t-body);
	}
	.portfolio-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex-wrap: wrap;
		gap: 12px;
		margin-top: 16px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	@media (prefers-reduced-motion: reduce) {
		.portfolio-card {
			transition: none;
		}
	}
	@media (max-width: 560px) {
		.portfolio-card {
			grid-template-columns: auto minmax(0, 1fr);
			align-items: start;
			gap: 12px;
			padding-right: 56px;
		}
		.portfolio-menu {
			top: 8px;
			transform: none;
		}
		.portfolio-signal {
			white-space: normal;
		}
		.portfolio-primary {
			grid-column: 2;
			justify-self: start;
		}
		.portfolio-title {
			flex-wrap: wrap;
			row-gap: 2px;
		}
	}
</style>
