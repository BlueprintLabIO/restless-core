<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { hostPlace, planeStatus, type ApplianceStatus } from '$lib/model/appliance';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { Page, Section, Item, Notice, Empty, Toggle, Segmented, Dot } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import Plug from '@lucide/svelte/icons/plug';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import {
		CLASS_LABEL,
		SUGGESTIONS,
		addConnection,
		currentClasses,
		disconnectConnection,
		fetchConnections,
		fetchReceipts,
		freezeConnection,
		grantTools,
		importPlugin,
		probeConnection,
		revokeGrant,
		signIn,
		statusLabel,
		type GrantedTool,
		type NewConnection,
		type ToolClass,
		type ToolConnection,
		type ToolReceipts
	} from '$lib/model/connections';
	let plane = $state<ApplianceStatus | null>(null);
	void planeStatus().then((value) => (plane = value));

	const companyId = $derived(page.params.companyId ?? 'aris');
	let connections = $state<ToolConnection[] | null>(null);
	let failure = $state('');
	let notice = $state('');
	let busy = $state('');
	let expanded = $state('');
	let receipts = $state<ToolReceipts | null>(null);
	/* Unsaved class choices for the expanded connection. */
	let draft = $state<Record<string, ToolClass>>({});

	type AddMode = 'url' | 'command' | 'plugin';
	let adding = $state<AddMode | null>(null);
	let addName = $state('');
	let addValue = $state('');
	let addToken = $state('');

	async function load() {
		failure = '';
		try {
			connections = await fetchConnections(companyId);
		} catch (cause) {
			failure = failureSentence(cause, 'Connections could not be read.');
		}
	}

	$effect(() => {
		void companyId;
		void load();
	});

	async function act(key: string, action: () => Promise<unknown>, done: string) {
		if (busy) return;
		busy = key;
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

	function slug(value: string): string {
		return value
			.toLowerCase()
			.replace(/^https?:\/\//, '')
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '')
			.slice(0, 40);
	}

	function openAdd(mode: AddMode, name = '', value = '') {
		adding = mode;
		addName = name;
		addValue = value;
		addToken = '';
	}

	async function submitAdd(event: SubmitEvent) {
		event.preventDefault();
		const value = addValue.trim();
		if (!value || !adding) return;
		if (adding === 'plugin') {
			await act(
				'add',
				async () => {
					const result = await importPlugin(companyId, value);
					const parts = [`${result.import.connections.length} connection(s) added`];
					if (result.import.skills.length)
						parts.push(
							result.skills_requested
								? `Exec is adding ${result.import.skills.length} skill(s) for you to review in Skills`
								: `${result.import.skills.length} skill(s) found; ask Exec to add them`
						);
					if (result.import.skipped.length)
						parts.push(`skipped: ${result.import.skipped.join('; ')}`);
					adding = null;
					notice = `${result.import.plugin}: ${parts.join('. ')}.`;
				},
				''
			);
			return;
		}
		const name = slug(addName || value.split(/\s+/)[0]);
		let input: NewConnection;
		if (adding === 'url') {
			input = {
				kind: 'remote',
				name,
				endpoint: value,
				auth: addToken.trim() ? { type: 'bearer', credential: addToken.trim() } : { type: 'none' }
			};
		} else {
			const [command, ...args] = value.split(/\s+/);
			input = { kind: 'local', name, command, args };
		}
		await act(
			'add',
			async () => {
				const { connection } = await addConnection(companyId, input);
				adding = null;
				expanded = connection.name;
			},
			`${name} added.`
		);
	}

	async function toggle(connection: ToolConnection) {
		if (expanded === connection.name) {
			expanded = '';
			return;
		}
		expanded = connection.name;
		draft = {};
		receipts = null;
		try {
			receipts = await fetchReceipts(companyId, connection.name);
		} catch {
			receipts = { reads: [], effects: [] };
		}
	}

	async function startSignIn(connection: ToolConnection) {
		await act(
			`signin:${connection.name}`,
			async () => {
				const url = await signIn(companyId, connection.name);
				window.location.assign(url);
			},
			''
		);
	}

	function classOf(connection: ToolConnection, tool: string): ToolClass {
		return draft[tool] ?? currentClasses(connection)[tool]?.class ?? 'acts';
	}

	function granted(connection: ToolConnection): boolean {
		return connection.grants.some((grant) => grant.grantee === '*');
	}

	async function grant(connection: ToolConnection) {
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
			`${connection.name} is available to every agent with these classes.`
		);
	}

	const grouped = (connection: ToolConnection) =>
		(['reads', 'acts', 'reserved'] as ToolClass[]).map((klass) => ({
			klass,
			tools: connection.tools.filter((tool) => classOf(connection, tool.name) === klass)
		}));

	const tone = (connection: ToolConnection) =>
		connection.frozen
			? 'warning'
			: connection.status === 'working'
				? 'success'
				: connection.status === 'failed'
					? 'danger'
					: 'neutral';

	const failureText: Record<string, string> = {
		sign_in_required: 'The server asks you to sign in.',
		timed_out: 'The server did not answer in time.'
	};
</script>

<CompanyTitle title="Connections" {companyId} />

{#snippet connectionItem(connection: ToolConnection)}
	<Item
		title={connection.name}
		meta={connection.account ?? connection.server_name ?? connection.endpoint ?? connection.command}
		onclick={() => void toggle(connection)}
		selected={expanded === connection.name}
		dim={connection.status === 'disconnected'}
		unread={connection.status === 'working' && !granted(connection)}
	>
		{#snippet leading()}<Plug size={15} strokeWidth={1.8} />{/snippet}
		{#snippet trailing()}
			<span
				title={connection.failure
					? (failureText[connection.failure] ?? connection.failure)
					: connection.changed.length
						? `Changed upstream since you granted them: ${connection.changed.join(', ')}`
						: undefined}><Dot tone={tone(connection)} label={statusLabel(connection)} show /></span
			>
			{#if connection.status === 'working'}<span title="Tools this server offers"
					>{connection.tools.length} tools</span
				>{/if}
			{#if connection.status === 'awaiting_sign_in' || connection.failure === 'sign_in_required'}
				<button
					class="btn small primary"
					type="button"
					disabled={!!busy}
					onclick={(event) => {
						event.stopPropagation();
						void startSignIn(connection);
					}}>Sign in</button
				>
			{:else if connection.status !== 'working' && connection.status !== 'disconnected'}
				<button
					class="btn small"
					type="button"
					disabled={!!busy}
					onclick={(event) => {
						event.stopPropagation();
						void act(
							`probe:${connection.name}`,
							() => probeConnection(companyId, connection.name),
							`${connection.name} checked.`
						);
					}}>{busy === `probe:${connection.name}` ? 'Checking…' : 'Check again'}</button
				>
			{/if}
		{/snippet}
		{#snippet actions()}
			{#if connection.status !== 'disconnected'}<ActionMenu label={`${connection.name} options`}>
					<button
						disabled={!!busy}
						onclick={() =>
							void act(
								`probe:${connection.name}`,
								() => probeConnection(companyId, connection.name),
								`${connection.name} checked.`
							)}>Check again</button
					>
					{#if granted(connection)}<button
							disabled={!!busy}
							onclick={() =>
								void act(
									`revoke:${connection.name}`,
									() => revokeGrant(companyId, connection.name),
									`Agents can no longer use ${connection.name}.`
								)}>Stop agents using it</button
						>{/if}
					<button
						disabled={!!busy}
						title="Signs out, removes the stored credential and stops every agent using it."
						onclick={() =>
							void act(
								`disconnect:${connection.name}`,
								() => disconnectConnection(companyId, connection.name),
								`${connection.name} is disconnected.`
							)}>Disconnect</button
					>
				</ActionMenu>{/if}
		{/snippet}
		{#if expanded === connection.name}
			<div class="detail">
				{#if connection.status === 'working'}
					<div class="controls">
						<Toggle
							checked={connection.frozen}
							label="Freeze acting tools"
							title={connection.frozen
								? 'Frozen: acting tools are refused at once. Reads still work.'
								: 'Freeze to refuse every acting call at once. Reads keep working.'}
							disabled={!!busy}
							onchange={() =>
								void act(
									`freeze:${connection.name}`,
									() => freezeConnection(companyId, connection.name, !connection.frozen),
									connection.frozen
										? `${connection.name} can act again.`
										: `${connection.name} is frozen.`
								)}
						/>
						<span>Freeze</span>
						<button
							class="btn small primary"
							type="button"
							disabled={!!busy}
							title="Lets every agent use these tools with the classes shown. You can change a class at any time."
							onclick={() => void grant(connection)}
							>{granted(connection)
								? Object.keys(draft).length || connection.changed.length
									? 'Save changes'
									: 'Granted'
								: 'Let agents use it'}</button
						>
					</div>
					{#each grouped(connection) as group (group.klass)}
						{#if group.tools.length}
							<h4 title={CLASS_LABEL[group.klass].title}>{CLASS_LABEL[group.klass].label}</h4>
							<ul class="tools">
								{#each group.tools as tool (tool.name)}
									<li>
										<span
											class="tool"
											class:changed={connection.changed.includes(tool.name)}
											title={tool.description ?? tool.name}>{tool.title ?? tool.name}</span
										>
										<Segmented
											label={`Class for ${tool.name}`}
											value={classOf(connection, tool.name)}
											disabled={!!busy}
											options={(['reads', 'acts', 'reserved'] as ToolClass[]).map((value) => ({
												value,
												label: CLASS_LABEL[value].label,
												title: CLASS_LABEL[value].title
											}))}
											onchange={(value) => (draft = { ...draft, [tool.name]: value })}
										/>
									</li>
								{/each}
							</ul>
						{/if}
					{/each}
				{/if}
				{#if receipts && (receipts.effects.length || receipts.reads.length)}
					<h4>Recent</h4>
					<ul class="receipts">
						{#each receipts.effects.slice(0, 8) as effect (effect.id)}
							<li title={effect.purpose}>
								<Dot
									tone={effect.success ? 'success' : 'danger'}
									label={effect.success ? 'Done' : 'Failed'}
								/>
								{effect.tool}{effect.parties?.length ? ` → ${effect.parties.join(', ')}` : ''}
								<time>{new Date(effect.created_at).toLocaleString()}</time>
							</li>
						{/each}
						{#each receipts.reads.slice(0, 5) as read (read.id)}
							<li>
								<Dot tone={read.status === 'complete' ? 'neutral' : 'danger'} label={read.status} />
								{read.tool} <time>{new Date(read.observed_at).toLocaleString()}</time>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		{/if}
	</Item>
{/snippet}

<Page
	title="Connections"
	info={`Services your agents use through MCP. Restless holds every sign-in ${hostPlace(plane)}, never in the company computer, and governs each tool by what it can do.`}
>
	{#snippet actions()}
		{#if !adding}<button class="btn small primary" type="button" onclick={() => openAdd('url')}
				>Add</button
			>{/if}
	{/snippet}

	{#if failure}<Notice tone="danger" title="That change was not made" details={failure} />{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}

	{#if adding}
		<form class="add" onsubmit={submitAdd}>
			<div class="mode">
				<Segmented
					label="How to connect"
					value={adding}
					options={[
						{ value: 'url', label: 'URL', title: 'A remote MCP server.' },
						{
							value: 'command',
							label: 'Command',
							title: `A local MCP server, run ${hostPlace(plane)} outside the company computer.`
						},
						{
							value: 'plugin',
							label: 'Plugin',
							title:
								'A Codex or Claude plugin from Git: its servers come here, its skills go to Skills.'
						}
					]}
					onchange={(mode) => openAdd(mode)}
				/>
			</div>
			{#if adding !== 'plugin'}<input
					aria-label="Name"
					placeholder="Name"
					autocomplete="off"
					bind:value={addName}
				/>{/if}
			<input
				aria-label={adding === 'url'
					? 'MCP server URL'
					: adding === 'command'
						? 'Command'
						: 'Plugin Git URL'}
				placeholder={adding === 'url'
					? 'https://example.com/mcp'
					: adding === 'command'
						? 'npx -y some-mcp-server'
						: 'https://github.com/owner/plugin'}
				autocomplete="off"
				required
				bind:value={addValue}
			/>
			{#if adding === 'url'}<input
					aria-label="Token from your Vault"
					placeholder="Vault token (optional)"
					title="The name of a Vault secret to send as a bearer token. Leave empty to sign in instead, if the server asks."
					autocomplete="off"
					bind:value={addToken}
				/>{/if}
			<button class="btn small primary" type="submit" disabled={busy === 'add'}
				>{busy === 'add' ? 'Checking…' : 'Add'}</button
			>
			<button class="btn small ghost" type="button" onclick={() => (adding = null)}>Cancel</button>
		</form>
	{/if}

	{#if !connections}
		{#if !failure}<Skeleton label="Reading connections…" variant="page" count={3} />{/if}
	{:else}
		<Section
			title="Connected"
			count={connections.filter((c) => c.status !== 'disconnected').length}
		>
			{#each connections.filter((c) => c.status !== 'disconnected') as connection (connection.name)}
				{@render connectionItem(connection)}
			{:else}
				<Empty
					compact
					title="Nothing connected yet"
					info="Add a service below, or any MCP server by URL, command or plugin."
				/>
			{/each}
		</Section>

		<Section title="Suggestions">
			<div class="suggestions">
				{#each SUGGESTIONS.filter((s) => !connections?.some((c) => c.name === s.name && c.status !== 'disconnected')) as suggestion (suggestion.name)}
					<button
						class="btn small"
						type="button"
						title={suggestion.title}
						onclick={() => openAdd('url', suggestion.name, suggestion.endpoint)}
						>{suggestion.label}</button
					>
				{/each}
			</div>
		</Section>
	{/if}
</Page>

<style>
	.add {
		display: grid;
		grid-template-columns: minmax(0, 0.6fr) minmax(0, 1.4fr) minmax(0, 0.8fr) auto auto;
		gap: var(--space-2);
		align-items: center;
		padding: 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
	}
	.mode {
		grid-column: 1 / -1;
	}
	.detail {
		display: grid;
		gap: var(--space-2);
		padding: var(--space-2) 0;
	}
	.controls {
		display: flex;
		gap: var(--space-2);
		align-items: center;
	}
	.controls .btn {
		margin-left: auto;
	}
	h4 {
		margin: var(--space-2) 0 0;
		font-size: var(--t-small);
		font-weight: 500;
		color: var(--text-secondary);
	}
	.tools,
	.receipts {
		display: grid;
		gap: 6px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.tools li {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		justify-content: space-between;
	}
	.tool.changed {
		color: var(--text-warning, inherit);
	}
	.receipts li {
		display: flex;
		gap: 8px;
		align-items: center;
		font-size: var(--t-body);
	}
	.receipts time {
		margin-left: auto;
		color: var(--text-secondary);
	}
	.suggestions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	@container page (max-width: 560px) {
		.add {
			grid-template-columns: 1fr;
		}
		.tools li {
			flex-direction: column;
			align-items: flex-start;
		}
	}
</style>
