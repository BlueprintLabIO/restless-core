<script lang="ts">
	import type { Snippet } from 'svelte';
	import AccountShell from './AccountShell.svelte';
	import MatrixGlyph, { GLYPHS } from '../glyph/MatrixGlyph.svelte';
	import SemanticMark from '../glyph/SemanticMark.svelte';
	import Skeleton from '../feedback/Skeleton.svelte';
	import type { CompanyPortfolioEntry } from '../portfolio';

	let {
		companies,
		loaded = true,
		brandName = 'Restless',
		homeHref = '/',
		headerActions = null,
		before = null,
		feedback = null,
		empty = null,
		footer = null,
		rowActions = null
	}: {
		companies: CompanyPortfolioEntry[];
		loaded?: boolean;
		brandName?: string;
		homeHref?: string;
		headerActions?: Snippet | null;
		before?: Snippet | null;
		feedback?: Snippet | null;
		empty?: Snippet | null;
		footer?: Snippet | null;
		rowActions?: Snippet<[CompanyPortfolioEntry]> | null;
	} = $props();

	function entryLabel(company: CompanyPortfolioEntry): string {
		if (company.entry?.label) return company.entry.label;
		if (company.issue) return 'Fix ' + company.name + ' setup. ' + company.issue;
		const next = company.next ? ' Next item of value: ' + company.next + '.' : '';
		const count = company.attentionCount;
		if (count == null) return 'Open ' + company.name + '.' + next;
		if (count === 0) return 'Open ' + company.name + '.' + next + ' No owner attention is waiting.';
		return (
			'Open ' +
			company.name +
			'.' +
			next +
			' ' +
			count +
			' item' +
			(count === 1 ? '' : 's') +
			' need owner attention.'
		);
	}
</script>

<AccountShell {brandName} {homeHref} actions={headerActions}>
	{#if !loaded}
		<main class="portfolio-loading">
			{#if feedback}{@render feedback()}{:else}<Skeleton
					label="Loading companies"
					variant="list"
					count={3}
				/>{/if}
		</main>
	{:else}
		<main class="portfolio-main">
			{#if before}{@render before()}{/if}
			<header class="portfolio-head"><h1>Companies</h1></header>
			{#if feedback}{@render feedback()}{/if}
			<section class="portfolio-table" aria-label="Companies">
				{#if companies.length}
					<div class="portfolio-table-scroll">
						<div class="portfolio-grid">
							<div class="portfolio-grid-head" aria-hidden="true">
								<span>Name</span><span>Current focus</span><span>Next</span><span>Needs you</span>
							</div>
							{#each companies as company (company.id)}
								<div
									class="portfolio-company-row"
									data-company-id={company.id}
									data-state={company.status}
								>
									{#if company.entry && 'href' in company.entry}
										<a
											class="portfolio-entry"
											href={company.entry.href}
											aria-label={entryLabel(company)}
											title={company.issue || company.nextHint || company.focus || undefined}
											><span class="sr-only">{entryLabel(company)}</span></a
										>
									{:else if company.entry && 'formAction' in company.entry}
										<form
											method="POST"
											action={company.entry.formAction}
											class="portfolio-entry-form"
										>
											{#each Object.entries(company.entry.fields) as [name, value]}<input
													type="hidden"
													{name}
													{value}
												/>{/each}
											<button
												class="portfolio-entry"
												aria-label={entryLabel(company)}
												title={company.issue || company.nextHint || company.focus || undefined}
												><span class="sr-only">{entryLabel(company)}</span></button
											>
										</form>
									{/if}
									<span class="portfolio-company-cell">
										<SemanticMark
											meaning={company.tone ?? 'waiting'}
											label={company.name + ': ' + company.status}
										/>
										<span class="portfolio-company-copy">
											<strong>{company.name}</strong>
											<small
												class:portfolio-company-unstartable={!!company.issue}
												title={company.issue || undefined}>{company.status}</small
											>
										</span>
									</span>
									<span class="portfolio-metric portfolio-focus" title={company.focus || undefined}>
										<small class="portfolio-mobile-label">Current focus</small><strong
											>{company.focus || 'Focus unavailable'}</strong
										>
									</span>
									<span class="portfolio-metric portfolio-proof">
										<small class="portfolio-mobile-label">Next</small><strong
											title={company.nextHint || company.issue || undefined}
											>{company.issue || company.next || 'Unavailable'}</strong
										>
									</span>
									<span class="portfolio-metric portfolio-attention">
										<small class="portfolio-mobile-label">Needs you</small>
										<strong class:urgent={!!company.issue || !!company.attentionCount}
											>{company.needsYou ??
												(company.issue
													? 'Fix setup'
													: company.attentionCount == null
														? 'Unavailable'
														: company.attentionCount === 0
															? 'Nothing now'
															: company.attentionCount +
																' item' +
																(company.attentionCount === 1 ? '' : 's'))}</strong
										>
									</span>
									{#if rowActions}<div class="portfolio-row-actions">
											{@render rowActions(company)}
										</div>{/if}
								</div>
							{/each}
						</div>
					</div>
				{:else}
					<div class="portfolio-empty">
						<MatrixGlyph rows={GLYPHS.ring} size={14} />
						{#if empty}{@render empty()}{:else}<h2>No companies yet</h2>{/if}
					</div>
				{/if}
				{#if footer}<footer class="portfolio-footer">{@render footer()}</footer>{/if}
			</section>
		</main>
	{/if}
</AccountShell>

<style>
	.portfolio-main {
		min-height: 0;
		overflow: auto;
		padding: clamp(24px, 3vw, 42px) clamp(18px, 6vw, 86px) 56px;
	}

	.portfolio-head,
	.portfolio-table {
		width: min(1320px, 100%);
		margin-inline: auto;
	}

	.portfolio-head h1 {
		margin: 0;
		font-size: var(--t-hero);
		line-height: 1;
		letter-spacing: -0.04em;
	}

	.portfolio-table {
		margin-top: 26px;
		border-top: 1px solid var(--border-strong);
	}

	.portfolio-table-scroll {
		overflow-x: auto;
	}

	.portfolio-grid {
		min-width: 760px;
	}

	.portfolio-grid-head,
	.portfolio-company-row {
		display: grid;
		grid-template-columns: 24% 27% 34% 15%;
	}

	.portfolio-grid-head > span {
		display: flex;
		min-height: 40px;
		align-items: center;
		padding: 0 15px;
		border-bottom: 1px solid var(--border);
		font-weight: 500;
		color: var(--text-tertiary);
	}

	.portfolio-company-row {
		position: relative;
		text-decoration: none;
		color: var(--ink);
		transition: background var(--motion-state) var(--ease-standard);
	}

	.portfolio-company-row:hover {
		background: var(--wash-hover);
	}

	.portfolio-company-row:focus-visible {
		z-index: 2;
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}

	.portfolio-company-cell,
	.portfolio-metric {
		min-width: 0;
		min-height: 72px;
		border-bottom: 1px solid var(--border);
	}

	.portfolio-company-cell {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 13px 15px;
	}

	.portfolio-company-copy {
		min-width: 0;
		flex: 1;
	}

	.portfolio-company-copy strong,
	.portfolio-company-copy small {
		display: block;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.portfolio-company-copy strong {
		font-size: var(--t-head);
		letter-spacing: -0.02em;
	}

	.portfolio-company-copy small {
		margin-top: 2px;
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}

	.portfolio-company-copy small::first-letter {
		text-transform: uppercase;
	}

	/* A company that cannot start is not merely idle: nothing will happen in it
   until its configuration is resolved. The exact reason is the hover
   explanation, so the row itself stays one short phrase. */
	.portfolio-company-copy small.portfolio-company-unstartable {
		color: var(--text-secondary);
		cursor: help;
	}

	.portfolio-metric {
		display: flex;
		flex-direction: column;
		justify-content: center;
		padding: 11px 15px;
		line-height: 1.4;
	}

	.portfolio-metric small,
	.portfolio-metric strong {
		display: block;
	}

	/* What needs the owner is the one column that should catch the eye. */
	.portfolio-attention strong.urgent {
		color: var(--intent-authority);
		font-weight: 600;
	}

	.portfolio-metric strong {
		font-size: var(--t-body);
		font-weight: 400;
		color: var(--ink);
	}

	.portfolio-metric small {
		margin-top: 2px;
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}

	.portfolio-metric .portfolio-mobile-label {
		display: none;
	}

	.portfolio-focus strong {
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		color: var(--text-secondary);
	}

	.portfolio-loading {
		min-height: 0;
		display: grid;
		place-items: center;
		padding: 24px;
	}

	.portfolio-empty {
		padding: 50px 18px;
		text-align: center;
		color: var(--text-tertiary);
	}

	.portfolio-empty :global(h2) {
		margin: 12px 0 5px;
		font-size: var(--t-head);
		color: var(--ink);
	}

	.portfolio-empty :global(p) {
		margin: 0;
		font-size: var(--t-body);
	}

	.portfolio-entry {
		position: absolute;
		inset: 0;
		z-index: 1;
		width: 100%;
		padding: 0;
		border: 0;
		background: transparent;
		color: inherit;
		text-decoration: none;
		cursor: pointer;
	}
	.portfolio-entry:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	.portfolio-entry-form {
		display: contents;
	}
	.portfolio-row-actions {
		position: relative;
		z-index: 2;
		grid-column: 1 / -1;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 8px;
		padding: 0 15px 12px;
		border-bottom: 1px solid var(--border);
	}
	.portfolio-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		flex-wrap: wrap;
		gap: 12px;
		padding: 16px 15px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	@media (prefers-reduced-motion: reduce) {
		.portfolio-company-row {
			transition: none;
		}
	}
	@media (max-width: 640px) {
		.portfolio-main {
			padding: 30px 14px 42px;
		}
		.portfolio-table-scroll {
			overflow-x: visible;
		}
		.portfolio-grid {
			min-width: 0;
		}
		.portfolio-grid-head {
			display: none;
		}
		.portfolio-company-row {
			grid-template-columns: minmax(0, 1fr);
			padding-bottom: 6px;
		}
		.portfolio-company-cell,
		.portfolio-metric {
			min-height: 0;
		}
		.portfolio-company-cell {
			padding-block: 14px;
		}
		.portfolio-metric {
			padding-block: 8px;
			border-bottom: 0;
		}
		.portfolio-metric .portfolio-mobile-label {
			display: block;
			margin: 0 0 2px;
			font-weight: 500;
		}
	}
</style>
