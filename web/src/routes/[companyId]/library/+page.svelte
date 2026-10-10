<script lang="ts">
	/* The Library: everything the company has written and made, in one place (S64). Native
	 * documents and sheets open in their editors; the sites, images, PDFs, decks and recordings Work
	 * produced on the company computer open in place, a deck as a slideshow. One table, filtered
	 * from the side. A new document or sheet is made untitled and opens straight into its editor. */
	import { page } from '$app/state';
	import { Item, Notice, Empty, Dot } from '$lib/ui/page';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import SidebarShell from '$lib/ui/views/SidebarShell.svelte';
	import SidebarRow from '$lib/ui/views/SidebarRow.svelte';
	import SidebarGroup from '$lib/ui/views/SidebarGroup.svelte';
	import SidebarSearch from '$lib/ui/views/SidebarSearch.svelte';
	import SidebarHeadButton from '$lib/ui/views/SidebarHeadButton.svelte';
	import SidebarEmpty from '$lib/ui/views/SidebarEmpty.svelte';
	import LibraryViewer from '$lib/components/LibraryViewer.svelte';
	import { personName } from '$lib/model/initials';
	import LibraryNew from '$lib/components/LibraryNew.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FileText from '@lucide/svelte/icons/file-text';
	import Sheet from '@lucide/svelte/icons/sheet';
	import Library from '@lucide/svelte/icons/library';
	import Plus from '@lucide/svelte/icons/plus';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Eye from '@lucide/svelte/icons/eye';
	import Globe from '@lucide/svelte/icons/globe';
	import ImageIcon from '@lucide/svelte/icons/image';
	import Presentation from '@lucide/svelte/icons/presentation';
	import FileType from '@lucide/svelte/icons/file-type';
	import Film from '@lucide/svelte/icons/film';
	import NotebookText from '@lucide/svelte/icons/notebook-text';
	import FileSpreadsheet from '@lucide/svelte/icons/file-spreadsheet';
	import Pin from '@lucide/svelte/icons/pin';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import {
		LIBRARY_KIND_LABEL,
		libraryFiles,
		readPins,
		writePins,
		type LibraryArtifact,
		type LibraryFileKind
	} from '$lib/model/library-files';
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

	type Filter = 'all' | 'docs' | 'sheets' | 'review' | 'pinned' | 'archived' | LibraryFileKind;
	const filter = $derived((page.url.searchParams.get('show') as Filter) || 'all');
	const openFileId = $derived(page.url.searchParams.get('file') ?? '');
	let search = $state('');

	/* What Work produced on the company computer. The owner reads the Work graph; a collaborator's
	 * projection carries no file locations, so files are the owner's view for now (S64 decision 5). */
	const files = $derived(
		owner
			? libraryFiles(
					((attention.view?.workGraph?.artifacts ?? []) as unknown as LibraryArtifact[]) ?? []
				)
			: []
	);
	/* Files from Work that waits on the owner's judgement are waiting on their review too. */
	const workWaitingOnOwner = $derived(
		new Set(
			(attention.view?.workGraph?.handoffs ?? [])
				.filter((handoff) => handoff.state === 'pending')
				.map((handoff) => handoff.work_id)
		)
	);
	const fileOpen = $derived(files.find((file) => file.id === openFileId) ?? null);

	let pins = $state<string[]>([]);
	$effect(() => {
		pins = readPins(companyId);
	});
	function togglePin(key: string) {
		pins = pins.includes(key) ? pins.filter((pin) => pin !== key) : [key, ...pins];
		writePins(companyId, pins);
	}
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
		kind: 'doc' | 'sheet' | LibraryFileKind;
		title: string;
		href: string;
		updatedAt: string;
		owner: string;
		status: string;
		review: boolean;
	};
	const pinKey = (entry: Pick<Entry, 'kind' | 'id'>) => `${entry.kind}:${entry.id}`;
	const isFile = (entry: Pick<Entry, 'kind'>) => entry.kind !== 'doc' && entry.kind !== 'sheet';
	const fileHref = (id: string) => {
		const query = new URLSearchParams(page.url.search);
		query.set('file', id);
		return `${root}/library?${query}`;
	};
	const entries = $derived<Entry[]>(
		[
			...files.map((file) => ({
				id: file.id,
				kind: file.kind,
				title: file.label,
				href: fileHref(file.id),
				updatedAt: file.createdAt,
				owner: name(file.createdBy),
				status: '',
				review: !!file.workId && workWaitingOnOwner.has(file.workId)
			})),
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
	const FILE_KINDS: LibraryFileKind[] = ['site', 'deck', 'image', 'pdf', 'media', 'text', 'office'];
	const KIND_ICON: Record<'doc' | 'sheet' | LibraryFileKind, typeof Library> = {
		doc: FileText,
		sheet: Sheet,
		site: Globe,
		deck: Presentation,
		image: ImageIcon,
		pdf: FileType,
		media: Film,
		text: NotebookText,
		office: FileSpreadsheet
	};
	const count = (kind: Entry['kind']) => entries.filter((entry) => entry.kind === kind).length;
	const counts = $derived({
		all: entries.length,
		docs: count('doc'),
		sheets: count('sheet'),
		review: entries.filter((entry) => entry.review).length,
		archived: archivedEntries.length
	});
	/* Documents and Sheets are always places; a file kind appears once Work has made one. */
	const fileKinds = $derived(
		FILE_KINDS.map((kind) => ({ kind, total: count(kind) })).filter((row) => row.total > 0)
	);
	const pinned = $derived(
		pins
			.map((key) => entries.find((entry) => pinKey(entry) === key))
			.filter((entry): entry is Entry => !!entry)
	);
	const shown = $derived(
		(filter === 'archived' ? archivedEntries : entries).filter(
			(entry) =>
				(filter === 'all' ||
					filter === 'archived' ||
					(filter === 'docs' && entry.kind === 'doc') ||
					(filter === 'sheets' && entry.kind === 'sheet') ||
					(filter === 'review' && entry.review) ||
					(filter === 'pinned' && pins.includes(pinKey(entry))) ||
					entry.kind === filter) &&
				(!search.trim() || entry.title.toLowerCase().includes(search.trim().toLowerCase()))
		)
	);
	const loading = $derived(documents.status === 'unknown' || !sheetsLoaded);
	const FILTERS: { key: Filter; label: string }[] = [
		{ key: 'all', label: 'All files' },
		{ key: 'review', label: 'Needs your review' },
		{ key: 'docs', label: 'Documents' },
		{ key: 'sheets', label: 'Sheets' },
		{ key: 'archived', label: 'Archived' }
	];
	const filterLabel = (key: Filter) =>
		key === 'pinned'
			? 'Pinned'
			: (FILTERS.find((item) => item.key === key)?.label ??
				(FILE_KINDS.includes(key as LibraryFileKind)
					? LIBRARY_KIND_LABEL[key as LibraryFileKind]
					: 'All files'));

	const filterHref = (key: Filter) =>
		key === 'all' ? `${root}/library` : `${root}/library?show=${key}`;
	const backHref = $derived(filterHref(filter));

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

<!-- The same sidebar as every area: a title, search, views, then the kinds of thing kept here. -->
<SidebarShell label="Library" title="Library">
	{#snippet action()}
		<SidebarHeadButton label="New document" icon={Plus} onclick={() => creator?.open('doc')} />
	{/snippet}
	{#snippet top()}
		<SidebarSearch bind:value={search} placeholder="Search files" />
	{/snippet}
	{#snippet nav()}
		<SidebarRow
			href={filterHref('all')}
			label="All files"
			icon={Library}
			active={filter === 'all' && !fileOpen}
			count={counts.all || null}
		/>
		<SidebarRow
			href={filterHref('review')}
			label="Needs your review"
			icon={Eye}
			active={filter === 'review' && !fileOpen}
			count={counts.review || null}
			todo
			title="Documents and outcomes waiting on your judgement"
		/>
		<SidebarGroup label="Kinds">
			<SidebarRow
				href={filterHref('docs')}
				label="Documents"
				icon={FileText}
				active={filter === 'docs' && !fileOpen}
				count={counts.docs || null}
			/>
			<SidebarRow
				href={filterHref('sheets')}
				label="Sheets"
				icon={Sheet}
				active={filter === 'sheets' && !fileOpen}
				count={counts.sheets || null}
			/>
			{#each fileKinds as row (row.kind)}
				<SidebarRow
					href={filterHref(row.kind)}
					label={LIBRARY_KIND_LABEL[row.kind]}
					icon={KIND_ICON[row.kind]}
					active={filter === row.kind && !fileOpen}
					count={row.total}
				/>
			{/each}
		</SidebarGroup>
		{#if pinned.length}
			<SidebarGroup label="Pinned" count={pinned.length}>
				{#each pinned as entry (pinKey(entry))}
					<SidebarRow
						href={entry.href}
						label={entry.title}
						icon={Pin}
						active={isFile(entry) && entry.id === openFileId}
						title={`${entry.title} · ${entry.owner}`}
					/>
				{/each}
			</SidebarGroup>
		{/if}
		{#if !loading && !entries.length && !search.trim()}
			<SidebarEmpty
				text="Everything the team makes lands here: documents, sheets, sites, decks and images."
			>
				<button type="button" onclick={() => creator?.open('doc')}
					><FileText size={15} strokeWidth={1.8} aria-hidden="true" />New document</button
				>
			</SidebarEmpty>
		{/if}
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
		<!-- An open file has its own way back; the list's search and views step aside for it. -->
		{#if !openFileId}<div class="library-narrow-search">
				<SidebarSearch bind:value={search} placeholder="Search files" />
			</div>{/if}
		<nav class="library-filters" aria-label="Library" hidden={!!openFileId}>
			{#each [...FILTERS.slice(0, -1), ...fileKinds.map( (row) => ({ key: row.kind as Filter, label: LIBRARY_KIND_LABEL[row.kind] }) ), ...(pinned.length ? [{ key: 'pinned' as Filter, label: 'Pinned' }] : []), FILTERS.at(-1)!] as item (item.key)}
				<a
					href={filterHref(item.key)}
					class:active={filter === item.key}
					aria-current={filter === item.key ? 'page' : undefined}>{item.label}</a
				>
			{/each}
		</nav>
	{/snippet}
	{#if fileOpen}
		<LibraryViewer {companyId} file={fileOpen} owner={name(fileOpen.createdBy)} {backHref} />
	{:else if openFileId && !loading && attention.view}
		<Empty
			title="This file is no longer in the Library"
			info="A newer version replaced it, or the Work that made it was retired."
		>
			{#snippet action()}<a class="btn small" href={backHref}>Back to the Library</a>{/snippet}
		</Empty>
	{:else}
		<div class="page">
			<header class="head">
				<h1>{filterLabel(filter)}</h1>
				<span class="head-count">{shown.length || ''}</span>
				<span class="spacer"></span>
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
								{#snippet leading()}{@const KindIcon = KIND_ICON[entry.kind]}<KindIcon
										size={15}
										strokeWidth={1.8}
										aria-label={entry.kind === 'doc'
											? 'Document'
											: entry.kind === 'sheet'
												? 'Sheet'
												: LIBRARY_KIND_LABEL[entry.kind as LibraryFileKind]}
									/>{/snippet}
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
										{#if filter !== 'archived'}<button onclick={() => togglePin(pinKey(entry))}
												>{pins.includes(pinKey(entry)) ? 'Unpin' : 'Pin to the sidebar'}</button
											>{/if}
										{#if entry.kind === 'doc' || entry.kind === 'sheet'}
											{@const native = entry as Entry & { kind: 'doc' | 'sheet' }}
											{#if filter === 'archived'}<button
													disabled={!!archiveBusy}
													onclick={() => void setArchived(native, false)}>Restore</button
												>{:else}<button
													disabled={!!archiveBusy}
													title="Out of the Library lists, kept under Archived; restore it any time."
													onclick={() => void setArchived(native, true)}>Archive</button
												>{/if}
										{/if}
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
						info="Documents, sheets and everything the team makes, such as sites, decks and images, appear here as they work."
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
	{/if}
</SidebarShell>

<LibraryNew bind:this={creator} {companyId} />

<style>
	.library-narrow-search {
		padding: 10px 12px 4px;
	}
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
	/* The selected view is raised, as it is in the sidebar and the top navigation. */
	.library-filters a.active {
		background: var(--surface-raised);
		box-shadow:
			var(--shadow-soft),
			var(--bevel),
			0 0 0 1px var(--border);
		color: var(--ink);
		font-weight: 600;
	}
	.library-filters[hidden] {
		display: none;
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
