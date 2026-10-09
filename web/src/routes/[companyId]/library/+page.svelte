<script lang="ts">
	/* The Library: what the company has written and built, in one place. Docs
	 * and sheets share one table, filtered from the side; each opens in its
	 * editor. A new item is made untitled and opens straight into its editor. */
	import { page } from '$app/state';
	import { Item, Notice, Empty, Dot } from '$lib/ui/page';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import SidebarShell from '$lib/ui/views/SidebarShell.svelte';
	import SidebarRow from '$lib/ui/views/SidebarRow.svelte';
	import SidebarGroup from '$lib/ui/views/SidebarGroup.svelte';
	import { personName } from '$lib/model/initials';
	import LibraryNew from '$lib/components/LibraryNew.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FileText from '@lucide/svelte/icons/file-text';
	import Sheet from '@lucide/svelte/icons/sheet';
	import Library from '@lucide/svelte/icons/library';
	import Plus from '@lucide/svelte/icons/plus';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { documentsQuery } from '$lib/model/document-queries.svelte';
	import { sheetJson, type SheetRow } from '$lib/model/sheets';
	import { getDocuments, type DocumentSummary } from '$lib/model/documents';
	import ArchiveIcon from '@lucide/svelte/icons/archive';
	import {
		attentionQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		companyPrincipalQuery
	} from '$lib/model/queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const root = $derived(`/${encodeURIComponent(companyId)}`);
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(principal.view?.membership_role === 'owner');
	const cockpit = $derived(cockpitQuery(companyId, () => owner));
	const collaboration = $derived(
		collaborationBootstrapQuery(companyId, () => (owner ? null : principal.view))
	);
	const attention = $derived(attentionQuery(companyId, () => owner));
	const documents = documentsQuery(() => companyId);
	const people = $derived(
		owner ? (cockpit.view?.people ?? []) : (collaboration.view?.people ?? [])
	);

	let sheets = $state<SheetRow[]>([]);
	let sheetsFailure = $state('');
	let sheetsLoaded = $state(false);
	/* Loads once per company: a refreshed identity must not blank the list back to loading. */
	let sheetsFor = '';
	$effect(() => {
		const target = companyId;
		if (!principal.view || sheetsFor === target) return;
		sheetsFor = target;
		sheetsLoaded = false;
		sheetsFailure = '';
		void sheetJson<SheetRow[]>(target)
			.then((rows) => {
				if (target === companyId) sheets = rows;
			})
			.catch((cause) => {
				if (target === companyId)
					sheetsFailure = cause instanceof Error ? cause.message : 'Sheets are unavailable';
			})
			.finally(() => {
				if (target === companyId) sheetsLoaded = true;
			});
	});

	/* Archived: out of every list, one view in the sidebar, restorable. */
	let archivedDocs = $state<DocumentSummary[]>([]);
	let archivedSheets = $state<SheetRow[]>([]);
	let archiveBusy = $state('');
	async function loadArchived() {
		const target = companyId;
		const [docs, sheetRows] = await Promise.all([
			getDocuments(target, null, 50, true).catch(() => null),
			sheetJson<SheetRow[]>(target, '-archived').catch(() => null)
		]);
		if (target !== companyId) return;
		if (docs) archivedDocs = docs.items.filter((item) => item.document.status === 'archived');
		if (sheetRows) archivedSheets = sheetRows;
	}
	$effect(() => {
		if (principal.view) void loadArchived();
	});
	async function setArchived(entry: { id: string; kind: 'doc' | 'sheet' }, archived: boolean) {
		if (archiveBusy) return;
		archiveBusy = entry.id;
		try {
			const path =
				entry.kind === 'doc'
					? `/api/companies/${encodeURIComponent(companyId)}/documents/${encodeURIComponent(entry.id)}/archive`
					: `/api/companies/${encodeURIComponent(companyId)}/sheets/${encodeURIComponent(entry.id)}/archive`;
			const response = await fetch(path, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				credentials: 'same-origin',
				body: JSON.stringify({ archived })
			});
			if (!response.ok) return;
			if (entry.kind === 'sheet') {
				sheets = await sheetJson<SheetRow[]>(companyId).catch(() => sheets);
			} else {
				await documents.refresh();
			}
			await loadArchived();
		} finally {
			archiveBusy = '';
		}
	}

	type Filter = 'all' | 'docs' | 'sheets' | 'review' | 'archived';
	const filter = $derived((page.url.searchParams.get('show') as Filter) || 'all');
	let search = $state('');
	let creator: LibraryNew | undefined = $state();
	$effect(() => {
		const create = page.url.searchParams.get('create');
		if (!creator || (create !== 'doc' && create !== 'sheet')) return;
		creator.open(create);
		const url = new URL(page.url);
		url.searchParams.delete('create');
		history.replaceState(history.state, '', url);
	});

	/* Documents an owner was asked to review or work on, from the Inbox. */
	const requested = $derived(
		new Set(
			(attention.view?.items ?? [])
				.map((item) => item.nativeDocument?.document_id)
				.filter((id): id is string => !!id)
		)
	);
	type Entry = {
		id: string;
		kind: 'doc' | 'sheet';
		title: string;
		href: string;
		updatedAt: string;
		owner: string;
		status: string;
		review: boolean;
	};
	const entries = $derived<Entry[]>(
		[
			...documents.summaries
				.filter((summary) => summary.document.status !== 'archived')
				.map((summary) => ({
					id: summary.document.id,
					kind: 'doc' as const,
					title: summary.document.title || 'Untitled document',
					href: `${root}/library/documents?document=${encodeURIComponent(summary.document.id)}`,
					updatedAt: summary.document.updated_at,
					owner: name(summary.document.owner_actor_id),
					status: summary.document.status,
					review: summary.document.status === 'in_review' || requested.has(summary.document.id)
				})),
			...sheets.map((sheet) => ({
				id: sheet.id,
				kind: 'sheet' as const,
				title: sheet.title || 'Untitled sheet',
				href: `${root}/library/sheets?sheet=${encodeURIComponent(sheet.id)}`,
				updatedAt: sheet.updated_at,
				owner: name(sheet.owner_actor_id),
				status: '',
				review: false
			}))
		].sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt))
	);
	const archivedEntries = $derived<Entry[]>(
		[
			...archivedDocs.map((summary) => ({
				id: summary.document.id,
				kind: 'doc' as const,
				title: summary.document.title || 'Untitled document',
				href: `${root}/library/documents?document=${encodeURIComponent(summary.document.id)}`,
				updatedAt: summary.document.updated_at,
				owner: name(summary.document.owner_actor_id),
				status: '',
				review: false
			})),
			...archivedSheets.map((sheet) => ({
				id: sheet.id,
				kind: 'sheet' as const,
				title: sheet.title || 'Untitled sheet',
				href: `${root}/library/sheets?sheet=${encodeURIComponent(sheet.id)}`,
				updatedAt: sheet.updated_at,
				owner: name(sheet.owner_actor_id),
				status: '',
				review: false
			}))
		].sort((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt))
	);
	const counts = $derived({
		all: entries.length,
		docs: entries.filter((entry) => entry.kind === 'doc').length,
		sheets: entries.filter((entry) => entry.kind === 'sheet').length,
		review: entries.filter((entry) => entry.review).length,
		archived: archivedEntries.length
	});
	const shown = $derived(
		(filter === 'archived' ? archivedEntries : entries).filter(
			(entry) =>
				(filter === 'all' ||
					filter === 'archived' ||
					(filter === 'docs' && entry.kind === 'doc') ||
					(filter === 'sheets' && entry.kind === 'sheet') ||
					(filter === 'review' && entry.review)) &&
				(!search.trim() || entry.title.toLowerCase().includes(search.trim().toLowerCase()))
		)
	);
	const loading = $derived(documents.status === 'unknown' || !sheetsLoaded);
	const FILTERS: { key: Filter; label: string; icon: typeof Library }[] = [
		{ key: 'all', label: 'All files', icon: Library },
		{ key: 'docs', label: 'Documents', icon: FileText },
		{ key: 'sheets', label: 'Sheets', icon: Sheet },
		{ key: 'archived', label: 'Archived', icon: ArchiveIcon }
	];
	const filterLabel = (key: Filter) =>
		key === 'review'
			? 'Needs your review'
			: (FILTERS.find((item) => item.key === key)?.label ?? 'All files');
	/* Each type's section shows its three most recent; the title opens the rest. */
	const RECENT = 3;
	const recentDocs = $derived(entries.filter((entry) => entry.kind === 'doc').slice(0, RECENT));
	const recentSheets = $derived(entries.filter((entry) => entry.kind === 'sheet').slice(0, RECENT));

	const filterHref = (key: Filter) =>
		key === 'all' ? `${root}/library` : `${root}/library?show=${key}`;

	function name(actorId: string): string {
		if (actorId === 'owner') return 'You';
		return (
			personName(people.find((person) => person.actor_id === actorId)?.display ?? '') ||
			actorId.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
		);
	}
	const statusLabel = (status: string) =>
		({ draft: 'Draft', in_review: 'In review', accepted: 'Accepted' })[status] ?? '';
	const statusTone = (status: string) =>
		status === 'in_review' ? 'warning' : status === 'accepted' ? 'success' : 'muted';
</script>

<CompanyTitle title="Library" {companyId} />

{#snippet typeSection(
	key: 'docs' | 'sheets',
	label: string,
	icon: typeof Library,
	recent: Entry[],
	total: number
)}
	<SidebarGroup
		{label}
		href={filterHref(key)}
		active={filter === key}
		count={total || null}
		action={{
			label: key === 'docs' ? 'New document' : 'New sheet',
			icon: Plus,
			onclick: () => creator?.open(key === 'docs' ? 'doc' : 'sheet')
		}}
	>
		{#each recent as entry (entry.id)}
			<SidebarRow
				href={entry.href}
				label={entry.title}
				{icon}
				dot={entry.review ? 'Waiting on your review' : undefined}
				title={`${entry.title} · ${entry.owner}`}
			/>
		{/each}
		{#if total > recent.length}<SidebarRow
				href={filterHref(key)}
				label={`${total - recent.length} more`}
				quiet
			/>{/if}
	</SidebarGroup>
{/snippet}

<!-- The same sidebar as Company and Apps: one way to move between views of an area. -->
<SidebarShell label="Library">
	{#snippet nav()}
		<SidebarRow
			href={filterHref('all')}
			label="All files"
			icon={Library}
			active={filter === 'all'}
			count={counts.all || null}
		/>
		{@render typeSection('docs', 'Documents', FileText, recentDocs, counts.docs)}
		{@render typeSection('sheets', 'Sheets', Sheet, recentSheets, counts.sheets)}
	{/snippet}
	{#snippet foot()}
		<SidebarRow
			href={filterHref('archived')}
			label="Archived"
			icon={ArchiveIcon}
			active={filter === 'archived'}
			count={counts.archived || null}
			title="Documents and sheets taken out of every list; open one to restore it"
		/>
	{/snippet}
	{#snippet narrow()}
		<nav class="library-filters" aria-label="Library">
			{#each FILTERS as item (item.key)}
				<a
					href={filterHref(item.key)}
					class:active={filter === item.key}
					aria-current={filter === item.key ? 'page' : undefined}>{item.label}</a
				>
			{/each}
		</nav>
	{/snippet}
	<div class="page">
		<header class="head">
			<h1>{filterLabel(filter)}</h1>
			<span class="head-count">{shown.length || ''}</span>
			<span class="spacer"></span>
			<input
				class="search"
				type="search"
				bind:value={search}
				placeholder="Search"
				aria-label="Search the Library"
			/>
			<!-- One action, the one this view is about; All offers both behind one button. -->
			{#if filter === 'docs' || filter === 'sheets'}
				{@const kind = filter === 'docs' ? 'doc' : 'sheet'}
				<button
					class="btn small primary new"
					type="button"
					title={kind === 'doc' ? 'Start an untitled document' : 'Start an untitled sheet'}
					onclick={() => creator?.open(kind)}
					><Plus size={14} strokeWidth={2} aria-hidden="true" /><span
						>{kind === 'doc' ? 'New document' : 'New sheet'}</span
					></button
				>
			{:else}
				<div class="new-menu">
					<ActionMenu label="New">
						{#snippet trigger()}<span class="new-trigger"
								><Plus size={14} strokeWidth={2} aria-hidden="true" /><span>New</span><ChevronDown
									size={13}
									strokeWidth={2}
									aria-hidden="true"
								/></span
							>{/snippet}
						<button type="button" onclick={() => creator?.open('doc')}
							><FileText size={14} strokeWidth={1.8} aria-hidden="true" />Document</button
						>
						<button type="button" onclick={() => creator?.open('sheet')}
							><Sheet size={14} strokeWidth={1.8} aria-hidden="true" />Sheet</button
						>
					</ActionMenu>
				</div>
			{/if}
		</header>

		<div class="body">
			{#if documents.failure}<Notice
					tone="danger"
					title="Documents could not be read"
					details={documents.failure.message}
				>
					{#snippet actions()}<button class="btn small" onclick={() => documents.refresh()}
							>Retry</button
						>{/snippet}
				</Notice>{/if}
			{#if sheetsFailure}<Notice
					tone="warning"
					title="Sheets could not be read"
					details={sheetsFailure}
				/>{/if}
			{#if loading && !entries.length}
				<Skeleton label="Loading the Library" variant="list" count={5} />
			{:else if shown.length}
				<div class="table" role="list">
					<div class="table-head" aria-hidden="true">
						<span>Name</span><span>Owner</span><span>Updated</span>
					</div>
					{#each shown as entry (entry.kind + entry.id)}
						<Item title={entry.title} meta={entry.owner} href={entry.href} unread={entry.review}>
							{#snippet leading()}{#if entry.kind === 'doc'}<FileText
										size={15}
										strokeWidth={1.8}
										aria-label="Document"
									/>{:else}<Sheet size={15} strokeWidth={1.8} aria-label="Sheet" />{/if}{/snippet}
							{#snippet trailing()}
								{#if entry.review}<span class="review">Needs review</span
									>{:else if statusLabel(entry.status)}<Dot
										show
										tone={statusTone(entry.status)}
										label={statusLabel(entry.status)}
									/>{/if}
								<RelativeTime value={entry.updatedAt} />
							{/snippet}
							{#snippet actions()}
								<ActionMenu label={`Actions for ${entry.title}`}>
									<a href={entry.href}>Open</a>
									{#if filter === 'archived'}<button
											disabled={!!archiveBusy}
											onclick={() => void setArchived(entry, false)}>Restore</button
										>{:else}<button
											disabled={!!archiveBusy}
											title="Out of the Library lists, kept under Archived; restore it any time."
											onclick={() => void setArchived(entry, true)}>Archive</button
										>{/if}
								</ActionMenu>
							{/snippet}
						</Item>
					{/each}
					{#if documents.hasMore && filter !== 'sheets'}<button
							class="more"
							disabled={documents.loadingMore}
							onclick={() => documents.loadMore()}
							>{documents.loadingMore ? 'Loading…' : 'Load more documents'}</button
						>{/if}
				</div>
			{:else}
				<Empty
					title={search
						? 'Nothing matches that search'
						: filter === 'review'
							? 'Nothing is waiting for your review'
							: filter === 'archived'
								? 'Nothing is archived'
								: 'Nothing here yet'}
					info="Documents and sheets the company writes appear here. Agents add to it as they work."
				>
					{#snippet action()}{#if !search && filter !== 'review' && filter !== 'archived'}<button
								class="btn small"
								type="button"
								onclick={() => creator?.open(filter === 'sheets' ? 'sheet' : 'doc')}
								>{filter === 'sheets' ? 'New sheet' : 'New document'}</button
							>{/if}{/snippet}
				</Empty>
			{/if}
		</div>
	</div>
</SidebarShell>

<LibraryNew bind:this={creator} {companyId} />

<style>
	/* On a narrow pane the sidebar steps out for a row of the same views. */
	.library-filters {
		display: flex;
		gap: 2px;
		padding: 6px 8px;
		overflow-x: auto;
		border-bottom: 1px solid var(--border);
		scrollbar-width: none;
	}
	.library-filters a {
		flex: none;
		padding: 5px 10px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
	}
	.library-filters a.active {
		background: var(--wash-active, var(--wash-hover));
		color: var(--ink);
		font-weight: 500;
	}
	.page {
		container: page / inline-size;
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		width: 100%;
		min-height: 0;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 52px;
		padding: 8px 12px 8px 20px;
		border-bottom: 1px solid var(--border);
	}
	h1 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.head-count {
		color: var(--text-tertiary);
		font-variant-numeric: tabular-nums;
	}
	.spacer {
		flex: 1;
	}
	/* Search stays small until it is used. */
	.search {
		width: 160px;
		transition: width var(--motion-disclosure) var(--ease-standard);
	}
	.search:focus,
	.search:not(:placeholder-shown) {
		width: 240px;
	}
	.new {
		gap: 6px;
	}
	/* New ▾ wears the primary button: the menu's own trigger is an icon square. */
	.new-menu :global(summary) {
		display: inline-flex;
		align-items: center;
		width: auto;
		height: 28px;
		padding: 0 10px;
		border: 1px solid var(--primary-edge);
		border-radius: var(--radius-control);
		background: linear-gradient(
			180deg,
			color-mix(in srgb, var(--primary) 88%, #fff),
			var(--primary)
		);
		box-shadow:
			inset 0 1px 0 color-mix(in srgb, var(--highlight) 20%, transparent),
			0 1px 2px rgba(20, 32, 52, 0.22);
		color: var(--on-primary);
		font-size: var(--t-body);
		font-weight: 500;
	}
	.new-trigger {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.new-menu :global(.action-menu-panel button) {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.body {
		min-height: 0;
		overflow: auto;
		padding: 4px 8px 24px;
	}
	.body > :global(.notice) {
		margin-bottom: 10px;
	}
	/* Rows sit on the pane itself, on hairlines, as Linear's lists do: no card inside the card. */
	.table {
		overflow: hidden;
	}
	.table > :global(* + *) {
		border-top: 1px solid var(--border);
	}
	.table-head {
		display: flex;
		gap: 10px;
		border-bottom: 1px solid var(--border);
		padding: 8px 16px 8px 44px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.table-head span:first-child {
		flex: 1;
	}
	.table-head span:nth-child(2) {
		display: none;
	}
	.review {
		color: var(--intent-authority);
	}
	.more {
		width: 100%;
		min-height: 40px;
		border: 0;
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
	}
	.more:hover {
		background: var(--surface-hover);
	}
	@media (max-width: 760px) {
		.search {
			width: 120px;
		}
		.new span,
		.new-trigger > span {
			display: none;
		}
		.new {
			width: 36px;
			padding: 0;
			justify-content: center;
		}
	}
</style>
