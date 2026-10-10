<script lang="ts">
	/* Company is a settings area: a titled, searchable section list beside one pane. Each row shows
	 * its area's current value ("Connected", "4 keys"), amber when it waits on the owner and red when
	 * it cannot be read, so the navigation is also a status summary. Where the area is too narrow for
	 * both, the list steps out and every detail page keeps a way back to the overview, which then
	 * lists the sections. */
	import { page } from '$app/state';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import LayoutGrid from '@lucide/svelte/icons/layout-grid';
	import History from '@lucide/svelte/icons/history';
	import HeartPulse from '@lucide/svelte/icons/heart-pulse';
	import { createCompanySetup, provideCompanySetup } from '$lib/model/company-setup-rows.svelte';
	import { COMPANY_PAGES, companyPageHref } from '$lib/model/company-pages';
	import SidebarShell from '$lib/ui/views/SidebarShell.svelte';
	import SidebarGroup from '$lib/ui/views/SidebarGroup.svelte';
	import SidebarRow from '$lib/ui/views/SidebarRow.svelte';
	import SidebarSearch from '$lib/ui/views/SidebarSearch.svelte';
	import type { SidebarTone } from '$lib/ui/views/SidebarRow.svelte';

	let { children } = $props();
	let query = $state('');
	const companyId = $derived(page.params.companyId ?? 'aris');
	const overview = $derived(`/${companyId}/company`);
	const detail = $derived(page.url.pathname !== overview);
	/* The desktop and the Company browser take the area's full width; the section
	 * list steps aside for them. */
	const desktopFocus = $derived(
		page.url.pathname === `/${companyId}/company/computer` &&
			['desktop', 'browser'].includes(page.url.searchParams.get('focus') ?? '')
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
		reading?: string;
		tone?: SidebarTone;
	};
	const tone = (todo: number, unavailable: boolean): SidebarTone =>
		unavailable ? 'block' : todo ? 'wait' : '';
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
							tooltip: row.todo && row.task ? row.task : row.value,
							reading: row.value,
							tone: tone(row.todo, row.unavailable)
						}
					: {
							key: target.key,
							label: target.label,
							href: companyPageHref(companyId, target),
							icon: record!.icon,
							tooltip:
								target.key === 'health'
									? 'Checks on the company computer, model and tools'
									: 'Decisions, receipts and external actions',
							...(target.key === 'health' && setup.health
								? setup.health === 'healthy'
									? { reading: 'Healthy', tone: 'work' as const }
									: { reading: 'Needs a look', tone: 'block' as const }
								: {})
						}
			);
		}
		const q = query.trim().toLocaleLowerCase();
		if (!q) return out;
		return out
			.map((group) => ({
				...group,
				rows: group.rows.filter((row) =>
					`${group.label} ${row.label} ${row.reading ?? ''}`.toLocaleLowerCase().includes(q)
				)
			}))
			.filter((group) => group.rows.length);
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
			const edit = [...document.querySelectorAll<HTMLButtonElement>('.sidebar-main button')].find(
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
	<SidebarShell label="Company sections" title="Company">
		{#snippet top()}
			<SidebarSearch bind:value={query} placeholder="Search settings" />
		{/snippet}
		{#snippet nav()}
			{#if !query.trim()}<SidebarRow
					href={overview}
					label="Overview"
					icon={LayoutGrid}
					active={active(overview)}
					reading={setup.todos.length ? `${setup.todos.length} to finish` : 'All set'}
					readingTone={setup.todos.length ? 'wait' : 'work'}
					title={setup.todos.length ? `${setup.todos.length} to finish setting up` : undefined}
				/>{/if}
			{#each groups as group (group.label)}
				<SidebarGroup label={group.label}>
					{#each group.rows as row (row.key)}
						<SidebarRow
							href={row.href}
							label={row.label}
							icon={row.icon}
							active={active(row.href)}
							reading={row.reading}
							readingTone={row.tone}
							title={row.tooltip}
						/>
					{/each}
				</SidebarGroup>
			{:else}
				<p class="company-none">No settings match.</p>
			{/each}
		{/snippet}
		{#snippet narrow()}
			{#if detail}<a class="company-back" href={overview}
					><ArrowLeft size={14} strokeWidth={1.8} aria-hidden="true" />Company</a
				>{/if}
		{/snippet}
		{@render children()}
	</SidebarShell>
{/if}

<style>
	.company-focus {
		position: relative;
		display: flex;
		flex: 1 1 auto;
		width: 100%;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
	}
	/* Too narrow for the list: every detail page keeps a way back to the overview, which then
	 * lists the sections. */
	.company-back {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		margin: 10px 0 -14px 10px;
		padding: 4px 8px;
		border-radius: var(--radius-control);
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.company-none {
		margin: 8px;
		color: var(--text-tertiary);
	}
	.company-back:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
</style>
