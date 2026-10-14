<script lang="ts">
	/* Apps, ask first. The default view is one box: say what the company should be able to do,
	 * or type a name. It searches as you type (your apps, then ones to add), and when nothing is
	 * the answer, Exec is: it finds the app and brings a one-click card back here. Below it, what
	 * the company can do, in plain words. Power users have the sidebar's views: a dense table of
	 * what is in use, by kind, and the full catalogue by category. */
	import { composing } from '$lib/primitives/composer-keys';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Notice, Empty, Dot } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { tick } from 'svelte';
	import { addConnection, importPlugin } from '$lib/model/connections';
	import { sendActorMessage } from '$lib/model/attention';
	import { composeSkillMd, createSkill, setSkillDisposition, skillSlug } from '$lib/model/skills';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import {
		KIND_INFO,
		KIND_LABEL,
		KIND_SECTION,
		appLevel,
		builtIn,
		classifyLink,
		connectTag,
		nameForLink,
		unlistedFor,
		type App,
		type UnlistedService
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
	let query = $state('');
	let highlighted = $state(0);
	let askInput: HTMLInputElement | undefined = $state();
	let link = $state('');
	let linkDismissed = $state(false);
	let skippedImport = $state<string[]>([]);
	let reviewHref = $state('');
	let skillDismissed = $state(false);
	let skillTitle = $state('');
	let skillWhen = $state('');
	let skillSteps = $state('');

	const view = $derived<AppsView>(viewFrom(page.url.searchParams.get('view')) ?? 'ask');
	const linking = $derived(owner && page.url.searchParams.get('add') === 'link' && !linkDismissed);
	const writing = $derived(
		owner && page.url.searchParams.get('add') === 'skill' && !skillDismissed
	);
	const route = (key: string) =>
		`/${encodeURIComponent(companyId)}/apps/${encodeURIComponent(key)}`;
	const failure = $derived(actionFailure || data.failure);
	let retrying = $state(false);
	async function retry() {
		retrying = true;
		try {
			await data.load();
		} finally {
			retrying = false;
		}
	}

	const TITLES: Record<string, string> = {
		ask: 'Apps',
		waiting: 'Waiting on you',
		'in-use': 'In use',
		removed: 'Removed',
		all: 'All apps'
	};
	const title = $derived(TITLES[view] ?? (isKind(view) ? KIND_SECTION[view] : view));

	/* What the box offers as you type: your apps first, then ones to add, then a pasted link,
	 * and always, last, asking Exec. Enter takes the highlighted one. */
	type Option =
		| { id: string; kind: 'open'; app: App }
		| { id: string; kind: 'add'; app: App }
		| { id: string; kind: 'link'; value: string }
		| { id: string; kind: 'unlisted'; service: UnlistedService }
		| { id: string; kind: 'ask'; value: string };
	const term = $derived(query.trim());
	const matches = (app: App) =>
		`${app.name} ${app.description} ${app.category}`.toLowerCase().includes(term.toLowerCase());
	const options = $derived.by<Option[]>(() => {
		if (!term) return [];
		const mine = data.inUse.filter(matches).slice(0, 4);
		const add = owner ? data.browse.filter(matches).slice(0, 4) : [];
		return [
			...mine.map((app) => ({ id: `open:${app.key}`, kind: 'open' as const, app })),
			...add.map((app) => ({ id: `add:${app.key}`, kind: 'add' as const, app })),
			...(owner && /^https?:\/\/\S+$/i.test(term)
				? [{ id: 'link', kind: 'link' as const, value: term }]
				: []),
			...unlistedFor(term).map((service) => ({
				id: `unlisted:${service.key}`,
				kind: 'unlisted' as const,
				service
			})),
			{ id: 'ask', kind: 'ask' as const, value: term }
		];
	});
	$effect(() => {
		void term;
		highlighted = 0;
	});

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

	/* Adding a known service probes it and lands on its page: a sign-in when it needs one,
	 * otherwise the list of what it can do, which the owner allows from there. A service that
	 * needs a token goes to its Add step first, where the token is chosen or pasted. */
	async function addCatalogue(app: App) {
		const entry = app.catalogue;
		if (!entry) return;
		if (entry.auth === 'token') {
			await goto(route(entry.key));
			return;
		}
		await act(
			`add:${app.key}`,
			async () => {
				const { connection } = await addConnection(companyId, {
					kind: 'remote',
					name: entry.key,
					endpoint: entry.endpoint,
					source: `catalogue:${entry.key}`
				});
				await goto(route(`c-${connection.name}`));
			},
			'That app was not added.'
		);
	}

	/* An ask the catalogue cannot answer with one click says so: email has its own path, and
	 * Exec looks for a working server for the rest. */
	function chooseUnlisted(service: UnlistedService) {
		if (service.path === 'email') {
			void goto(`/${encodeURIComponent(companyId)}/company/resources#email`);
			return;
		}
		void askExec(
			`connect ${service.name} (${term}). The catalogue has no checked address for it, so find a working MCP server or another way`
		);
	}

	async function writeSkill(event: SubmitEvent) {
		event.preventDefault();
		const name = skillSlug(skillTitle);
		if (!name || !skillWhen.trim() || !skillSteps.trim()) return;
		await act(
			'skill',
			async () => {
				await createSkill(companyId, composeSkillMd(name, skillWhen, skillSteps));
				skillTitle = '';
				skillWhen = '';
				skillSteps = '';
				skillDismissed = true;
				await data.load();
				await goto(route(`s-${name}`));
			},
			'That know-how was not saved.'
		);
	}

	async function restore(app: App) {
		const skill = app.skills[0];
		if (!skill) return;
		await act(
			`restore:${app.key}`,
			async () => {
				await setSkillDisposition(companyId, skill.name, 'accepted');
				await data.load();
				toast(`${app.name} is company know-how again.`);
			},
			'That know-how was not restored.'
		);
	}

	async function addFromLink(value: string) {
		value = value.trim();
		if (!value) return;
		await act(
			'link',
			async () => {
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
							`${result.import.skills.length === 1 ? 'one piece' : `${result.import.skills.length} pieces`} of know-how for you to review`
						);
					notice = `${result.import.plugin} added${parts.length ? `: ${parts.join(' and ')}` : ''}.`;
					skippedImport = result.import.skipped;
					/* Its services and know-how wait on the owner; Exec adds the know-how shortly. */
					reviewHref =
						result.import.connections.length || result.import.skills.length
							? `/${encodeURIComponent(companyId)}/apps?view=waiting`
							: '';
					link = '';
					query = '';
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
				await goto(route(`c-${connection.name}`));
			},
			'That link was not added.'
		);
	}

	/* Exec matches the need to an app (its sourcing know-how) and brings back one card. */
	async function askExec(value: string) {
		await act(
			'ask',
			async () => {
				await sendActorMessage(
					companyId,
					'exec',
					`The company should be able to: ${value}. Find the app that does this and bring it to ${owner ? 'me' : 'the owner'} to add, or tell me if nothing fits.`,
					undefined,
					[],
					`/${companyId}/apps`
				);
				notice = owner
					? `Asked Exec. Its answer comes to your conversation, and anything to add waits here.`
					: `Asked Exec. Its answer comes to your conversation; anything to add waits here for the owner.`;
				query = '';
			},
			'Exec was not asked. Try again.'
		);
	}

	function choose(option: Option | undefined) {
		if (!option) return;
		if (option.kind === 'open') void goto(route(option.app.key));
		else if (option.kind === 'add') void addCatalogue(option.app);
		else if (option.kind === 'link') void addFromLink(option.value);
		else if (option.kind === 'unlisted') chooseUnlisted(option.service);
		else void askExec(option.value);
	}

	function onAskKey(event: KeyboardEvent) {
		if (!options.length || composing(event)) return;
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			const step = event.key === 'ArrowDown' ? 1 : -1;
			highlighted = (highlighted + step + options.length) % options.length;
		} else if (event.key === 'Enter') {
			event.preventDefault();
			choose(options[highlighted]);
		} else if (event.key === 'Escape') {
			query = '';
		}
	}

	/* "/" focuses the box from anywhere on Apps, as search does in Linear. */
	function onWindowKey(event: KeyboardEvent) {
		if (event.key !== '/' || event.metaKey || event.ctrlKey || event.altKey) return;
		const target = event.target as HTMLElement | null;
		if (target?.closest('input, textarea, select, [contenteditable]')) return;
		if (view !== 'ask') return;
		event.preventDefault();
		askInput?.focus();
	}

	$effect(() => {
		if (linking) void tick().then(() => document.getElementById('apps-link')?.focus());
	});
	$effect(() => {
		if (writing) void tick().then(() => document.getElementById('skill-title')?.focus());
	});

	const tone = (app: App) =>
		app.state === 'needs_you' ? 'warning' : app.state === 'paused' ? 'muted' : 'success';
	const status = (app: App) => app.attention ?? (app.state === 'paused' ? 'Paused' : 'In use');

	const added = $derived(data.added);
	const included = $derived(data.included);
	const tableApps = $derived(
		view === 'in-use' ? data.inUse : isKind(view) ? data.ofKind(view) : []
	);
	const tiles = $derived(
		view === 'all'
			? data.browse
			: !isKind(view) && !['ask', 'waiting', 'in-use', 'removed'].includes(view)
				? data.inCategory(view as never)
				: []
	);
	const showWaiting = $derived(view === 'ask' || view === 'waiting');
	/* The public directory runs to thousands: search it, and show at most a screenful. */
	const TILE_LIMIT = 60;
	let tileQuery = $state('');
	const matchingTiles = $derived.by(() => {
		const terms = tileQuery.toLowerCase().split(/\s+/).filter(Boolean);
		return terms.length
			? tiles.filter((app) => {
					const text = `${app.name} ${app.description}`.toLowerCase();
					return terms.every((term) => text.includes(term));
				})
			: tiles;
	});
	const shownTiles = $derived(matchingTiles.slice(0, TILE_LIMIT));
</script>

<svelte:window onkeydown={onWindowKey} />

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

<Page
	{title}
	info={view === 'ask' && !owner
		? 'What the company can do with its apps, and how much it may do without asking. Search them, or ask Exec what would fit; adding an app is the owner’s.'
		: view === 'ask'
			? 'Say what the company should be able to do, or type an app’s name. Exec finds what fits and brings it here to add. Each app shows what it may do, and asks you before it acts the first time.'
			: view === 'removed'
				? 'Know-how you removed. Agents no longer follow it; restoring it makes it company know-how again.'
				: isKind(view)
					? KIND_INFO[view]
					: undefined}
>
	{#if failure}<Notice
			tone="danger"
			title={actionFailure ? actionFailure : 'Apps could not be read'}
			details={actionFailure ? null : failure}
		/>{/if}
	{#if !failure && data.partial}<Notice
			tone="warning"
			title={data.partial}
			details="What did load is shown. Nothing was removed."
			actions={retryAction}
		/>{/if}
	{#if notice}<Notice
			tone="success"
			title={notice}
			details={skippedImport.length ? `Not added: ${skippedImport.join('; ')}.` : null}
			actions={reviewHref ? review : undefined}
		/>{/if}

	{#if writing}
		<form class="skill-form" onsubmit={writeSkill}>
			<input
				id="skill-title"
				aria-label="Name"
				placeholder="Name, such as Reply to refund requests"
				title="A short name. It becomes the know-how’s id, in lowercase words joined by hyphens."
				autocomplete="off"
				required
				bind:value={skillTitle}
			/>
			<input
				aria-label="When to use it"
				placeholder="When to use it"
				title="One sentence agents read to decide whether this know-how fits what they are doing."
				autocomplete="off"
				required
				bind:value={skillWhen}
			/>
			<textarea
				aria-label="How to do it"
				placeholder="How to do it: the steps, the standard, and what to avoid"
				title="The instructions agents follow, in plain words or Markdown."
				rows="8"
				required
				bind:value={skillSteps}></textarea>
			<div class="skill-actions">
				<button
					class="btn small primary"
					type="submit"
					disabled={busy === 'skill' || !skillSlug(skillTitle)}
					title="Saves it as know-how to review. It is followed once you accept it."
					>{busy === 'skill' ? 'Saving…' : 'Save for review'}</button
				>
				<button class="btn small ghost" type="button" onclick={() => (skillDismissed = true)}
					>Cancel</button
				>
			</div>
		</form>
	{/if}

	{#if linking}
		<form
			class="link"
			onsubmit={(event) => {
				event.preventDefault();
				void addFromLink(link);
			}}
		>
			<input
				id="apps-link"
				aria-label="Link to an app"
				placeholder="https://mcp.example.com/mcp, a GitHub link, or a command"
				autocomplete="off"
				required
				bind:value={link}
			/>
			<button class="btn small primary" type="submit" disabled={busy === 'link'}
				>{busy === 'link' ? 'Checking…' : 'Add'}</button
			>
			<button class="btn small ghost" type="button" onclick={() => (linkDismissed = true)}
				>Cancel</button
			>
		</form>
	{/if}

	{#if view === 'ask'}
		<div class="ask">
			<div class="ask-box">
				<svg
					width="18"
					height="18"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					aria-hidden="true"
					><circle cx="11" cy="11" r="7"></circle><path d="m20 20-3.5-3.5"></path></svg
				>
				<input
					bind:this={askInput}
					bind:value={query}
					onkeydown={onAskKey}
					role="combobox"
					aria-expanded={options.length > 0}
					aria-label="Ask for an ability or search apps"
					aria-controls="ask-options"
					aria-activedescendant={options.length ? `ask-${options[highlighted]?.id}` : undefined}
					placeholder="What should the company be able to do?"
					autocomplete="off"
					disabled={busy === 'ask'}
				/>
				<kbd title="Press / to search from anywhere on Apps">/</kbd>
			</div>
			{#if options.length}
				<ul class="ask-options" id="ask-options" role="listbox" aria-label="Results">
					{#each options as option, index (option.id)}
						{@const first = index === 0 || options[index - 1].kind !== option.kind}
						{#if first && option.kind === 'open'}<li class="ask-label" role="presentation">
								Your apps
							</li>{/if}
						{#if first && option.kind === 'add'}<li class="ask-label" role="presentation">
								Add
							</li>{/if}
						<li
							id={`ask-${option.id}`}
							role="option"
							aria-selected={index === highlighted}
							class:on={index === highlighted}
							class:exec={option.kind === 'ask'}
							onmouseenter={() => (highlighted = index)}
							onmousedown={(event) => {
								event.preventDefault();
								choose(option);
							}}
						>
							{#if option.kind === 'open' || option.kind === 'add'}
								{@render mark(option.app)}
								<strong>{option.app.name}</strong>
								<span class="ask-what">{option.app.description}</span>
								<span class="ask-go" title={KIND_INFO[option.app.kind]}
									>{option.kind === 'open'
										? 'Open'
										: busy === `add:${option.app.key}`
											? 'Adding…'
											: 'Add'}</span
								>
							{:else if option.kind === 'unlisted'}
								<span class="ask-glyph" aria-hidden="true"
									>{option.service.path === 'email' ? '@' : '?'}</span
								>
								<strong>{option.service.name}</strong>
								<span class="ask-what" title={option.service.note}>{option.service.note}</span>
								<span class="ask-go"
									>{option.service.path === 'email' ? 'Email settings' : 'Ask Exec'}</span
								>
							{:else if option.kind === 'link'}
								<span class="ask-glyph" aria-hidden="true">↗</span>
								<span>Add from this link</span>
								<span class="ask-go"
									>{classifyLink(option.value) === 'plugin' ? 'Plugin' : 'Service'}</span
								>
							{:else}
								<span class="ask-glyph exec-mark" aria-hidden="true">E</span>
								<span>Ask Exec: <strong>“{option.value}”</strong></span>
								<span class="ask-go"
									>{busy === 'ask' ? 'Asking…' : index === highlighted ? '↵' : ''}</span
								>
							{/if}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}

	{#if !data.ready}
		{#if !data.failure}<Skeleton label="Reading apps…" variant="page" count={4} />{/if}
	{:else}
		{#if showWaiting && (data.requests.length || data.needsYou.length)}
			<Section
				title={view === 'ask' ? 'Waiting on you' : undefined}
				count={view === 'ask' ? data.waiting : undefined}
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

		{#if view === 'ask'}
			<Section
				title="The company can"
				count={added.length || undefined}
				info="What the company can do with the apps it has, and how much it may do without asking."
			>
				{#each added as app (app.key)}
					{@const level = appLevel(app)}
					<Item
						title={app.name}
						meta={app.description}
						href={route(app.key)}
						dim={app.state === 'paused'}
					>
						{#snippet leading()}{@render mark(app)}{/snippet}
						{#snippet trailing()}
							<span class="level" title={level.info}>{level.label}</span>
						{/snippet}
					</Item>
				{:else}
					<Empty
						compact
						title="Nothing added yet"
						info="Say what the company should be able to do above. Exec also brings apps here when work needs one."
					/>
				{/each}
			</Section>
			{#if included.length}
				<p class="included">
					Plus <a href="?view=skill" title="Skills: instructions agents follow"
						>{included.length === 1 ? 'one piece' : `${included.length} pieces`} of know-how</a
					> that come with Restless.
				</p>
			{/if}
		{/if}

		{#if view === 'in-use' || isKind(view)}
			{#if tableApps.length}
				<div class="table" role="table" aria-label={title}>
					<div class="tr head" role="row">
						<span role="columnheader">App</span>
						<span role="columnheader">Kind</span>
						<span role="columnheader" class="wide">Can do</span>
						<span role="columnheader">Permission</span>
						<span role="columnheader">Status</span>
					</div>
					{#each tableApps as app (app.key)}
						{@const level = appLevel(app)}
						<a class="tr" role="row" href={route(app.key)} class:dim={app.state === 'paused'}>
							<span role="cell" class="name"
								>{@render mark(app)}<strong>{app.name}</strong>{#if builtIn(app)}<small
										title="Comes with Restless">Built in</small
									>{/if}</span
							>
							<span role="cell" class="kind" title={KIND_INFO[app.kind]}
								>{KIND_LABEL[app.kind]}</span
							>
							<span role="cell" class="wide what">{app.description}</span>
							<span role="cell" title={level.info}>{level.label}</span>
							<span role="cell"><Dot tone={tone(app)} label={status(app)} show /></span>
						</a>
					{/each}
				</div>
			{:else}
				<Empty
					compact
					title={isKind(view) ? `No ${KIND_SECTION[view].toLowerCase()} yet` : 'Nothing in use yet'}
					info="Ask for what the company should be able to do, or browse."
				>
					{#snippet action()}<a class="btn small" href="?view=ask">Ask</a>{/snippet}
				</Empty>
			{/if}
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

		{#if tiles.length > TILE_LIMIT}
			<input
				class="tile-search"
				type="search"
				placeholder={`Search ${tiles.length.toLocaleString()} apps`}
				aria-label="Search these apps"
				bind:value={tileQuery}
			/>
		{/if}
		{#if tiles.length}
			<Section>
				{#each shownTiles as app (app.key)}
					<Item title={app.name} meta={app.description} href={route(app.key)}>
						{#snippet leading()}{@render mark(app)}{/snippet}
						{#snippet trailing()}
							<span class="level" title={app.howTip}>{connectTag(app)}</span>
							{#if owner && app.registrySkill}<a
									class="btn small"
									href={route(app.key)}
									aria-label={`Install ${app.name}`}
									title="Opens it to read what it does, then install">Install</a
								>{:else if owner}<button
									class="btn small"
									type="button"
									disabled={!!busy}
									aria-label={`Add ${app.name}`}
									title={app.catalogue?.auth === 'token'
										? `Opens ${app.name} to choose or paste the token it uses, then adds it.`
										: `Adds ${app.name} and checks it answers. Sign-in, then what it may do, come next.`}
									onclick={() => void addCatalogue(app)}
									>{busy === `add:${app.key}` ? 'Adding…' : 'Add'}</button
								>{/if}
						{/snippet}
					</Item>
				{:else}
					<Empty
						compact
						title="No app matches"
						info="Try other words, or ask Exec what would fit."
					/>
				{/each}
			</Section>
			{#if matchingTiles.length > shownTiles.length}
				<p class="tile-more">
					Showing {shownTiles.length} of {matchingTiles.length.toLocaleString()}. Search to find
					more.
				</p>
			{/if}
		{:else if view === 'all' || (!isKind(view) && !['ask', 'waiting', 'in-use', 'removed'].includes(view))}
			<Empty
				compact
				title="Everything here is already added"
				info={owner ? 'Add one from a link instead.' : undefined}
			/>
		{/if}
	{/if}
</Page>

<style>
	.tile-search {
		width: 100%;
		max-width: 420px;
		margin: 0 0 12px;
	}
	.tile-more {
		margin: 10px 0 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.ask {
		position: relative;
	}
	.ask-box {
		display: flex;
		align-items: center;
		gap: 10px;
		height: 52px;
		padding: 0 14px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		color: var(--text-tertiary);
		box-shadow: var(--shadow-soft);
		transition: border-color var(--motion-state) var(--ease-standard);
	}
	.ask-box:focus-within {
		border-color: var(--intent-conversation);
	}
	.ask-box input {
		flex: 1;
		min-width: 0;
		height: 100%;
		border: 0;
		outline: 0;
		background: transparent;
		color: var(--ink);
		font: inherit;
		font-size: var(--t-head);
	}
	.ask-box kbd {
		padding: 0 6px;
		border: 1px solid var(--border);
		border-radius: 4px;
		font: inherit;
		font-size: var(--t-label);
	}
	.ask-options {
		position: absolute;
		top: calc(100% + 6px);
		right: 0;
		left: 0;
		z-index: 20;
		display: grid;
		gap: 1px;
		margin: 0;
		padding: 6px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-float);
		list-style: none;
		animation: bridge-popover-in var(--motion-disclosure, 160ms) var(--ease-spring, ease-out) both;
	}
	.ask-label {
		padding: 6px 10px 2px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.ask-options [role='option'] {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
		height: 40px;
		padding: 0 10px;
		border-radius: var(--radius-control);
		cursor: pointer;
	}
	.ask-options [role='option'].on {
		background: var(--wash-hover, var(--surface-alt));
	}
	.ask-options [role='option'].exec.on {
		background: color-mix(in srgb, var(--intent-conversation) 10%, var(--surface-raised));
	}
	.ask-options strong {
		font-weight: 500;
	}
	.ask-what {
		min-width: 0;
		overflow: hidden;
		color: var(--text-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.ask-go {
		flex: none;
		margin-left: auto;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.ask-glyph {
		display: grid;
		flex: none;
		place-items: center;
		width: 22px;
		height: 22px;
		border-radius: 5px;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.exec-mark {
		background: var(--ink);
		color: var(--surface-raised);
	}
	.skill-form {
		display: grid;
		gap: var(--space-2);
		padding: 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
	}
	.skill-form textarea {
		resize: vertical;
		font: inherit;
	}
	.skill-actions {
		display: flex;
		gap: var(--space-2);
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
	.kind,
	.level {
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.level {
		padding: 2px 8px;
		border-radius: 999px;
		background: var(--surface-alt);
	}
	.included {
		margin: -6px 0 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}

	/* The power-user table: one row per app, its kind and permission at a glance. */
	.table {
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.tr {
		display: grid;
		grid-template-columns: minmax(160px, 1.2fr) 96px minmax(0, 2fr) 150px 120px;
		gap: 12px;
		align-items: center;
		min-height: 44px;
		padding: 0 14px;
		border-top: 1px solid var(--border);
		color: var(--ink);
		font-size: var(--t-body);
		text-decoration: none;
	}
	.tr.head {
		min-height: 34px;
		border-top: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	a.tr:hover {
		background: var(--wash-hover, var(--surface-alt));
	}
	a.tr:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	.tr.dim {
		color: var(--text-tertiary);
	}
	.tr .name {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}
	.tr .name strong {
		overflow: hidden;
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.tr .name small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.tr .what {
		overflow: hidden;
		color: var(--text-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.tr > span:nth-child(4) {
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	@container page (max-width: 720px) {
		.tr {
			grid-template-columns: minmax(0, 1fr) auto;
		}
		.tr > span:nth-child(2),
		.tr > .wide,
		.tr > span:nth-child(4) {
			display: none;
		}
	}
	@container page (max-width: 560px) {
		.link {
			grid-template-columns: 1fr;
		}
		.ask-what {
			display: none;
		}
	}
</style>
