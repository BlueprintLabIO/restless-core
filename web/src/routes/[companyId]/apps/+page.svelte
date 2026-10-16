<script lang="ts">
	import AppSetupDialog, { type SetupResult } from '$lib/components/AppSetupDialog.svelte';
	/* Discovery and company-owned apps share data, while setup stays a deliberate secondary task. */
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Notice, Empty, Dot } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import { goto, replaceState } from '$app/navigation';
	import AppCard from '$lib/components/AppCard.svelte';
	import { BROWSE_CATEGORIES } from '$lib/model/apps';
	import { setSkillDisposition } from '$lib/model/skills';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import {
		KIND_INFO,
		KIND_LABEL,
		KIND_SECTION,
		appLevel,
		builtIn,
		type App
	} from '$lib/model/apps';
	import { useAppsData, viewFrom, isKind, type AppsView } from '$lib/model/apps-data.svelte';
	import AppMark from '$lib/primitives/AppMark.svelte';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { hasOwnerSurfaceAccess } from '$lib/model/company-access';

	const data = useAppsData();
	const companyId = $derived(page.params.companyId ?? '');
	/* An admin reads Apps beside the owner: adding, importing, writing or restoring is the owner's. */
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(hasOwnerSurfaceAccess(principal.view));
	let notice = $state('');
	let actionFailure = $state('');
	let busy = $state('');
	let skippedImport = $state<string[]>([]);
	let reviewHref = $state('');
	const rawView = $derived(viewFrom(page.url.searchParams.get('view')) ?? 'all');
	const view = $derived<AppsView>(rawView === 'ask' ? 'all' : rawView);
	const setup = $derived(
		rawView === 'ask'
			? 'ask'
			: owner && ['link', 'skill'].includes(page.url.searchParams.get('add') ?? '')
				? (page.url.searchParams.get('add') as 'link' | 'skill')
				: null
	);
	async function closeSetup() {
		const url = new URL(window.location.href);
		url.searchParams.delete('add');
		if (url.searchParams.get('view') === 'ask') url.searchParams.delete('view');
		await goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	}
	async function setupSaved(result: SetupResult) {
		if (result.company !== companyId) return;
		notice = result.notice ?? '';
		skippedImport = result.skipped ?? [];
		reviewHref = result.review ? `/${encodeURIComponent(companyId)}/apps?view=waiting` : '';
		await data.load();
		if (result.company !== companyId) return;
		if (result.app) await goto(route(result.app));
		else await closeSetup();
	}
	const returnHref = $derived.by(() => {
		const url = new URL(page.url);
		if (tileQuery) url.searchParams.set('q', tileQuery);
		else url.searchParams.delete('q');
		if (pageIndex > 1) url.searchParams.set('page', String(pageIndex));
		else url.searchParams.delete('page');
		return url.pathname + url.search;
	});
	const route = (key: string) =>
		`/${encodeURIComponent(companyId)}/apps/${encodeURIComponent(key)}?from=${encodeURIComponent(returnHref)}`;
	const failure = $derived(actionFailure || data.failure);
	let retrying = $state(false);
	async function retry() {
		retrying = true;
		try {
			await data.retry();
		} finally {
			retrying = false;
		}
	}

	const TITLES: Record<string, string> = {
		ask: 'Apps',
		waiting: 'Needs setup',
		'in-use': 'Installed',
		'installed-apps': 'Apps',
		'installed-know-how': 'Know-how',
		removed: 'Removed',
		all: 'Discover',
		More: 'Community'
	};
	const title = $derived(TITLES[view] ?? (isKind(view) ? KIND_SECTION[view] : view));

	async function act(key: string, action: () => Promise<unknown>, failed: string) {
		if (busy) return;
		busy = key;
		actionFailure = '';
		notice = '';
		skippedImport = [];
		reviewHref = '';
		try {
			await action();
		} catch (cause) {
			actionFailure = failureSentence(cause, failed);
		} finally {
			busy = '';
		}
	}

	async function restore(app: App) {
		const target = companyId;
		const skill = app.skills[0];
		if (!skill) return;
		await act(
			`restore:${app.key}`,
			async () => {
				await setSkillDisposition(target, skill.name, 'accepted');
				if (target !== companyId) return;
				await data.load();
				toast(`${app.name} is company know-how again.`);
			},
			'That know-how was not restored.'
		);
	}

	const tone = (app: App) =>
		app.state === 'needs_you' ? 'warning' : app.state === 'paused' ? 'muted' : 'success';
	const status = (app: App) => app.attention ?? (app.state === 'paused' ? 'Paused' : 'In use');

	const tableApps = $derived(
		(view === 'in-use'
			? data.inUse
			: view === 'installed-apps'
				? data.inUse.filter((app) => app.kind !== 'skill')
				: view === 'installed-know-how'
					? data.inUse.filter((app) => app.skills.length > 0)
					: isKind(view)
						? data.ofKind(view)
						: []
		).filter((app) =>
			tileQuery
				.toLowerCase()
				.split(/\s+/)
				.filter(Boolean)
				.every((term) => `${app.name} ${app.description}`.toLowerCase().includes(term))
		)
	);
	const tiles = $derived(
		view === 'all'
			? data.browse
			: !isKind(view) &&
				  !['ask', 'waiting', 'in-use', 'installed-apps', 'installed-know-how', 'removed'].includes(
						view
				  )
				? data.inCategory(view as never)
				: []
	);
	const showWaiting = $derived(view === 'waiting');
	const discovering = $derived(
		view === 'all' ||
			(!isKind(view) &&
				!['ask', 'waiting', 'in-use', 'installed-apps', 'installed-know-how', 'removed'].includes(
					view
				))
	);
	const installed = $derived(
		['in-use', 'installed-apps', 'installed-know-how'].includes(view) || isKind(view)
	);
	const partial = $derived(discovering ? data.partial : data.companyPartial);
	const PAGE_SIZE = 24;
	const tileQuery = $derived(
		page.state.appsDiscovery?.query ?? page.url.searchParams.get('q') ?? ''
	);
	const searchIndex = $derived(
		tiles.map((app) => ({
			app,
			text: `${app.name} ${app.description} ${app.catalogue?.registryId ?? ''} ${app.catalogue?.homepage ?? ''}`.toLowerCase()
		}))
	);
	const matchingTiles = $derived.by(() => {
		const terms = tileQuery.toLowerCase().split(/\s+/).filter(Boolean);
		return terms.length
			? searchIndex
					.filter((row) => terms.every((term) => row.text.includes(term)))
					.map((row) => row.app)
			: tiles;
	});
	const landing = $derived(
		view === 'all' && !tileQuery.trim() && !page.url.searchParams.has('browse')
	);
	const pageCount = $derived(Math.max(1, Math.ceil(matchingTiles.length / PAGE_SIZE)));
	const pageIndex = $derived(
		Math.min(
			pageCount,
			Math.max(
				1,
				page.state.appsDiscovery?.page ?? (Number(page.url.searchParams.get('page')) || 1)
			)
		)
	);
	const featured = $derived(
		data.browse.filter((app) => app.catalogue && !app.catalogue.registryId).slice(0, 8)
	);
	const knowHow = $derived(data.browse.filter((app) => app.kind === 'skill').slice(0, 4));
	const shownTiles = $derived(
		landing ? featured : matchingTiles.slice((pageIndex - 1) * PAGE_SIZE, pageIndex * PAGE_SIZE)
	);
	const categoryHref = (category: string) =>
		`?view=${encodeURIComponent(category.toLowerCase())}${tileQuery ? `&q=${encodeURIComponent(tileQuery)}` : ''}`;
	function searchApps(value: string) {
		const url = new URL(window.location.href);
		url.searchParams.delete('page');
		if (value) url.searchParams.set('q', value);
		else url.searchParams.delete('q');
		replaceState(url, { ...page.state, appsDiscovery: { query: value, page: 1 } });
	}
	async function turnPage(value: number) {
		const url = new URL(window.location.href);
		url.searchParams.set('page', String(value));
		url.searchParams.set('browse', '1');
		await goto(url, { keepFocus: true });
		document.getElementById('discover-search')?.focus();
	}
</script>

{#if setup}
	{#key `${companyId}:${setup}`}<AppSetupDialog
			company={companyId}
			mode={setup}
			onclose={() => void closeSetup()}
			onsaved={setupSaved}
		/>{/key}
{/if}

<CompanyTitle {title} {companyId} />

{#snippet retryAction()}<button
		class="btn small"
		type="button"
		disabled={retrying}
		onclick={() => void retry()}>{retrying ? 'Trying…' : 'Try again'}</button
	>{/snippet}

{#snippet review()}<a class="btn small" href={reviewHref}>Review</a>{/snippet}

{#snippet mark(app: App, size: 24 | 32 | 40 = 24)}
	<AppMark
		name={app.name}
		catalogueKey={app.catalogue?.key}
		knowHow={app.kind === 'skill'}
		{size}
	/>
{/snippet}

{#key view}
	<Page
		{title}
		variant={discovering || installed ? 'list' : 'settings'}
		info={discovering
			? data.catalogueStatus
			: view === 'removed'
				? 'Know-how you removed, kept so it can be restored.'
				: isKind(view)
					? KIND_INFO[view]
					: undefined}
	>
		{#snippet toolbar()}
			{#if discovering || installed}
				<div class="store-toolbar">
					<input
						id="discover-search"
						class="store-search"
						type="search"
						placeholder={discovering ? 'Search apps and know-how' : 'Search installed apps'}
						aria-label="Search apps"
						value={tileQuery}
						oninput={(event) => searchApps(event.currentTarget.value)}
					/>
					{#if discovering}
						<select
							aria-label="App category"
							value={view}
							onchange={(event) => void goto(categoryHref(event.currentTarget.value))}
						>
							<option value="all">All categories</option>
							{#each BROWSE_CATEGORIES as category}<option value={category}
									>{category === 'More' ? 'Community' : category}</option
								>{/each}
						</select>
					{/if}
				</div>
			{/if}
		{/snippet}

		{#if failure}<Notice
				tone="danger"
				title={actionFailure ? actionFailure : 'Apps could not be read'}
				details={actionFailure ? null : failure}
			/>{/if}
		{#if !failure && partial}<Notice
				tone="warning"
				title={partial}
				details="What did load is shown. Nothing was removed."
				actions={retryAction}
			/>{/if}
		{#if notice}<Notice
				tone="success"
				title={notice}
				details={skippedImport.length ? `Not added: ${skippedImport.join('; ')}.` : null}
				actions={reviewHref ? review : undefined}
			/>{/if}

		{#if !data.ready}
			{#if !data.failure}<Skeleton label="Reading apps…" variant="page" count={4} />{/if}
		{:else}
			{#if showWaiting && (data.requests.length || data.needsYou.length)}
				<Section
					info="Apps Exec asked for to unblock work, and ones that need a sign-in, a review or a decision."
				>
					{#each data.requests as request (request.handoff_id)}
						<Item title={request.name} meta={request.reason} href={request.href(companyId)} unread>
							{#snippet leading()}<AppMark
									name={request.name}
									catalogueKey={request.catalogueKey}
									size={24}
								/>{/snippet}
							{#snippet trailing()}
								<span
									class="asked"
									title={request.work_title ? `For “${request.work_title}”` : undefined}
									>{request.asker} asked</span
								>
								{#if owner}<a class="btn small primary" href={request.href(companyId)}>Add</a>{/if}
							{/snippet}
						</Item>
					{/each}
					{#each data.needsYou as app (app.key)}
						<Item title={app.name} meta={app.description} href={route(app.key)} unread>
							{#snippet leading()}{@render mark(app)}{/snippet}
							{#snippet trailing()}
								<span class="kind" title={KIND_INFO[app.kind]}>{KIND_LABEL[app.kind]}</span>
								<Dot tone="warning" label={app.attention ?? 'Needs you'} show />
							{/snippet}
						</Item>
					{/each}
				</Section>
			{:else if view === 'waiting'}
				<Empty compact title="Nothing is waiting on you" info="Apps that need you appear here." />
			{/if}

			{#if installed}
				<div class="installed">
					{#if tableApps.length}
						{@const permissions = tableApps.some((app) => app.connections.length > 0)}
						<table class="table" class:with-permissions={permissions} aria-label={title}>
							<thead
								><tr class="tr head">
									<th scope="col">App</th>
									{#if permissions}<th scope="col" class="access">Access</th>{/if}
									<th scope="col">Status</th>
								</tr></thead
							>
							<tbody
								>{#each tableApps as app (app.key)}
									{@const level = appLevel(app)}
									<tr class="tr" class:dim={app.state === 'paused'}>
										<td>
											<a class="identity" href={route(app.key)} aria-label={app.name}>
												{@render mark(app)}
												<span class="app-copy">
													<span class="app-name"
														><strong>{app.name}</strong>{#if builtIn(app)}<small>Included</small
															>{/if}</span
													>
													<span class="what">{app.description}</span>
												</span>
											</a>
										</td>
										{#if permissions}<td class="access" title={level.info}>{level.label}</td>{/if}
										<td><Dot tone={tone(app)} label={status(app)} show /></td>
									</tr>
								{/each}</tbody
							>
						</table>
					{:else}
						<Empty
							compact
							title={tileQuery
								? 'No installed apps match'
								: isKind(view)
									? `No ${KIND_SECTION[view].toLowerCase()} yet`
									: 'Nothing in use yet'}
							info={tileQuery
								? 'Try another name or clear the search.'
								: 'Ask Exec for what the company should be able to do, or discover an app.'}
						>
							{#snippet action()}{#if tileQuery}<button
										class="btn small"
										type="button"
										onclick={() => searchApps('')}>Clear search</button
									>{:else}<a class="btn small" href="?view=ask">Ask Exec</a>{/if}{/snippet}
						</Empty>
					{/if}
				</div>
			{/if}

			{#if view === 'removed'}
				<Section>
					{#each data.removed as app (app.key)}
						<Item title={app.name} meta={app.description} href={route(app.key)} dim>
							{#snippet leading()}{@render mark(app)}{/snippet}
							{#snippet trailing()}
								{#if owner}<button
										class="btn small"
										type="button"
										disabled={!!busy}
										onclick={() => void restore(app)}
										>{busy === `restore:${app.key}` ? 'Restoring…' : 'Restore'}</button
									>{/if}
							{/snippet}
						</Item>
					{:else}
						<Empty compact title="Nothing removed" info="Know-how you remove is kept here." />
					{/each}
				</Section>
			{/if}

			{#if discovering}
				<div class="discovery">
					{#if landing}<div class="collection-heading">
							<h2>Featured</h2>
							<a href="?view=all&browse=1">Browse all</a>
						</div>{/if}
					<div class="app-grid">
						{#each shownTiles as app (app.key)}<AppCard {app} href={route(app.key)} />
						{:else}
							{#if data.catalogueLoading}<Skeleton
									label="Reading the catalogue…"
									variant="list"
									count={3}
								/>
							{:else}<Empty
									compact
									title={tileQuery ? 'No apps match' : 'No apps in this collection'}
									info={tileQuery
										? 'Try another name or describe what you need to Exec.'
										: 'Explore another category or search the directory.'}
								>
									{#snippet action()}<a class="btn small" href="?view=ask">Ask Exec</a>{/snippet}
								</Empty>{/if}
						{/each}
					</div>
					{#if landing}
						{#if knowHow.length}
							<div class="collection-heading">
								<h2>Know-how</h2>
								<a href="?view=know-how">Explore</a>
							</div>
							<div class="app-grid">
								{#each knowHow as app (app.key)}<AppCard {app} href={route(app.key)} />{/each}
							</div>
						{/if}
						<a class="directory-link" href="?view=more"
							><span
								><strong>Community directory</strong><span
									>Explore services published by the community.</span
								></span
							><span aria-hidden="true">→</span></a
						>
					{:else if matchingTiles.length}
						<nav class="store-pagination" aria-label="Catalogue pages">
							<button
								class="btn small"
								type="button"
								disabled={pageIndex === 1}
								onclick={() => void turnPage(pageIndex - 1)}>Previous</button
							>
							<span aria-live="polite">Page {pageIndex} of {pageCount}</span>
							<button
								class="btn small"
								type="button"
								disabled={pageIndex === pageCount}
								onclick={() => void turnPage(pageIndex + 1)}>Next</button
							>
						</nav>
					{/if}
				</div>
			{/if}
		{/if}
	</Page>
{/key}

<style>
	.store-toolbar {
		display: flex;
		flex: 1;
		flex-wrap: wrap;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}
	.store-search {
		flex: 1 1 220px;
		min-width: 0;
		max-width: 480px;
		scroll-margin-top: 68px;
	}
	.store-toolbar select {
		flex: 0 1 auto;
		max-width: 100%;
	}
	.discovery {
		padding: 24px;
		display: grid;
		gap: 20px;
	}
	.collection-heading {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-top: 8px;
	}
	.collection-heading h2 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
		letter-spacing: -0.01em;
	}
	.collection-heading a {
		font-size: var(--t-body);
		text-decoration: none;
	}
	.app-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(100%, 240px), 1fr));
		gap: 14px;
	}
	.directory-link {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 20px;
		padding: 22px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		color: var(--ink);
		text-decoration: none;
		background: var(--surface-alt);
	}
	.directory-link > span:first-child {
		display: grid;
		gap: 6px;
		min-width: 0;
	}
	.directory-link strong {
		font-size: var(--t-body);
		font-weight: 600;
	}
	.directory-link span span {
		font-size: var(--t-body);
		color: var(--text-secondary);
	}
	.store-pagination {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding-block: 12px;
	}
	.store-pagination span {
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	@container page (max-width: 560px) {
		.discovery {
			padding: 16px 14px;
		}
	}
	.asked {
		color: var(--intent-authority);
	}
	.kind {
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.installed {
		padding: 24px;
	}
	.table {
		width: 100%;
		border-spacing: 0;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.tr {
		position: relative;
		display: grid;
		grid-template-columns: minmax(0, 1fr) 100px;
		gap: 20px;
		align-items: center;
		min-height: 74px;
		padding: 12px 14px;
		border-top: 1px solid var(--border);
		color: var(--ink);
		font-size: var(--t-body);
		text-decoration: none;
	}
	.with-permissions .tr {
		grid-template-columns: minmax(0, 1fr) 140px 100px;
	}
	.tr.head {
		min-height: 34px;
		padding-block: 8px;
		border-top: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.tr:not(.head):hover {
		background: var(--wash-hover, var(--surface-alt));
	}
	.tr:focus-within {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	.tr.dim {
		color: var(--text-tertiary);
	}
	.tr th,
	.tr td {
		min-width: 0;
		padding: 0;
		text-align: left;
		font-weight: inherit;
	}
	.identity {
		color: inherit;
		text-decoration: none;
	}
	.identity::after {
		content: '';
		position: absolute;
		inset: 0;
	}
	.identity:focus-visible {
		outline: none;
	}
	.identity {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
	}
	.app-copy {
		display: grid;
		gap: 4px;
		min-width: 0;
	}
	.app-name {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}
	.app-name strong {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 500;
	}
	.app-name small {
		flex: none;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: nowrap;
	}
	.what {
		overflow: hidden;
		color: var(--text-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.access {
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	@container page (max-width:560px) {
		.tr,
		.with-permissions .tr {
			grid-template-columns: minmax(0, 1fr) auto;
			gap: 12px;
		}
		.tr .access {
			display: none;
		}
		.app-name small {
			display: none;
		}
	}
</style>
