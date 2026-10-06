<script lang="ts">
	/* Apps is a settings area too: what the company uses, then everything it could add by
	 * category, beside one pane. Counts come from the same read as the page, so they agree. */
	import { page } from '$app/state';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import LayoutGrid from '@lucide/svelte/icons/layout-grid';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import Kanban from '@lucide/svelte/icons/kanban';
	import Code from '@lucide/svelte/icons/code';
	import Wallet from '@lucide/svelte/icons/wallet';
	import TrendingUp from '@lucide/svelte/icons/trending-up';
	import Palette from '@lucide/svelte/icons/palette';
	import FileText from '@lucide/svelte/icons/file-text';
	import Globe from '@lucide/svelte/icons/globe';
	import SidebarShell from '$lib/ui/views/SidebarShell.svelte';
	import SidebarGroup from '$lib/ui/views/SidebarGroup.svelte';
	import SidebarRow from '$lib/ui/views/SidebarRow.svelte';
	import { BROWSE_CATEGORIES, type AppCategory } from '$lib/model/apps';
	import {
		createAppsData,
		provideAppsData,
		viewFrom,
		viewKey,
		type AppsView
	} from '$lib/model/apps-data.svelte';

	let { children } = $props();
	const companyId = $derived(page.params.companyId ?? 'aris');
	const data = createAppsData(() => companyId);
	provideAppsData(data);

	const ICONS: Record<string, typeof LayoutGrid> = {
		Communication: MessageSquare,
		Projects: Kanban,
		Engineering: Code,
		Finance: Wallet,
		Sales: TrendingUp,
		Design: Palette,
		Documents: FileText,
		Websites: Globe
	};

	const index = $derived(`/${encodeURIComponent(companyId)}/apps`);
	const href = (view: AppsView) => `${index}?view=${viewKey(view)}`;
	/* Only the index shows a view; an app's own page leaves the list without a selection. */
	const current = $derived<AppsView | null>(
		page.url.pathname.replace(/\/$/, '') === index
			? (viewFrom(page.url.searchParams.get('view')) ??
					(data.waiting || data.added.length ? 'in-use' : 'all'))
			: null
	);
	const views = $derived<
		{ view: AppsView; label: string; count: number; icon: typeof LayoutGrid }[]
	>([
		{
			view: 'in-use',
			label: 'In use',
			count: data.added.length + data.included.length,
			icon: CircleCheck
		},
		{ view: 'all', label: 'All apps', count: data.browse.length, icon: LayoutGrid },
		...BROWSE_CATEGORIES.map((category: AppCategory) => ({
			view: category,
			label: category,
			count: data.inCategory(category).length,
			icon: ICONS[category]
		}))
	]);
</script>

<SidebarShell label="Apps">
	{#snippet nav()}
		<SidebarRow
			href={href('in-use')}
			label="In use"
			icon={CircleCheck}
			active={current === 'in-use'}
			count={data.waiting || data.added.length + data.included.length}
			todo={!!data.waiting}
			title={data.waiting
				? `${data.waiting} waiting on you`
				: `${data.added.length + data.included.length} in use`}
		/>
		<SidebarGroup label="Add">
			{#each views.slice(1) as row (row.view)}
				<SidebarRow
					href={href(row.view)}
					label={row.label}
					icon={row.icon}
					active={current === row.view}
					count={row.count}
				/>
			{/each}
		</SidebarGroup>
	{/snippet}
	{#snippet narrow()}
		<div class="views" role="group" aria-label="Apps">
			{#each views as row (row.view)}
				<a
					class="view"
					href={href(row.view)}
					aria-current={current === row.view ? 'page' : undefined}>{row.label}</a
				>
			{/each}
		</div>
	{/snippet}
	{@render children()}
</SidebarShell>

<style>
	/* Too narrow for the list: the same views as one row that scrolls sideways, its edge fading
	 * so the rest read as more rather than cut off. */
	.views {
		display: flex;
		gap: 6px;
		padding: 12px 16px 0;
		overflow-x: auto;
		scrollbar-width: none;
		mask-image: linear-gradient(to right, black 85%, transparent);
	}
	.views::-webkit-scrollbar {
		display: none;
	}
	.view {
		display: inline-flex;
		flex: none;
		align-items: center;
		height: 28px;
		padding: 0 11px;
		border: 1px solid var(--border);
		border-radius: 14px;
		background: var(--surface);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
	}
	.view[aria-current='page'] {
		background: var(--accent-strong);
		border-color: var(--accent-strong);
		color: var(--text-inverse);
	}
</style>
