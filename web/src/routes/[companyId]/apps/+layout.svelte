<script lang="ts">
	/* Apps, ask first: the sidebar is where power users go straight to something. Ask (the
	 * default), what waits on the owner, what is in use by kind, and everything to add by category.
	 * Counts come from the same read as the page, so they agree. */
	import { page } from '$app/state';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Inbox from '@lucide/svelte/icons/inbox';
	import Globe2 from '@lucide/svelte/icons/cloud';
	import Terminal from '@lucide/svelte/icons/terminal';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import Package from '@lucide/svelte/icons/package';
	import Link2 from '@lucide/svelte/icons/link-2';
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
	import {
		APP_KINDS,
		BROWSE_CATEGORIES,
		KIND_INFO,
		KIND_SECTION,
		type AppCategory,
		type AppKind
	} from '$lib/model/apps';
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
	const KIND_ICONS: Record<AppKind, typeof LayoutGrid> = {
		service: Globe2,
		local: Terminal,
		skill: BookOpen,
		bundle: Package
	};

	const index = $derived(`/${encodeURIComponent(companyId)}/apps`);
	const href = (view: AppsView) => (view === 'ask' ? index : `${index}?view=${viewKey(view)}`);
	/* Only the index shows a view; an app's own page leaves the list without a selection. */
	const current = $derived<AppsView | null>(
		page.url.pathname.replace(/\/$/, '') === index
			? (viewFrom(page.url.searchParams.get('view')) ?? 'ask')
			: null
	);
	const kinds = $derived(
		APP_KINDS.map((kind) => ({ kind, count: data.ofKind(kind).length })).filter(
			(row) => row.count > 0 || row.kind === 'service'
		)
	);
	const categories = $derived(
		BROWSE_CATEGORIES.map((category: AppCategory) => ({
			view: category,
			count: data.inCategory(category).length
		}))
	);
	/* The phone row: the same places, fewest first. */
	const narrowViews = $derived<{ view: AppsView; label: string }[]>([
		{ view: 'ask', label: 'Ask' },
		...(data.waiting ? [{ view: 'waiting' as const, label: 'Waiting on you' }] : []),
		{ view: 'in-use', label: 'In use' },
		{ view: 'all', label: 'Browse' }
	]);
</script>

<SidebarShell label="Apps">
	{#snippet nav()}
		<SidebarRow
			href={href('ask')}
			label="Ask"
			icon={Sparkles}
			active={current === 'ask'}
			title="Say what the company should be able to do, or search your apps (press /)"
		/>
		<SidebarRow
			href={href('waiting')}
			label="Waiting on you"
			icon={Inbox}
			active={current === 'waiting'}
			count={data.waiting}
			todo={!!data.waiting}
			title="Apps Exec asked for, and ones that need a sign-in or a decision"
		/>
		<SidebarGroup label="In use">
			<SidebarRow
				href={href('in-use')}
				label="All in use"
				icon={LayoutGrid}
				active={current === 'in-use'}
				count={data.inUse.length}
				title="Everything the company can use, with what it may do"
			/>
			{#each kinds as row (row.kind)}
				<SidebarRow
					href={href(row.kind)}
					label={KIND_SECTION[row.kind]}
					icon={KIND_ICONS[row.kind]}
					active={current === row.kind}
					count={row.count}
					title={KIND_INFO[row.kind]}
				/>
			{/each}
		</SidebarGroup>
		<SidebarGroup label="Browse">
			<SidebarRow
				href={href('all')}
				label="All apps"
				icon={LayoutGrid}
				active={current === 'all'}
				count={data.browse.length}
			/>
			{#each categories as row (row.view)}
				<SidebarRow
					href={href(row.view)}
					label={row.view}
					icon={ICONS[row.view]}
					active={current === row.view}
					count={row.count}
				/>
			{/each}
		</SidebarGroup>
		<SidebarGroup label="More">
			<SidebarRow
				href={`${index}?add=link`}
				label="Add from a link"
				icon={Link2}
				title="Paste a service's MCP address, a plugin or skill on GitHub, or a command"
			/>
		</SidebarGroup>
	{/snippet}
	{#snippet narrow()}
		<div class="views" role="group" aria-label="Apps">
			{#each narrowViews as row (row.view)}
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
