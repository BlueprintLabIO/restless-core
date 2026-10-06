<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Notice, Toggle, Segmented, Dot } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import {
		CLASS_LABEL,
		addConnection,
		currentClasses,
		disconnectConnection,
		fetchConnections,
		fetchReceipts,
		freezeConnection,
		grantTools,
		probeConnection,
		revokeGrant,
		signIn,
		statusLabel,
		type GrantedTool,
		type ToolClass,
		type ToolConnection,
		type ToolReceipts
	} from '$lib/model/connections';
	import {
		assignSkill,
		fetchSkillLibrary,
		setSkillDisposition,
		type SkillLibrary,
		type SkillRow
	} from '$lib/model/skills';
	import { buildApps, nameForLink, skillName, type App } from '$lib/model/apps';
	import AppMark from '$lib/primitives/AppMark.svelte';
	import { fetchAppRequests, type AppRequest } from '$lib/model/app-requests';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const key = $derived(decodeURIComponent(page.params.app ?? ''));
	const requestId = $derived(page.url.searchParams.get('request'));
	const appsHref = $derived(`/${encodeURIComponent(companyId)}/apps`);

	let connections = $state<ToolConnection[] | null>(null);
	let library = $state<SkillLibrary | null>(null);
	let request = $state<AppRequest | null>(null);
	let receipts = $state<Record<string, ToolReceipts>>({});
	let failure = $state('');
	let notice = $state('');
	let busy = $state('');
	let draft = $state<Record<string, ToolClass>>({});
	let vaultSecret = $state('');

	async function load() {
		failure = '';
		try {
			const [tools, skills] = await Promise.all([
				fetchConnections(companyId),
				fetchSkillLibrary(companyId).catch(() => null)
			]);
			connections = tools;
			library = skills;
			if (requestId) {
				const requests = await fetchAppRequests(companyId).catch(() => []);
				request = requests.find((row) => row.handoff_id === requestId) ?? null;
			}
			for (const connection of app?.connections ?? []) {
				if (connection.status === 'working')
					receipts[connection.name] = await fetchReceipts(companyId, connection.name).catch(() => ({
						reads: [],
						effects: []
					}));
			}
		} catch (cause) {
			failure = failureSentence(cause, 'This app could not be read.');
		}
	}

	$effect(() => {
		void companyId;
		void key;
		void load();
	});

	const apps = $derived(connections ? buildApps(connections, library) : null);
	/* A catalogue key or a pasted link names an app that is not added yet. */
	const app = $derived<App | undefined>(
		apps?.mine.find((candidate) => candidate.key === key) ??
			apps?.mine.find((candidate) => candidate.catalogue?.key === key) ??
			apps?.browse.find((candidate) => candidate.key === key)
	);
	const pendingLink = $derived(key.startsWith('link:') ? key.slice('link:'.length) : '');

	async function act(id: string, action: () => Promise<unknown>, done = '') {
		if (busy) return;
		busy = id;
		failure = '';
		notice = '';
		try {
			await action();
			notice = done;
			await load();
		} catch (cause) {
			failure = failureSentence(cause, 'That change was not made.');
		} finally {
			busy = '';
		}
	}

	async function add() {
		const entry = app?.catalogue;
		await act('add', async () => {
			const { connection } = entry
				? await addConnection(companyId, {
						kind: 'remote',
						name: entry.key,
						endpoint: entry.endpoint,
						source: `catalogue:${entry.key}`,
						auth:
							entry.auth === 'token'
								? { type: 'bearer', credential: vaultSecret.trim() }
								: { type: 'none' }
					})
				: await addConnection(companyId, {
						kind: 'remote',
						name: nameForLink(pendingLink),
						endpoint: pendingLink
					});
			await goto(
				`${appsHref}/${encodeURIComponent(`c-${connection.name}`)}${requestId ? `?request=${requestId}` : ''}`,
				{ replaceState: true }
			);
		});
	}

	async function startSignIn(connection: ToolConnection) {
		await act(`signin:${connection.name}`, async () => {
			window.location.assign(await signIn(companyId, connection.name));
		});
	}

	const classOf = (connection: ToolConnection, tool: string): ToolClass =>
		draft[`${connection.name}/${tool}`] ?? currentClasses(connection)[tool]?.class ?? 'acts';
	const granted = (connection: ToolConnection) =>
		connection.grants.some((grant) => grant.grantee === '*');

	async function allow(connection: ToolConnection) {
		const classes = currentClasses(connection);
		const tools = connection.tools.map((tool) => {
			const base: GrantedTool | undefined = classes[tool.name];
			return {
				tool: tool.name,
				class: classOf(connection, tool.name),
				party_args: base?.party_args ?? []
			};
		});
		await act(
			`grant:${connection.name}`,
			async () => {
				await grantTools(companyId, connection.name, tools);
				draft = {};
			},
			request
				? `${app?.name ?? connection.name} is in use. The work that was waiting on it resumes on its own.`
				: `${app?.name ?? connection.name} is in use.`
		);
	}

	const grouped = (connection: ToolConnection) =>
		(['reads', 'acts', 'reserved'] as ToolClass[]).map((klass) => ({
			klass,
			tools: connection.tools.filter((tool) => classOf(connection, tool.name) === klass)
		}));
	const dirty = (connection: ToolConnection) =>
		Object.keys(draft).some((id) => id.startsWith(`${connection.name}/`)) ||
		connection.changed.length > 0;

	const offForEveryone = (skill: SkillRow) =>
		library?.assignments.some(
			(row) => row.skill_name === skill.name && row.scope === 'company' && !row.enabled
		) ?? false;
	const decide = (
		skill: SkillRow,
		disposition: 'accepted' | 'retired' | 'candidate',
		done: string
	) =>
		act(
			`${disposition}:${skill.name}`,
			() => setSkillDisposition(companyId, skill.name, disposition),
			done
		);

	const connectionTone = (connection: ToolConnection) =>
		connection.frozen
			? 'warning'
			: connection.status === 'working'
				? 'success'
				: connection.status === 'failed'
					? 'danger'
					: 'warning';
</script>

<CompanyTitle title={app?.name ?? 'App'} {companyId} />

{#snippet service(connection: ToolConnection)}
	<Section
		title={app && app.connections.length > 1 ? connection.name : 'Status'}
		info={connection.endpoint
			? `An MCP server at ${connection.endpoint}. The sign-in stays on your plane, never in the company computer.`
			: `Runs ${connection.command ?? ''} on your plane, outside the company computer.`}
	>
		<div class="status">
			<span title={connection.account ? `Signed in as ${connection.account}` : undefined}
				><Dot tone={connectionTone(connection)} label={statusLabel(connection)} show /></span
			>
			{#if connection.account}<span class="quiet">{connection.account}</span>{/if}
			<span class="spacer"></span>
			{#if connection.status === 'awaiting_sign_in' || connection.failure === 'sign_in_required'}
				<button
					class="btn small primary"
					type="button"
					disabled={!!busy}
					onclick={() => void startSignIn(connection)}
					>{busy === `signin:${connection.name}` ? 'Opening…' : `Sign in`}</button
				>
			{:else if connection.status !== 'working'}
				<button
					class="btn small"
					type="button"
					disabled={!!busy}
					onclick={() =>
						void act(
							`probe:${connection.name}`,
							() => probeConnection(companyId, connection.name),
							'Checked.'
						)}>{busy === `probe:${connection.name}` ? 'Checking…' : 'Check again'}</button
				>
			{/if}
			{#if connection.status === 'working'}
				<Toggle
					checked={connection.frozen}
					label="Freeze"
					title={connection.frozen
						? 'Frozen: anything that changes something is refused at once. Reading still works.'
						: 'Freeze to refuse every call that changes something, at once. Reading keeps working.'}
					disabled={!!busy}
					onchange={() =>
						void act(
							`freeze:${connection.name}`,
							() => freezeConnection(companyId, connection.name, !connection.frozen),
							connection.frozen ? 'It can act again.' : 'Frozen.'
						)}
				/>
				<span class="quiet">Freeze</span>
			{/if}
		</div>
	</Section>

	{#if connection.status === 'working'}
		<Section
			title={granted(connection) ? 'What the company may do' : 'Choose what the company may do'}
			info="Each tool is either used freely, used with a receipt (and asks you before reaching anyone new), or asks you every time. Changes apply to the next call."
		>
			{#snippet actions()}
				{#if !granted(connection) || dirty(connection)}<button
						class="btn small primary"
						type="button"
						disabled={!!busy}
						onclick={() => void allow(connection)}
						>{busy === `grant:${connection.name}`
							? 'Saving…'
							: granted(connection)
								? 'Save'
								: 'Allow'}</button
					>{/if}
			{/snippet}
			{#each grouped(connection) as group (group.klass)}
				{#if group.tools.length}
					<h4 title={CLASS_LABEL[group.klass].title}>{CLASS_LABEL[group.klass].label}</h4>
					<ul class="tools">
						{#each group.tools as tool (tool.name)}
							<li>
								<span
									class="tool"
									class:changed={connection.changed.includes(tool.name)}
									title={connection.changed.includes(tool.name)
										? 'Changed upstream since you allowed it; it is unavailable until you save again.'
										: (tool.description ?? tool.name)}>{tool.title ?? skillName(tool.name)}</span
								>
								<Segmented
									label={`What ${tool.name} may do`}
									value={classOf(connection, tool.name)}
									disabled={!!busy}
									options={(['reads', 'acts', 'reserved'] as ToolClass[]).map((value) => ({
										value,
										label: CLASS_LABEL[value].label,
										title: CLASS_LABEL[value].title
									}))}
									onchange={(value) =>
										(draft = { ...draft, [`${connection.name}/${tool.name}`]: value })}
								/>
							</li>
						{/each}
					</ul>
				{/if}
			{/each}
		</Section>

		{@const recent = receipts[connection.name]}
		{#if recent && (recent.effects.length || recent.reads.length)}
			<Section title="Recent">
				<ul class="receipts">
					{#each recent.effects.slice(0, 8) as effect (effect.id)}
						<li title={effect.purpose}>
							<Dot
								tone={effect.success ? 'success' : 'danger'}
								label={effect.success ? 'Done' : 'Failed'}
							/>
							<span
								>{skillName(effect.tool)}{effect.parties?.length
									? ` → ${effect.parties.join(', ')}`
									: ''}</span
							>
							<span class="quiet">{effect.actor}</span>
							<time>{new Date(effect.created_at).toLocaleString()}</time>
						</li>
					{/each}
					{#each recent.reads.slice(0, 5) as read (read.id)}
						<li>
							<Dot tone={read.status === 'complete' ? 'neutral' : 'danger'} label={read.status} />
							<span>{skillName(read.tool)}</span>
							<span class="quiet">{read.actor}</span>
							<time>{new Date(read.observed_at).toLocaleString()}</time>
						</li>
					{/each}
				</ul>
			</Section>
		{/if}
	{/if}
{/snippet}

{#snippet knowHow(skill: SkillRow)}
	{@const uses = library?.usage?.[skill.name] ?? 0}
	<Section title={app && app.skills.length > 1 ? skillName(skill.name) : 'Know-how'}>
		<p class="description">{skill.description || 'No description.'}</p>
		<div class="status">
			<span title="Work that explicitly chose this know-how" class="quiet">Used on {uses} Work</span
			>
			{#if skill.has_scripts}<span
					class="badge"
					title="Ships scripts. They run with an agent's ordinary computer access, never extra authority."
					>Scripts</span
				>{/if}
			<span class="spacer"></span>
			{#if skill.disposition === 'candidate'}
				<button
					class="btn small primary"
					type="button"
					disabled={!!busy}
					onclick={() =>
						void decide(skill, 'accepted', `${skillName(skill.name)} is now company know-how.`)}
					>Accept</button
				>
				<button
					class="btn small"
					type="button"
					disabled={!!busy}
					onclick={() => void decide(skill, 'retired', `${skillName(skill.name)} was declined.`)}
					>Decline</button
				>
			{:else if skill.disposition === 'accepted'}
				<Toggle
					checked={!offForEveryone(skill)}
					label={`${skillName(skill.name)} available to everyone`}
					title={offForEveryone(skill)
						? 'Off: only agents given it directly can use it'
						: 'On: every agent can use it'}
					disabled={!!busy}
					onchange={() =>
						void act(`assign:${skill.name}`, () =>
							assignSkill(
								companyId,
								skill.name,
								'company',
								'',
								offForEveryone(skill) ? null : false
							)
						)}
				/>
				<span class="quiet">Everyone</span>
				{#if skill.source !== 'builtin'}<button
						class="btn small ghost"
						type="button"
						disabled={!!busy}
						onclick={() => void decide(skill, 'retired', `${skillName(skill.name)} was removed.`)}
						>Remove</button
					>{/if}
			{/if}
		</div>
	</Section>
{/snippet}

{#if !apps}
	{#if failure}<Notice tone="danger" title="This app could not be read" details={failure} />
	{:else}<Skeleton label="Reading the app…" variant="page" count={3} />{/if}
{:else if !app && !pendingLink}
	<Page title="App not found">
		<Notice tone="warning" title="This app is not here" details="It may have been removed." />
		<a class="btn small" href={appsHref}>All apps</a>
	</Page>
{:else}
	<Page
		title={app?.name ?? pendingLink}
		info={app ? `${app.description} ${app.howTip}` : `An MCP server at ${pendingLink}.`}
	>
		{#snippet leading()}
			<AppMark
				name={app?.name ?? pendingLink}
				catalogueKey={app?.catalogue?.key}
				knowHow={app?.category === 'Know-how'}
				size={32}
			/>
		{/snippet}
		{#snippet actions()}
			{#if app?.connections.some((connection) => connection.status !== 'disconnected')}
				<button
					class="btn small ghost"
					type="button"
					disabled={!!busy}
					title="Signs out, removes the stored credential and stops every agent using it."
					onclick={() =>
						void act('disconnect', async () => {
							for (const connection of app?.connections ?? []) {
								if (granted(connection)) await revokeGrant(companyId, connection.name);
								await disconnectConnection(companyId, connection.name);
							}
							await goto(appsHref);
						})}>Remove</button
				>
			{/if}
		{/snippet}

		<a class="back" href={appsHref}>Apps</a>
		{#if failure}<Notice tone="danger" title="That change was not made" details={failure} />{/if}
		{#if notice}<Notice tone="success" title={notice} />{/if}
		{#if request}
			<p
				class="request"
				title={`${request.work_title ? `For “${request.work_title}”. ` : ''}Allowing it resumes that work on its own.`}
			>
				<span class="asked">{request.asker} asked</span>{request.reason}
			</p>
		{/if}

		{#if app && app.state === 'available'}
			<Section title="Add">
				<div class="status">
					<span title={app.howTip}>{app.how}</span>
					<span class="spacer"></span>
					{#if app.catalogue?.auth === 'token'}
						<input
							aria-label="Vault secret"
							placeholder="Vault secret name"
							title="The name of the Vault secret holding the token. The token itself never leaves your plane."
							bind:value={vaultSecret}
						/>
					{/if}
					<button
						class="btn small primary"
						type="button"
						disabled={!!busy || (app.catalogue?.auth === 'token' && !vaultSecret.trim())}
						onclick={() => void add()}>{busy === 'add' ? 'Adding…' : `Add ${app.name}`}</button
					>
				</div>
			</Section>
		{:else if !app && pendingLink}
			<Section title="Add">
				<div class="status">
					<span class="quiet">{pendingLink}</span>
					<span class="spacer"></span>
					<button
						class="btn small primary"
						type="button"
						disabled={!!busy}
						onclick={() => void add()}>{busy === 'add' ? 'Adding…' : 'Add'}</button
					>
				</div>
			</Section>
		{:else if app}
			{#each app.connections as connection (connection.name)}{@render service(connection)}{/each}
			{#each app.skills as skill (skill.name)}{@render knowHow(skill)}{/each}
			{#if app.plugin}
				<p class="quiet" title={app.plugin}>From {app.plugin.replace(/^https:\/\//, '')}</p>
			{/if}
		{/if}
	</Page>
{/if}

<style>
	.back {
		align-self: flex-start;
		color: var(--text-tertiary);
		font-size: var(--t-body);
		text-decoration: none;
	}
	.back::before {
		content: '← ';
	}
	.back:hover {
		color: var(--text-secondary);
	}
	.request {
		display: flex;
		gap: 10px;
		align-items: baseline;
		margin: 0;
		padding: 12px 14px;
		border: 1px solid color-mix(in srgb, var(--intent-authority) 30%, transparent);
		border-radius: var(--radius-lg);
		background: var(--intent-authority-soft);
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.5;
	}
	.asked {
		flex-shrink: 0;
		color: var(--intent-authority);
		font-weight: 500;
	}
	/* Section cards pad their Items; these blocks carry their own inset. */
	.status {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		align-items: center;
		padding: 10px 14px;
	}
	.spacer {
		flex-grow: 1;
	}
	.quiet {
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
	h4 {
		margin: 0;
		padding: 10px 14px 0;
		font-size: var(--t-body);
		font-weight: 500;
		color: var(--text-secondary);
	}
	.tools,
	.receipts {
		display: grid;
		gap: 6px;
		margin: 0;
		padding: 6px 14px 10px;
		list-style: none;
	}
	.tools li {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		justify-content: space-between;
	}
	.tool {
		font-size: var(--t-body);
	}
	.tool.changed {
		color: var(--intent-authority);
	}
	.receipts li {
		display: flex;
		gap: 8px;
		align-items: center;
		font-size: var(--t-body);
	}
	.receipts time {
		margin-left: auto;
		color: var(--text-tertiary);
	}
	.description {
		margin: 0;
		padding: 12px 14px 0;
		max-width: 72ch;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.6;
	}
	.badge {
		padding: 1px 6px;
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		font-size: var(--t-body);
	}
	@container page (max-width: 560px) {
		.tools li {
			flex-direction: column;
			align-items: flex-start;
		}
	}
</style>
