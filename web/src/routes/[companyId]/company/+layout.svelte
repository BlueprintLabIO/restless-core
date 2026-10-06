<script lang="ts">
	/* Company is a settings area, as Linear's is: a quiet section list beside one pane. Each row
	 * carries what waits on the owner there as a count, so setup reads as a short to-do list in the
	 * navigation itself. Where the area is too narrow for both, the list steps out and every detail
	 * page keeps a way back to the overview, which then lists the sections. */
	import { page } from '$app/state';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import LayoutGrid from '@lucide/svelte/icons/layout-grid';
	import History from '@lucide/svelte/icons/history';
	import HeartPulse from '@lucide/svelte/icons/heart-pulse';
	import { createCompanySetup, provideCompanySetup } from '$lib/model/company-setup-rows.svelte';
	import { COMPANY_PAGES, companyPageHref } from '$lib/model/company-pages';

	let { children } = $props();
	const companyId = $derived(page.params.companyId ?? 'aris');
	const overview = $derived(`/${companyId}/company`);
	const detail = $derived(page.url.pathname !== overview);
	const desktopFocus = $derived(
		page.url.pathname === `/${companyId}/company/computer` &&
			page.url.searchParams.get('focus') === 'desktop'
	);
	const setup = createCompanySetup(() => companyId);
	provideCompanySetup(setup);

	/* The navigation: the overview, the setup rows in the registry's groups, then the records. */
	type NavRow = {
		key: string;
		label: string;
		href: string;
		icon: (typeof setup.rows)[number]['icon'];
		todo?: number;
		problem?: boolean;
		tooltip?: string;
	};
	const groups = $derived.by(() => {
		const byKey = new Map(setup.rows.map((row) => [row.key, row]));
		const out: { label: string; rows: NavRow[] }[] = [];
		for (const target of COMPANY_PAGES) {
			if (target.key === 'overview') continue;
			const row = byKey.get(target.key);
			const record =
				target.key === 'activity'
					? { icon: History }
					: target.key === 'health'
						? { icon: HeartPulse }
						: null;
			if (!row && !(record && setup.owner)) continue;
			let group = out.find((candidate) => candidate.label === target.group);
			if (!group) out.push((group = { label: target.group, rows: [] }));
			group.rows.push(
				row
					? {
							key: row.key,
							label: row.label,
							href: row.href,
							icon: row.icon,
							todo: row.todo,
							problem: row.unavailable,
							tooltip: row.value
						}
					: {
							key: target.key,
							label: target.label,
							href: companyPageHref(companyId, target),
							icon: record!.icon,
							problem: target.key === 'health' && !!setup.health && setup.health !== 'healthy',
							tooltip:
								target.key === 'health'
									? 'Checks on the company computer, model and tools'
									: 'Decisions, receipts and external actions'
						}
			);
		}
		return out;
	});
	const active = (href: string) =>
		href === overview ? page.url.pathname === overview : page.url.pathname.startsWith(href);

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
	<div class="company-shell">
		<nav class="company-nav" aria-label="Company sections">
			<a
				class="company-nav-row"
				class:active={active(overview)}
				href={overview}
				aria-current={active(overview) ? 'page' : undefined}
				><LayoutGrid size={15} strokeWidth={1.8} aria-hidden="true" /><span>Overview</span
				>{#if setup.todos.length}<b
						class="company-nav-count"
						title={`${setup.todos.length} to finish setting up`}>{setup.todos.length}</b
					>{/if}</a
			>
			{#each groups as group (group.label)}
				<div class="company-nav-group" role="group" aria-label={group.label}>
					<span class="company-nav-label">{group.label}</span>
					{#each group.rows as row (row.key)}
						{@const Icon = row.icon}
						<a
							class="company-nav-row"
							class:active={active(row.href)}
							href={row.href}
							title={row.tooltip}
							aria-current={active(row.href) ? 'page' : undefined}
							><Icon size={15} strokeWidth={1.8} aria-hidden="true" /><span>{row.label}</span
							>{#if row.todo}<b class="company-nav-count" aria-label={`${row.todo} to do`}
									>{row.todo}</b
								>{:else if row.problem}<i class="company-nav-problem" aria-label="Needs attention"
								></i>{/if}</a
						>
					{/each}
				</div>
			{/each}
		</nav>
		<main class="company-main">
			{#if detail}<a class="company-back" href={overview}
					><ArrowLeft size={14} strokeWidth={1.8} aria-hidden="true" />Company</a
				>{/if}
			{@render children()}
		</main>
	</div>
{/if}

<style>
	.company-shell {
		container: company / inline-size;
		display: flex;
		flex: 1 1 auto;
		gap: 8px;
		width: 100%;
		min-width: 0;
		min-height: 0;
	}
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

	/* The section list: Linear's settings sidebar, 13px rows at 30px, quiet group names. */
	.company-nav {
		display: grid;
		flex: none;
		align-content: start;
		gap: 1px;
		width: 208px;
		padding: 6px 4px;
		overflow-y: auto;
	}
	.company-nav-group {
		display: grid;
		gap: 1px;
		margin-top: 12px;
	}
	.company-nav-label {
		padding: 0 8px 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
	}
	.company-nav-row {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 30px;
		padding: 0 8px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
		white-space: nowrap;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.company-nav-row > span {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.company-nav-row :global(svg) {
		flex: none;
		color: var(--text-tertiary);
	}
	.company-nav-row:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.company-nav-row.active {
		background: var(--wash-active, var(--wash-hover));
		color: var(--ink);
		font-weight: 500;
	}
	.company-nav-row:hover :global(svg),
	.company-nav-row.active :global(svg) {
		color: var(--ink);
	}
	.company-nav-row:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	.company-nav-count {
		flex: none;
		min-width: 18px;
		padding: 0 5px;
		border-radius: 9px;
		background: color-mix(in srgb, var(--surface-attention) 22%, transparent);
		color: var(--ink);
		font-size: var(--t-label);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
		line-height: 18px;
		text-align: center;
	}
	.company-nav-problem {
		flex: none;
		width: 6px;
		height: 6px;
		margin-right: 6px;
		border-radius: 50%;
		background: var(--state-danger);
	}

	.company-back {
		display: none;
	}
	.company-back:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}

	/* Too narrow for both: the list steps out; the overview lists the sections instead, and each
	 * detail page carries a way back to it. */
	@container company (max-width: 760px) {
		.company-nav {
			display: none;
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
	}
	@media (max-width: 760px) {
		.company-back {
			position: static;
			align-self: flex-start;
			margin: 10px 0 -14px 10px;
		}
	}
</style>
