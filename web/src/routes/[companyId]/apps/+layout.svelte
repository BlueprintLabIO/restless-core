<script lang="ts">
	import { page } from '$app/state';
	import ResizableSidebar from '$lib/components/ResizableSidebar.svelte';
	import SidebarShell from '$lib/ui/views/SidebarShell.svelte';
	import SidebarRow from '$lib/ui/views/SidebarRow.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import Compass from '@lucide/svelte/icons/compass';
	import PackageCheck from '@lucide/svelte/icons/package-check';
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
	const current = $derived.by(() => {
		if (page.url.pathname.replace(/\/$/, '') === index)
			return viewFrom(page.url.searchParams.get('view')) ?? 'all';
		// Details retain the selected list's navigation context; foreign/from-another-company URLs do not.
		const from = page.url.searchParams.get('from');
		if (!from) return null;
		try {
			const list = new URL(from, page.url.origin);
			return list.origin === page.url.origin && list.pathname === index
				? (viewFrom(list.searchParams.get('view')) ?? 'all')
				: null;
		} catch {
			return null;
		}
	});
	const discover = $derived(
		!!current &&
			!['in-use', 'installed-apps', 'installed-know-how', 'waiting', 'removed'].includes(current) &&
			!isKind(current)
	);
	const installed = $derived(
		!!current &&
			(['in-use', 'installed-apps', 'installed-know-how', 'waiting'].includes(current) ||
				isKind(current))
	);
	const installedViews = $derived([
		{ label: 'All', href: index + '?view=in-use', active: current === 'in-use' },
		...(data.inUse.some((app) => app.kind !== 'skill')
			? [
					{
						label: 'Apps',
						href: index + '?view=installed-apps',
						active:
							current === 'installed-apps' || (!!current && isKind(current) && current !== 'skill')
					}
				]
			: []),
		...(data.inUse.some((app) => app.skills.length > 0)
			? [
					{
						label: 'Know-how',
						href: index + '?view=installed-know-how',
						active: current === 'installed-know-how' || current === 'skill'
					}
				]
			: []),
		...(data.waiting
			? [{ label: 'Needs setup', href: index + '?view=waiting', active: current === 'waiting' }]
			: [])
	]);
	const views = $derived([
		{ label: 'Discover', href: index, active: discover },
		{ label: 'Installed', href: index + '?view=in-use', active: installed },
		...(data.removed.length
			? [{ label: 'Removed', href: index + '?view=removed', active: current === 'removed' }]
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
			<section class="installed-group" aria-labelledby="apps-installed-label">
				<h2 id="apps-installed-label">
					<PackageCheck size={16} aria-hidden="true" /><span>Installed</span>
				</h2>
				<nav class="installed-views" aria-label="Installed">
					{#each installedViews as view (view.label)}
						<SidebarRow href={view.href} label={view.label} active={view.active} />
					{/each}
				</nav>
			</section>
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
			<nav class="apps-subviews" aria-label="Installed">
				{#each installedViews as view (view.label)}<a
						href={view.href}
						aria-current={view.active ? 'page' : undefined}
						>{view.label === 'All' ? 'All installed' : view.label}</a
					>{/each}
			</nav>
		{/snippet}
		{@render children()}
	</SidebarShell>
</ResizableSidebar>

<style>
	.installed-group h2 {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
		color: var(--text-secondary);
		font-size: var(--t-body);
		font-weight: 500;
		margin: 0;
	}
	.installed-views {
		padding-left: 24px;
	}
	.apps-subviews {
		display: flex;
		gap: 4px;
		overflow-x: auto;
		padding: 6px 12px;
		border-bottom: 1px solid var(--border);
	}
	.apps-subviews a {
		flex: none;
		padding: 7px 10px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		text-decoration: none;
		font-size: var(--t-label);
	}
	.apps-subviews a[aria-current='page'] {
		background: var(--wash-hover);
		color: var(--ink);
		font-weight: 600;
	}

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
