<script lang="ts">
	/* The Library: what the company has written and built, in one place. Docs
	 * and sheets share one table, filtered from the side; each opens in its
	 * editor. New items start in that editor's own create flow. */
	import { page } from '$app/state';
	import { Item, Notice, Empty, Dot } from '$lib/ui/page';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FileText from '@lucide/svelte/icons/file-text';
	import Sheet from '@lucide/svelte/icons/sheet';
	import Library from '@lucide/svelte/icons/library';
	import Eye from '@lucide/svelte/icons/eye';
	import Plus from '@lucide/svelte/icons/plus';
	import { documentsQuery } from '$lib/model/document-queries.svelte';
	import { sheetJson, type SheetRow } from '$lib/model/sheets';
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
	$effect(() => {
		const target = companyId;
		if (!principal.view) return;
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

	type Filter = 'all' | 'docs' | 'sheets' | 'review';
	const filter = $derived((page.url.searchParams.get('show') as Filter) || 'all');
	let search = $state('');

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
	const counts = $derived({
		all: entries.length,
		docs: entries.filter((entry) => entry.kind === 'doc').length,
		sheets: entries.filter((entry) => entry.kind === 'sheet').length,
		review: entries.filter((entry) => entry.review).length
	});
	const shown = $derived(
		entries.filter(
			(entry) =>
				(filter === 'all' ||
					(filter === 'docs' && entry.kind === 'doc') ||
					(filter === 'sheets' && entry.kind === 'sheet') ||
					(filter === 'review' && entry.review)) &&
				(!search.trim() || entry.title.toLowerCase().includes(search.trim().toLowerCase()))
		)
	);
	const loading = $derived(documents.status === 'unknown' || !sheetsLoaded);
	const FILTERS: { key: Filter; label: string; icon: typeof Library }[] = [
		{ key: 'all', label: 'All', icon: Library },
		{ key: 'docs', label: 'Documents', icon: FileText },
		{ key: 'sheets', label: 'Sheets', icon: Sheet },
		{ key: 'review', label: 'Needs your review', icon: Eye }
	];

	function name(actorId: string): string {
		if (actorId === 'owner') return 'You';
		return (
			people.find((person) => person.actor_id === actorId)?.display ??
			actorId.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
		);
	}
	const statusLabel = (status: string) =>
		({ draft: 'Draft', in_review: 'In review', accepted: 'Accepted' })[status] ?? '';
	const statusTone = (status: string) =>
		status === 'in_review' ? 'warning' : status === 'accepted' ? 'success' : 'muted';
</script>

<CompanyTitle title="Library" {companyId} />

<div class="library">
	<nav class="library-nav" aria-label="Library">
		<h2>Library</h2>
		{#each FILTERS as item (item.key)}
			{@const Icon = item.icon}
			<a
				href={item.key === 'all' ? `${root}/library` : `${root}/library?show=${item.key}`}
				class:active={filter === item.key}
				aria-current={filter === item.key ? 'page' : undefined}
				><Icon size={15} strokeWidth={1.8} aria-hidden="true" /><span>{item.label}</span
				>{#if counts[item.key]}<span class="count">{counts[item.key]}</span>{/if}</a
			>
		{/each}
	</nav>

	<main class="library-main">
		<div class="page">
			<header class="head">
				<h1>{FILTERS.find((item) => item.key === filter)?.label ?? 'All'}</h1>
				<span class="head-count">{shown.length || ''}</span>
				<span class="spacer"></span>
				<input
					class="search"
					type="search"
					bind:value={search}
					placeholder="Search"
					aria-label="Search the Library"
				/>
				<a class="btn small new-sheet" href={`${root}/library/sheets?new=1`}>New sheet</a>
				<a
					class="btn small primary new"
					href={`${root}/library/documents?new=1`}
					aria-label="New document"
					title="New document"
					><Plus size={14} strokeWidth={2} aria-hidden="true" /><span>New document</span></a
				>
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
								: 'Nothing here yet'}
						info="Documents and sheets the company writes appear here. Agents add to it as they work."
					>
						{#snippet action()}{#if !search && filter !== 'review'}<a
									class="btn small"
									href={`${root}/library/${filter === 'sheets' ? 'sheets' : 'documents'}?new=1`}
									>{filter === 'sheets' ? 'New sheet' : 'New document'}</a
								>{/if}{/snippet}
					</Empty>
				{/if}
			</div>
		</div>
	</main>
</div>

<style>
	.library {
		display: flex;
		flex: 1 1 auto;
		gap: var(--pane-gap);
		width: 100%;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
	}
	.library-nav,
	.library-main {
		min-height: 0;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		box-shadow: var(--bevel), var(--shadow-soft);
	}
	.library-nav {
		display: grid;
		align-content: start;
		gap: 1px;
		flex: none;
		width: 220px;
		padding: 14px 8px;
		background: var(--surface-rail);
	}
	h2 {
		margin: 0 8px 10px;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.library-nav a {
		display: flex;
		align-items: center;
		gap: 9px;
		min-height: 30px;
		padding: 0 8px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
	}
	.library-nav a :global(svg) {
		color: var(--text-tertiary);
	}
	.library-nav a:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.library-nav a.active {
		background: var(--surface-raised);
		box-shadow: var(--control-depth);
		color: var(--ink);
		font-weight: 500;
	}
	.count {
		margin-left: auto;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
	}
	.library-main {
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		background: var(--surface-pane);
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
	.search {
		width: 200px;
	}
	.new {
		gap: 6px;
	}
	.body {
		min-height: 0;
		overflow: auto;
		padding: 12px 12px 24px;
	}
	.body > :global(.notice) {
		margin-bottom: 10px;
	}
	.table {
		overflow: hidden;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.table > :global(* + *) {
		border-top: 1px solid var(--border);
	}
	.table-head {
		display: flex;
		gap: 10px;
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
		.library {
			flex-direction: column;
		}
		.library-nav {
			display: flex;
			width: auto;
			overflow-x: auto;
			padding: 4px;
		}
		.library-nav h2,
		.count {
			display: none;
		}
		.library-nav a {
			flex: none;
		}
		.search {
			width: 120px;
		}
		.new-sheet,
		.new span {
			display: none;
		}
		.new {
			width: 36px;
			padding: 0;
			justify-content: center;
		}
	}
</style>
