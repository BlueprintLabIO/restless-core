<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Notice, Empty, Dot } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { addConnection, importPlugin } from '$lib/model/connections';
	import { classifyLink, nameForLink, type App } from '$lib/model/apps';
	import { useAppsData, viewFrom, type AppsView } from '$lib/model/apps-data.svelte';
	import AppTile from './AppTile.svelte';
	import AppMark from '$lib/primitives/AppMark.svelte';

	const data = useAppsData();
	const companyId = $derived(page.params.companyId ?? 'aris');
	let notice = $state('');
	let actionFailure = $state('');
	let busy = $state('');
	let search = $state('');
	let linking = $state(false);
	let link = $state('');

	const view = $derived<AppsView>(
		viewFrom(page.url.searchParams.get('view')) ??
			(data.waiting || data.added.length ? 'in-use' : 'all')
	);
	const query = $derived(search.trim().toLowerCase());
	const matches = (app: App) =>
		!query || `${app.name} ${app.description}`.toLowerCase().includes(query);

	/* A search looks everywhere; otherwise the pane shows the view the list selected. */
	const title = $derived(
		query ? 'Search' : view === 'in-use' ? 'In use' : view === 'all' ? 'All apps' : view
	);
	const tiles = $derived(
		query
			? data.browse.filter(matches)
			: view === 'all'
				? data.browse
				: view === 'in-use'
					? []
					: data.inCategory(view)
	);
	const showInUse = $derived(!!query || view === 'in-use');
	const requests = $derived(
		data.requests.filter((request) => !query || request.name.toLowerCase().includes(query))
	);
	const needsYou = $derived(data.needsYou.filter(matches));
	const added = $derived(data.added.filter(matches));
	const included = $derived(data.included.filter(matches));
	const route = (key: string) =>
		`/${encodeURIComponent(companyId)}/apps/${encodeURIComponent(key)}`;
	const failure = $derived(actionFailure || data.failure);

	async function act(key: string, action: () => Promise<unknown>) {
		if (busy) return;
		busy = key;
		actionFailure = '';
		notice = '';
		try {
			await action();
		} catch (cause) {
			actionFailure = failureSentence(cause, 'That app was not added.');
		} finally {
			busy = '';
		}
	}

	/* Adding a known service probes it at once; the app page then asks for
	 * sign-in or for what the company may do with it. */
	async function addCatalogue(app: App) {
		const entry = app.catalogue;
		if (!entry) return;
		await act(`add:${app.key}`, async () => {
			const { connection } = await addConnection(companyId, {
				kind: 'remote',
				name: entry.key,
				endpoint: entry.endpoint,
				source: `catalogue:${entry.key}`
			});
			await goto(route(`c-${connection.name}`));
		});
	}

	async function addLink(event: SubmitEvent) {
		event.preventDefault();
		const value = link.trim();
		if (!value) return;
		await act('link', async () => {
			const kind = classifyLink(value);
			if (kind === 'plugin') {
				const result = await importPlugin(companyId, value);
				const parts = [];
				if (result.import.connections.length)
					parts.push(
						`${result.import.connections.length} service${result.import.connections.length === 1 ? '' : 's'}`
					);
				if (result.import.skills.length)
					parts.push(
						`${result.import.skills.length} piece${result.import.skills.length === 1 ? '' : 's'} of know-how for you to review`
					);
				notice = `${result.import.plugin} added${parts.length ? `: ${parts.join(' and ')}` : ''}.`;
				linking = false;
				link = '';
				await data.load();
				return;
			}
			const name = nameForLink(value);
			const { connection } =
				kind === 'remote'
					? await addConnection(companyId, { kind: 'remote', name, endpoint: value })
					: await addConnection(companyId, {
							kind: 'local',
							name,
							command: value.split(/\s+/)[0],
							args: value.split(/\s+/).slice(1)
						});
			linking = false;
			link = '';
			await goto(route(`c-${connection.name}`));
		});
	}

	const tone = (app: App) =>
		app.state === 'needs_you' ? 'warning' : app.state === 'paused' ? 'muted' : 'success';
</script>

<CompanyTitle title="Apps" {companyId} />

{#snippet mark(app: App)}
	<AppMark
		name={app.name}
		catalogueKey={app.catalogue?.key}
		knowHow={app.category === 'Know-how'}
		size={24}
	/>
{/snippet}

{#snippet row(app: App)}
	<Item title={app.name} meta={app.description} href={route(app.key)} dim={app.state === 'paused'}>
		{#snippet leading()}{@render mark(app)}{/snippet}
		{#snippet trailing()}
			<span title={app.howTip}><Dot tone={tone(app)} label={app.attention ?? 'In use'} show /></span
			>
		{/snippet}
	</Item>
{/snippet}

<Page
	{title}
	info="Apps give the company new abilities: services it can use and know-how it can follow. Each one shows exactly what it may do, and you can freeze or remove it at any time."
>
	{#snippet actions()}
		<input
			class="search"
			type="search"
			bind:value={search}
			placeholder="Search apps"
			aria-label="Search apps"
		/>
		{#if !linking}<button
				class="btn small"
				type="button"
				title="Add an app from a link: a service's MCP address, a plugin or skill on GitHub, or a command"
				onclick={() => (linking = true)}>Add from a link</button
			>{/if}
	{/snippet}

	<!-- On a phone the title keeps the header; the search moves into the page. -->
	<input
		class="search narrow"
		type="search"
		bind:value={search}
		placeholder="Search apps"
		aria-label="Search apps"
	/>
	{#if failure}<Notice
			tone="danger"
			title={actionFailure ? 'That app was not added' : 'Apps could not be read'}
			details={failure}
		/>{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}

	{#if linking}
		<form class="link" onsubmit={addLink}>
			<!-- svelte-ignore a11y_autofocus -->
			<input
				aria-label="Link to an app"
				placeholder="https://mcp.example.com/mcp, a GitHub link, or a command"
				autocomplete="off"
				autofocus
				required
				bind:value={link}
			/>
			<button class="btn small primary" type="submit" disabled={busy === 'link'}
				>{busy === 'link' ? 'Checking…' : 'Add'}</button
			>
			<button class="btn small ghost" type="button" onclick={() => (linking = false)}>Cancel</button
			>
		</form>
	{/if}

	{#if !data.ready}
		{#if !data.failure}<Skeleton label="Reading apps…" variant="page" count={4} />{/if}
	{:else}
		{#if showInUse}
			{#if requests.length || needsYou.length}
				<Section
					title="Waiting on you"
					count={requests.length + needsYou.length}
					info="Apps Exec asked for to unblock work, and ones that need a sign-in, a review or a decision."
				>
					{#each requests as request (request.handoff_id)}
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
								<a class="btn small primary" href={request.href(companyId)}>Add</a>
							{/snippet}
						</Item>
					{/each}
					{#each needsYou as app (app.key)}
						<Item title={app.name} meta={app.description} href={route(app.key)} unread>
							{#snippet leading()}{@render mark(app)}{/snippet}
							{#snippet trailing()}
								<span title={app.howTip}
									><Dot tone="warning" label={app.attention ?? 'Needs you'} show /></span
								>
							{/snippet}
						</Item>
					{/each}
				</Section>
			{/if}

			{#if added.length || !query}
				<Section
					title="Added"
					count={added.length}
					info="Services and know-how the company added. Each shows what it may do."
				>
					{#each added as app (app.key)}{@render row(app)}{:else}
						<Empty
							compact
							title="Nothing added yet"
							info="Browse the apps in the list, or ask Exec: it finds what the work needs and brings it here."
						>
							{#snippet action()}<a class="btn small" href="?view=all">Browse apps</a>{/snippet}
						</Empty>
					{/each}
				</Section>
			{/if}

			{#if included.length}
				<Section
					title="Comes with Restless"
					count={included.length}
					info="Know-how every company starts with. It needs no sign-in."
				>
					{#each included as app (app.key)}{@render row(app)}{/each}
				</Section>
			{/if}
		{/if}

		{#if tiles.length || !showInUse}
			<Section
				title={query ? 'Add' : undefined}
				count={query ? tiles.length : undefined}
				group={false}
			>
				<div class="grid">
					{#each tiles as app (app.key)}
						<AppTile
							{app}
							href={route(app.key)}
							busy={busy === `add:${app.key}`}
							disabled={!!busy}
							onadd={() => void addCatalogue(app)}
						/>
					{:else}
						<Empty compact title="Nothing here yet" info="Add one from a link instead." />
					{/each}
				</div>
			</Section>
		{/if}

		{#if query && !requests.length && !needsYou.length && !added.length && !included.length && !tiles.length}
			<Empty
				compact
				title="No apps match your search"
				info="Try another word, or add one from a link."
			/>
		{/if}
	{/if}
</Page>

<style>
	.search {
		width: 200px;
		height: 30px;
	}
	.search.narrow {
		display: none;
		width: 100%;
	}
	.link {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		gap: var(--space-2);
		align-items: center;
		padding: 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
	}
	.asked {
		color: var(--intent-authority);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: 10px;
	}
	@container page (max-width: 560px) {
		.search {
			display: none;
		}
		.search.narrow {
			display: block;
		}
		.link {
			grid-template-columns: 1fr;
		}
	}
</style>
