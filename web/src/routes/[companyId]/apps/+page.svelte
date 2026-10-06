<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Notice, Empty, Dot, Fold } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import {
		addConnection,
		fetchConnections,
		importPlugin,
		type ToolConnection
	} from '$lib/model/connections';
	import { fetchSkillLibrary, type SkillLibrary } from '$lib/model/skills';
	import {
		BROWSE_CATEGORIES,
		buildApps,
		builtIn,
		classifyLink,
		monogram,
		nameForLink,
		type App,
		type AppCategory
	} from '$lib/model/apps';
	import { fetchAppRequests, type AppRequest } from '$lib/model/app-requests';
	import AppTile from './AppTile.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	let connections = $state<ToolConnection[] | null>(null);
	let library = $state<SkillLibrary | null>(null);
	let requests = $state<AppRequest[]>([]);
	let failure = $state('');
	let notice = $state('');
	let busy = $state('');
	let search = $state('');
	let category = $state<AppCategory | 'Popular'>('Popular');
	let linking = $state(false);
	let link = $state('');

	async function load() {
		failure = '';
		const [tools, skills, asked] = await Promise.allSettled([
			fetchConnections(companyId),
			fetchSkillLibrary(companyId),
			fetchAppRequests(companyId)
		]);
		if (tools.status === 'fulfilled') connections = tools.value;
		else failure = failureSentence(tools.reason, 'Apps could not be read.');
		if (skills.status === 'fulfilled') library = skills.value;
		requests = asked.status === 'fulfilled' ? asked.value : [];
	}

	$effect(() => {
		void companyId;
		void load();
	});

	const apps = $derived(
		connections
			? buildApps(
					connections,
					library,
					requests.map((request) => request.app)
				)
			: null
	);
	const matches = (app: App) =>
		!search || `${app.name} ${app.description}`.toLowerCase().includes(search.toLowerCase());
	const needsYou = $derived(
		apps?.mine.filter((app) => app.state === 'needs_you' && matches(app)) ?? []
	);
	const inUse = $derived(
		apps?.mine.filter((app) => app.state !== 'needs_you' && !builtIn(app) && matches(app)) ?? []
	);
	const included = $derived(
		apps?.mine.filter((app) => app.state !== 'needs_you' && builtIn(app) && matches(app)) ?? []
	);
	const browse = $derived(
		apps?.browse.filter(
			(app) => matches(app) && (category === 'Popular' || app.category === category)
		) ?? []
	);
	const route = (key: string) =>
		`/${encodeURIComponent(companyId)}/apps/${encodeURIComponent(key)}`;

	async function act(key: string, action: () => Promise<unknown>) {
		if (busy) return;
		busy = key;
		failure = '';
		notice = '';
		try {
			await action();
		} catch (cause) {
			failure = failureSentence(cause, 'That app was not added.');
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
				await load();
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

{#snippet tile(app: App)}
	<span class="mono" data-category={app.category} aria-hidden="true">{monogram(app.name)}</span>
{/snippet}

<Page
	title="Apps"
	info="Apps give the company new abilities: services it can use and know-how it can follow. Each one shows exactly what it may do, and you can freeze or remove it at any time."
>
	{#snippet actions()}
		{#if !linking}<button
				class="btn small"
				type="button"
				title="Add an app from a link: a service's MCP address, a plugin or skill on GitHub, or a command"
				onclick={() => (linking = true)}>Add from a link</button
			>{/if}
	{/snippet}

	<input
		class="search"
		type="search"
		bind:value={search}
		placeholder="Search apps"
		aria-label="Search apps"
	/>
	{#if failure}<Notice tone="danger" title="That did not work" details={failure} />{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}

	{#if linking}
		<form class="link" onsubmit={addLink}>
			<input
				aria-label="Link to an app"
				placeholder="https://mcp.example.com/mcp, a GitHub link, or a command"
				autocomplete="off"
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

	{#if !apps}
		{#if !failure}<Skeleton label="Reading apps…" variant="page" count={4} />{/if}
	{:else}
		{#if requests.length || needsYou.length}
			<Section
				title="Exec recommends"
				count={requests.length + needsYou.length}
				info="Apps waiting on you: ones Exec asked for to unblock work, and ones that need a sign-in, a review or a decision."
			>
				{#each requests as request (request.handoff_id)}
					<Item title={request.name} meta={request.reason} href={request.href(companyId)} unread>
						{#snippet leading()}<span class="mono" aria-hidden="true">{monogram(request.name)}</span
							>{/snippet}
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
						{#snippet leading()}{@render tile(app)}{/snippet}
						{#snippet trailing()}
							<span title={app.howTip}
								><Dot tone="warning" label={app.attention ?? 'Needs you'} show /></span
							>
						{/snippet}
					</Item>
				{/each}
			</Section>
		{/if}

		<Section title="In use" count={inUse.length}>
			{#each inUse as app (app.key)}
				<Item
					title={app.name}
					meta={app.description}
					href={route(app.key)}
					dim={app.state === 'paused'}
				>
					{#snippet leading()}{@render tile(app)}{/snippet}
					{#snippet trailing()}
						<span title={app.howTip}
							><Dot tone={tone(app)} label={app.attention ?? 'In use'} show /></span
						>
					{/snippet}
				</Item>
			{:else}
				<Empty
					compact
					title={search ? 'No apps match your search' : 'No apps yet'}
					info="Add one below, or ask Exec: it finds what the work needs and brings it here."
				/>
			{/each}
			{#if included.length}
				<Fold label={`Comes with Restless · ${included.length}`}>
					{#each included as app (app.key)}
						<Item
							title={app.name}
							meta={app.description}
							href={route(app.key)}
							dim={app.state === 'paused'}
						>
							{#snippet leading()}{@render tile(app)}{/snippet}
							{#snippet trailing()}
								<span title={app.howTip}
									><Dot tone={tone(app)} label={app.attention ?? 'In use'} show /></span
								>
							{/snippet}
						</Item>
					{/each}
				</Fold>
			{/if}
		</Section>

		<Section title="Browse" count={browse.length}>
			<div class="categories" role="group" aria-label="Categories">
				{#each ['Popular', ...BROWSE_CATEGORIES] as name (name)}
					<button
						type="button"
						class="chip"
						aria-pressed={category === name}
						onclick={() => (category = name as AppCategory | 'Popular')}>{name}</button
					>
				{/each}
			</div>
			<div class="grid">
				{#each browse as app (app.key)}
					<AppTile
						{app}
						busy={busy === `add:${app.key}`}
						disabled={!!busy}
						onadd={() => void addCatalogue(app)}
					/>
				{:else}
					<Empty
						compact
						title="Nothing here yet"
						info="Try another category, or add one from a link."
					/>
				{/each}
			</div>
		</Section>
	{/if}
</Page>

<style>
	.search {
		width: min(320px, 100%);
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
	.mono {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 24px;
		height: 24px;
		border-radius: 7px;
		background: var(--accent-soft);
		color: var(--accent-strong);
		font-size: var(--t-label);
		font-weight: 600;
		flex-shrink: 0;
	}
	.mono[data-category='Know-how'] {
		background: var(--intent-feedback-soft);
		color: var(--intent-feedback);
	}
	.asked {
		color: var(--intent-authority);
	}
	/* One row that scrolls sideways on a phone rather than stacking. */
	.categories {
		display: flex;
		gap: 6px;
		margin-bottom: var(--space-2);
		overflow-x: auto;
		scrollbar-width: none;
	}
	.categories::-webkit-scrollbar {
		display: none;
	}
	.chip {
		flex-shrink: 0;
	}
	.chip {
		height: 28px;
		padding: 0 10px;
		border: 1px solid var(--border-strong);
		border-radius: 14px;
		background: var(--surface);
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-body);
		cursor: pointer;
	}
	.chip[aria-pressed='true'] {
		background: var(--accent-strong);
		border-color: var(--accent-strong);
		color: var(--text-inverse);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: var(--space-2);
	}
	@container page (max-width: 560px) {
		.search {
			width: 100%;
		}
		.link {
			grid-template-columns: 1fr;
		}
	}
</style>
