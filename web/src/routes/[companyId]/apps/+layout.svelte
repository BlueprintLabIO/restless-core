<script lang="ts">
	import { page } from '$app/state';
	import ResizableSidebar from '$lib/components/ResizableSidebar.svelte';
	import SidebarShell from '$lib/ui/views/SidebarShell.svelte';
	import SidebarRow from '$lib/ui/views/SidebarRow.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import Compass from '@lucide/svelte/icons/compass';
	import PackageCheck from '@lucide/svelte/icons/package-check';
	import Inbox from '@lucide/svelte/icons/inbox';
	import ArchiveRestore from '@lucide/svelte/icons/archive-restore';
	import Plus from '@lucide/svelte/icons/plus';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { createAppsData, provideAppsData, viewFrom, isKind } from '$lib/model/apps-data.svelte';

	let { children } = $props();
	const companyId = $derived(page.params.companyId ?? '');
	const data = createAppsData(() => companyId);
	provideAppsData(data);
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(principal.view?.membership_role === 'owner');
	const index = $derived('/' + encodeURIComponent(companyId) + '/apps');
	const current = $derived(
		page.url.pathname.replace(/\/$/, '') === index
			? (viewFrom(page.url.searchParams.get('view')) ?? 'all')
			: null
	);
	const discover = $derived(
		!!current && !['in-use', 'waiting', 'removed'].includes(current) && !isKind(current)
	);
	const installed = $derived(current === 'in-use' || (!!current && isKind(current)));
	const views = $derived([
		{ label: 'Discover', href: index, active: discover },
		{ label: 'Installed', href: index + '?view=in-use', active: installed },
		...(data.waiting
			? [{ label: 'Needs setup', href: index + '?view=waiting', active: current === 'waiting' }]
			: [])
	]);
</script>

{#snippet addActions()}
	{#if owner}
		<ActionMenu label="Add an app">
			{#snippet trigger()}<Plus size={16} aria-hidden="true" />{/snippet}
			<a href={index + '?add=link'}>Add from a link</a>
			<a href={index + '?add=skill'}>Write know-how</a>
			<a href={index + '?view=ask'}>Ask Exec</a>
		</ActionMenu>
	{/if}
{/snippet}

<ResizableSidebar area="apps" label="Resize the Apps sidebar">
	<SidebarShell label="Apps" title="Apps">
		{#snippet action()}{@render addActions()}{/snippet}
		{#snippet nav()}
			<SidebarRow href={index} label="Discover" icon={Compass} active={discover} />
			<SidebarRow
				href={index + '?view=in-use'}
				label="Installed"
				icon={PackageCheck}
				active={installed}
			/>
			{#if data.waiting}
				<SidebarRow
					href={index + '?view=waiting'}
					label="Needs setup"
					icon={Inbox}
					active={current === 'waiting'}
					count={data.waiting}
					todo
				/>
			{/if}
			{#if data.removed.length}
				<SidebarRow
					href={index + '?view=removed'}
					label="Removed"
					icon={ArchiveRestore}
					active={current === 'removed'}
					title="Removed know-how, kept so it can be restored"
				/>
			{/if}
		{/snippet}
		{#snippet narrow()}
			<nav class="apps-views" aria-label="Apps">
				{#each views as view (view.label)}
					<a href={view.href} aria-current={view.active ? 'page' : undefined}>{view.label}</a>
				{/each}
				<span class="add">{@render addActions()}</span>
			</nav>
		{/snippet}
		{@render children()}
	</SidebarShell>
</ResizableSidebar>

<style>
	.apps-views {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 8px 12px;
		border-bottom: 1px solid var(--border);
	}
	.apps-views > a {
		padding: 8px 10px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
		white-space: nowrap;
	}
	.apps-views > a[aria-current='page'] {
		background: var(--surface-raised);
		color: var(--ink);
		box-shadow: var(--shadow-soft);
		font-weight: 600;
	}
	.add {
		margin-left: auto;
	}
</style>
