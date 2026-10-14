<script lang="ts">
	import { formatDay, formatMoment } from '$lib/ui/time';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import TokenPicker from '$lib/components/TokenPicker.svelte';
	import { Page, Section, Notice, Toggle, Segmented, Dot, Fold } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import {
		CLASS_LABEL,
		addConnection,
		changeConnection,
		classSummary,
		currentClasses,
		disconnectConnection,
		fetchConnections,
		fetchReceipts,
		freezeConnection,
		setConnectionBrowser,
		grantTools,
		isOff,
		probeConnection,
		restoredDecisions,
		revokeGrant,
		signIn,
		rememberSignInReturn,
		takeSignInReturn,
		statusLabel,
		type GrantedTool,
		type ToolClass,
		type ToolConnection,
		type ToolDecision,
		type ToolReceipts
	} from '$lib/model/connections';
	import {
		assignSkill,
		editSkill,
		fetchSkillLibrary,
		fetchSkillPackage,
		setSkillDisposition,
		shortDigest,
		type SkillLibrary,
		type SkillPackage,
		type SkillRow
	} from '$lib/model/skills';
	import { LOCAL_INFO, buildApps, nameForLink, skillName, type App } from '$lib/model/apps';
	import {
		fetchAppCatalogue,
		installFromCatalogue,
		type RegistryItem
	} from '$lib/model/app-catalogue';
	import { cockpitQuery, companyPrincipalQuery } from '$lib/model/queries.svelte';
	import AppMark from '$lib/primitives/AppMark.svelte';
	import { declineAppRequest, fetchAppRequests, type AppRequest } from '$lib/model/app-requests';

	const companyId = $derived(page.params.companyId ?? '');
	const key = $derived(decodeURIComponent(page.params.app ?? ''));
	const requestId = $derived(page.url.searchParams.get('request'));
	const appsHref = $derived(`/${encodeURIComponent(companyId)}/apps`);
	const workHref = (id: string) =>
		`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(id)}`;

	let connections = $state<ToolConnection[] | null>(null);
	let library = $state<SkillLibrary | null>(null);
	let request = $state<AppRequest | null>(null);
	let receipts = $state<Record<string, ToolReceipts>>({});
	/* How far back each app's activity reaches; "Show more" widens it. */
	let reach = $state<Record<string, number>>({});
	let packages = $state<Record<string, SkillPackage | { error: string }>>({});
	let failure = $state('');
	let notice = $state('');
	let busy = $state('');
	let draft = $state<Record<string, ToolClass>>({});
	/* For how long a saved grant lasts, per connection: days, or '' until turned off. */
	let expiry = $state<Record<string, string>>({});
	let address = $state<Record<string, string>>({});
	/* A skill's SKILL.md while the owner edits it, and the version it was opened at. */
	let editing = $state<Record<string, { text: string; digest: string }>>({});
	let declining = $state(false);
	let declineReason = $state('');
	/* Parts that could not be read: said, never shown as missing. */
	let partial = $state('');

	/* Who know-how can be given to: the company's teams and agents (owner only). */
	const principal = $derived(companyPrincipalQuery(companyId));
	/* An admin reads every app beside the owner; adding, allowing or changing one is the owner's. */
	const owner = $derived(principal.view?.membership_role === 'owner');
	const cockpit = $derived(cockpitQuery(companyId, () => owner));
	const teams = $derived(cockpit.view?.teams ?? []);
	const agents = $derived(cockpit.view?.people ?? []);
	const holderName = (scope: string, id: string) =>
		scope === 'team'
			? (teams.find((team) => team.id === id)?.name ?? id)
			: (agents.find((person) => person.actor_id === id)?.display ?? skillName(id));

	async function loadReceipts(connection: ToolConnection) {
		receipts[connection.name] = await fetchReceipts(
			companyId,
			connection.name,
			reach[connection.name] ?? 20
		).catch(() => ({ reads: [], effects: [] }));
	}

	async function loadPackage(skill: SkillRow) {
		const known = packages[skill.name];
		if (known && 'package' in known && known.package.digest === skill.digest) return;
		packages[skill.name] = await fetchSkillPackage(companyId, skill.name).catch((cause) => ({
			error: failureSentence(cause, 'Its files could not be read from the company computer.')
		}));
	}

	async function load() {
		failure = '';
		partial = '';
		try {
			const missing: string[] = [];
			const [tools, skills, directory] = await Promise.all([
				fetchConnections(companyId),
				fetchSkillLibrary(companyId).catch(() => {
					missing.push('know-how');
					return library;
				}),
				/* The public directory names catalogue know-how and services; extra, never required. */
				fetchAppCatalogue().catch(() => null)
			]);
			connections = tools;
			library = skills;
			if (directory) registry = directory.items;
			if (requestId) {
				const requests = await fetchAppRequests(companyId).catch(() => {
					missing.push('the request');
					return null;
				});
				if (requests) request = requests.find((row) => row.handoff_id === requestId) ?? null;
			}
			partial = missing.length ? `Couldn’t load ${missing.join(' or ')}` : '';
			/* History is kept whatever the app's state: a broken app still shows what it did. */
			await Promise.all([
				...(app?.connections ?? [])
					.filter((connection) => connection.status !== 'disconnected')
					.map(loadReceipts),
				...(app?.skills ?? []).map(loadPackage)
			]);
		} catch (cause) {
			failure = failureSentence(cause, 'This app could not be read.');
		}
	}

	$effect(() => {
		void companyId;
		void key;
		void load();
	});

	let registry = $state<RegistryItem[]>([]);
	const apps = $derived(connections ? buildApps(connections, library, [], registry) : null);
	/* A catalogue key or a pasted link names an app that is not added yet. */
	const app = $derived<App | undefined>(
		apps?.mine.find((candidate) => candidate.key === key) ??
			apps?.mine.find((candidate) => candidate.catalogue?.key === key) ??
			/* A connection's own address, also when a plugin bundle holds it. */
			apps?.mine.find((candidate) =>
				candidate.connections.some((connection) => `c-${connection.name}` === key)
			) ??
			apps?.removed.find((candidate) => candidate.key === key) ??
			apps?.browse.find((candidate) => candidate.key === key)
	);
	const pendingLink = $derived(key.startsWith('link:') ? key.slice('link:'.length) : '');
	/* A pasted link reads as the service it names: mcp.exa.ai is Exa. */
	const linkName = $derived(
		pendingLink ? nameForLink(pendingLink).replace(/^./, (first) => first.toUpperCase()) : ''
	);

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

	/* Adding checks the app and lands on what it can do; the owner allows it from that list. */
	async function add(credential?: string) {
		const entry = app?.catalogue;
		await act('add', async () => {
			const { connection } = entry
				? await addConnection(companyId, {
						kind: 'remote',
						name: entry.key,
						endpoint: entry.endpoint,
						source: `catalogue:${entry.key}`,
						auth: credential ? { type: 'bearer', credential } : { type: 'none' }
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

	/* Know-how from the public catalogue: install its files as a candidate. One without scripts
	 * is accepted at once, the deliberate click being the owner's yes; one with scripts opens for
	 * the same review as any candidate (Sprint 87). */
	async function install() {
		const skill = app?.registrySkill;
		if (!skill) return;
		await act('add', async () => {
			const installed = await installFromCatalogue(companyId, skill.key);
			if (!installed.runs_scripts)
				await setSkillDisposition(companyId, installed.skill.name, 'accepted');
			await goto(`${appsHref}/${encodeURIComponent(`s-${installed.skill.name}`)}`, {
				replaceState: true
			});
		});
	}

	/* Where a sign-in began, so the owner gets back there once it completes: the Inbox's
	 * "Sign in again" names it with ?return=. */
	const returnTo = $derived(page.url.searchParams.get('return') ?? '');
	let signedInReturn = $state('');
	$effect(() => {
		const name = app?.connections.find((connection) => connection.status === 'working')?.name;
		if (name) {
			const back = takeSignInReturn(companyId, name);
			if (back) signedInReturn = back;
		}
	});
	async function startSignIn(connection: ToolConnection) {
		await act(`signin:${connection.name}`, async () => {
			if (returnTo) rememberSignInReturn(companyId, connection.name, returnTo);
			window.location.assign(await signIn(companyId, connection.name));
		});
	}

	const classOf = (connection: ToolConnection, tool: string): ToolClass =>
		draft[`${connection.name}/${tool}`] ?? currentClasses(connection)[tool]?.class ?? 'acts';
	const granted = (connection: ToolConnection) =>
		connection.grants.some((grant) => grant.grantee === '*');
	const liveGrant = (connection: ToolConnection) =>
		connection.grants.find((grant) => grant.grantee === '*');
	const until = (days: string) =>
		days ? new Date(Date.now() + Number(days) * 86_400_000).toISOString() : null;

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
				await grantTools(companyId, connection.name, tools, until(expiry[connection.name] ?? ''));
				draft = {};
				delete expiry[connection.name];
			},
			request
				? `${app?.name ?? connection.name} is in use. The work that was waiting on it resumes on its own.`
				: `${app?.name ?? connection.name} is in use.`
		);
	}

	/* Off takes back what it may do and keeps the sign-in; On restores exactly what it allowed. */
	async function setOn(connection: ToolConnection, on: boolean) {
		const name = app?.name ?? connection.name;
		if (on) {
			await act(`on:${connection.name}`, () =>
				grantTools(companyId, connection.name, restoredDecisions(connection))
			);
			if (!failure) toast(`${name} is on again: ${classSummary(restoredDecisions(connection))}.`);
			return;
		}
		const before: ToolDecision[] = (liveGrant(connection)?.tools ?? []).map((tool) => ({
			tool: tool.tool,
			class: tool.class,
			party_args: tool.party_args
		}));
		await act(`on:${connection.name}`, () => revokeGrant(companyId, connection.name));
		if (!failure)
			toast(`${name} is off. Its sign-in is kept.`, {
				undo: before.length
					? async () => {
							await grantTools(companyId, connection.name, before);
							await load();
						}
					: undefined
			});
	}

	async function change(
		connection: ToolConnection,
		update: { endpoint?: string; credential?: string },
		done: string
	) {
		await act(
			`change:${connection.name}`,
			async () => {
				const { connection: changed } = await changeConnection(companyId, connection.name, update);
				delete address[connection.name];
				if (changed.status === 'awaiting_sign_in')
					window.location.assign(await signIn(companyId, changed.name));
			},
			done
		);
	}

	const grouped = (connection: ToolConnection) =>
		(['reads', 'acts', 'reserved'] as ToolClass[]).map((klass) => ({
			klass,
			tools: connection.tools.filter((tool) => classOf(connection, tool.name) === klass)
		}));
	const dirty = (connection: ToolConnection) =>
		Object.keys(draft).some((id) => id.startsWith(`${connection.name}/`)) ||
		connection.name in expiry ||
		connection.changed.length > 0;

	/* What it was used for: effects and reads together, newest first, each with its purpose,
	 * who used it and the Work it served. */
	type Use = {
		id: string;
		tool: string;
		at: string;
		ok: boolean;
		label: string;
		purpose?: string;
		parties?: string[];
		actor: string;
		work?: string | null;
	};
	function uses(recent: ToolReceipts | undefined): Use[] {
		if (!recent) return [];
		const named = (actor: string) =>
			recent.actors?.[actor] ?? (actor === 'exec' ? 'Exec' : skillName(actor));
		return [
			...recent.effects.map((effect) => ({
				id: effect.id,
				tool: effect.tool,
				at: effect.created_at,
				ok: effect.success,
				label: effect.success ? 'Done' : 'Failed',
				purpose: effect.purpose,
				parties: effect.parties,
				actor: named(effect.actor),
				work: effect.work_id
			})),
			...recent.reads.map((read) => ({
				id: read.id,
				tool: read.tool,
				at: read.observed_at,
				ok: read.status === 'complete',
				label: read.status === 'complete' ? 'Read' : 'Read failed',
				actor: named(read.actor),
				work: read.work_id
			}))
		].sort((a, b) => b.at.localeCompare(a.at));
	}
	const more = (recent: ToolReceipts | undefined) =>
		!!recent &&
		(recent.effects.length >= (recent.limit ?? 20) || recent.reads.length >= (recent.limit ?? 20));

	/* Who loses what when the app goes: the agents and Work that used it lately, its know-how. */
	async function remove() {
		const target = app;
		if (!target) return;
		const live = target.connections.filter((connection) => connection.status !== 'disconnected');
		const recent = live.flatMap((connection) => uses(receipts[connection.name]));
		const people = [...new Set(recent.map((use) => use.actor))];
		const work = new Set(recent.flatMap((use) => (use.work ? [use.work] : []))).size;
		const skills = target.skills.filter(
			(skill) => skill.source !== 'builtin' && skill.disposition !== 'retired'
		);
		const lines = [
			live.length
				? 'Every agent loses it at once, its sign-in is deleted, and what you allowed it to do is forgotten.'
				: '',
			people.length
				? `Lately used by ${people.slice(0, 4).join(', ')}${people.length > 4 ? ` and ${people.length - 4} more` : ''}${work ? ` on ${work} Work` : ''}.`
				: live.length
					? 'Nothing used it lately.'
					: '',
			skills.length
				? `Its know-how (${skills.map((skill) => skillName(skill.name)).join(', ')}) is removed too; you can restore it under Removed.`
				: '',
			live.some((connection) => connection.status === 'working')
				? 'To stop it for now and keep the sign-in, turn it off instead.'
				: ''
		];
		const yes = await confirmAction({
			title: `Remove ${target.name}?`,
			body: lines.filter(Boolean).join(' '),
			action: 'Remove',
			danger: true
		});
		if (!yes) return;
		await act('disconnect', async () => {
			for (const connection of live) {
				if (granted(connection)) await revokeGrant(companyId, connection.name);
				await disconnectConnection(companyId, connection.name);
			}
			for (const skill of skills) await setSkillDisposition(companyId, skill.name, 'retired');
			await goto(appsHref);
		});
	}

	const givenTo = (skill: SkillRow) =>
		library?.assignments.filter(
			(row) => row.skill_name === skill.name && row.scope !== 'company' && row.enabled
		) ?? [];
	async function give(skill: SkillRow, holder: string) {
		const [scope, id] = holder.split(':', 2) as ['team' | 'actor', string];
		if (!id) return;
		await act(
			`give:${skill.name}`,
			() => assignSkill(companyId, skill.name, scope, id, true),
			`${skillName(skill.name)} was given to ${holderName(scope, id)}.`
		);
	}

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

	/* Removing know-how keeps it, so it says so with Undo rather than asking first. */
	async function retire(skill: SkillRow) {
		await act(`retired:${skill.name}`, () => setSkillDisposition(companyId, skill.name, 'retired'));
		if (!failure)
			toast(`${skillName(skill.name)} was removed. It is kept under Removed.`, {
				undo: async () => {
					await setSkillDisposition(companyId, skill.name, 'accepted');
					await load();
				}
			});
	}

	const editable = (skill: SkillRow) =>
		owner &&
		skill.source !== 'builtin' &&
		skill.source !== 'project' &&
		skill.disposition !== 'retired';
	const body = (text: string) => text.replace(/^\uFEFF?---\r?\n[\s\S]*?\r?\n---\r?\n?/, '').trim();

	async function saveSkill(skill: SkillRow) {
		const edit = editing[skill.name];
		if (!edit) return;
		await act(
			`edit:${skill.name}`,
			async () => {
				await editSkill(companyId, skill.name, edit.text, edit.digest);
				delete editing[skill.name];
			},
			`${skillName(skill.name)} was saved. Agents follow the new version from their next use.`
		);
	}

	const connectionTone = (connection: ToolConnection) =>
		connection.frozen || isOff(connection)
			? 'warning'
			: connection.status === 'working'
				? 'success'
				: connection.status === 'failed'
					? 'danger'
					: 'warning';
</script>

<CompanyTitle title={app?.name ?? 'App'} {companyId} />

{#snippet retry()}<button class="btn small" type="button" onclick={() => void load()}
		>Try again</button
	>{/snippet}

{#snippet service(connection: ToolConnection)}
	{@const grant = liveGrant(connection)}
	<Section
		title={app && app.connections.length > 1 ? connection.name : 'Status'}
		info={connection.endpoint
			? `An MCP server at ${connection.endpoint}. The sign-in stays on your plane, never in the company computer.`
			: `Runs ${connection.command ?? 'a command'}. ${LOCAL_INFO}`}
	>
		<div class="status">
			<span title={connection.account ? `Signed in as ${connection.account}` : undefined}
				><Dot
					tone={connectionTone(connection)}
					label={isOff(connection) ? 'Off' : statusLabel(connection)}
					show
				/></span
			>
			{#if connection.account}<span class="quiet">{connection.account}</span>{/if}
			{#if grant?.expires_at}<span
					class="quiet"
					title="What it may do ends then, and it turns off on its own. Its sign-in is kept."
					>Until {formatDay(grant.expires_at)}</span
				>{/if}
			{#if isOff(connection) && connection.ended_at}<span
					class="quiet"
					title="Nothing is allowed until it is turned back on. Its sign-in is kept."
					>Since {formatDay(connection.ended_at)}</span
				>{/if}
			<span class="spacer"></span>
			{#if !owner}
				<!-- An admin reads its state; signing in or checking it again is the owner's. -->
			{:else if connection.status === 'awaiting_sign_in' || connection.failure === 'sign_in_required'}
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
			{#if connection.status === 'working' && (grant || isOff(connection))}
				<Toggle
					checked={!!grant}
					label="On"
					title={grant
						? 'On: the company may use it as allowed below. Turn off to take that away at once and keep the sign-in.'
						: 'Off: nothing is allowed, and the sign-in is kept. Turn on to allow exactly what it could do before.'}
					disabled={!!busy || !owner}
					onchange={() => void setOn(connection, !grant)}
				/>
				<span class="quiet">On</span>
			{/if}
			{#if connection.status === 'working' && grant}
				<Toggle
					checked={connection.frozen}
					label="Freeze"
					title={connection.frozen
						? 'Frozen: anything that changes something is refused at once. Reading still works.'
						: 'Freeze to refuse every call that changes something, at once. Reading keeps working.'}
					disabled={!!busy || !owner}
					onchange={() =>
						void act(
							`freeze:${connection.name}`,
							() => freezeConnection(companyId, connection.name, !connection.frozen),
							connection.frozen ? 'It can act again.' : 'Frozen.'
						)}
				/>
				<span class="quiet">Freeze</span>
			{/if}
			{#if connection.kind === 'local'}
				<Toggle
					checked={!!connection.browser}
					label="Use the company browser"
					title={connection.browser
						? 'It drives the company computer’s browser, with the sign-ins you made there. Turn off to take that away.'
						: 'Let it drive the company computer’s browser and the sign-ins you made there, such as a marketplace account. Only for tools you trust with those accounts.'}
					disabled={!!busy || !owner}
					onchange={() =>
						void act(
							`browser:${connection.name}`,
							() => setConnectionBrowser(companyId, connection.name, !connection.browser),
							connection.browser
								? 'It no longer uses the company browser.'
								: 'It can use the company browser from its next run.'
						)}
				/>
				<span class="quiet">Company browser</span>
			{/if}
		</div>
		{#if connection.status !== 'working' && connection.last_output}
			<pre
				class="last-output"
				title="What it printed last. Exec and Staff see this too, and can fix what they can.">{connection.last_output.trim()}</pre>
		{/if}
		{#if owner && connection.kind === 'remote'}
			<Fold
				label={connection.auth_type === 'oauth'
					? 'Change address or sign in again'
					: 'Change address or token'}
			>
				<form
					class="edit-row"
					onsubmit={(event) => {
						event.preventDefault();
						const endpoint = (address[connection.name] ?? '').trim();
						if (endpoint) void change(connection, { endpoint }, 'Address changed. Checked again.');
					}}
				>
					<input
						type="url"
						aria-label="Address"
						title="Its MCP address. What you allowed it to do is kept; a tool that differs at the new address waits until you allow it again."
						value={address[connection.name] ?? connection.endpoint ?? ''}
						oninput={(event) => (address[connection.name] = event.currentTarget.value)}
						disabled={!!busy}
					/>
					<button
						class="btn small"
						type="submit"
						disabled={!!busy ||
							!(address[connection.name] ?? '').trim() ||
							address[connection.name] === connection.endpoint}
						>{busy === `change:${connection.name}` ? 'Saving…' : 'Save address'}</button
					>
				</form>
				<div class="edit-row">
					{#if connection.auth_type === 'oauth'}
						<button
							class="btn small"
							type="button"
							disabled={!!busy}
							title="Sign in again, as the same or another account. What you allowed it to do is kept."
							onclick={() => void startSignIn(connection)}
							>{busy === `signin:${connection.name}` ? 'Opening…' : 'Sign in again'}</button
						>
					{:else}
						<TokenPicker
							{companyId}
							app={connection.name}
							action={connection.auth_type === 'bearer' ? 'Replace token' : 'Use a token'}
							busy={busy === `change:${connection.name}`}
							onchoose={(credential) =>
								change(connection, { credential }, 'Token changed. Checked again.')}
						/>
					{/if}
				</div>
			</Fold>
		{/if}
	</Section>

	{#if connection.status === 'working'}
		<Section
			title={granted(connection) ? 'What the company may do' : 'Choose what the company may do'}
			info="Each tool is either used freely, used with a receipt (and asks you before reaching anyone new), or asks you every time. Changes apply to the next call."
		>
			{#snippet actions()}
				{#if owner && (!granted(connection) || dirty(connection))}
					<select
						aria-label="For how long"
						title="How long it may do this. When it ends, it turns off on its own and keeps the sign-in."
						value={expiry[connection.name] ?? ''}
						onchange={(event) => (expiry[connection.name] = event.currentTarget.value)}
						disabled={!!busy}
					>
						<option value="">Until turned off</option>
						<option value="1">For a day</option>
						<option value="7">For a week</option>
						<option value="30">For 30 days</option>
					</select>
					<button
						class="btn small primary"
						type="button"
						disabled={!!busy}
						onclick={() => void allow(connection)}
						>{busy === `grant:${connection.name}`
							? 'Saving…'
							: granted(connection)
								? 'Save'
								: 'Allow'}</button
					>
				{/if}
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
									disabled={!!busy || !owner}
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
	{/if}

	{@const recent = receipts[connection.name]}
	{@const used = uses(recent)}
	{#if used.length}
		<Section
			title="Activity"
			count={used.length}
			info="What it was used for, newest first: each action with its purpose, and each read, with who used it and for which Work."
		>
			<ul class="receipts">
				{#each used as use (use.id)}
					<li>
						<Dot
							tone={use.ok ? (use.purpose ? 'success' : 'neutral') : 'danger'}
							label={use.label}
						/>
						<span class="use">
							<span
								>{skillName(use.tool)}{use.parties?.length
									? ` → ${use.parties.join(', ')}`
									: ''}</span
							>
							{#if use.purpose}<span class="purpose">{use.purpose}</span>{/if}
						</span>
						<span class="quiet">{use.actor}</span>
						{#if use.work}<a class="quiet" href={workHref(use.work)}>Work</a>{/if}
						<time>{formatMoment(use.at)}</time>
					</li>
				{/each}
			</ul>
			{#if more(recent)}
				<button
					class="btn small ghost more"
					type="button"
					disabled={!!busy}
					onclick={async () => {
						reach[connection.name] = (reach[connection.name] ?? 20) * 3;
						await loadReceipts(connection);
					}}>Show more</button
				>
			{/if}
		</Section>
	{/if}
{/snippet}

{#snippet knowHow(skill: SkillRow)}
	{@const uses = library?.usage?.[skill.name] ?? 0}
	{@const read = packages[skill.name]}
	{@const edit = editing[skill.name]}
	<Section title={app && app.skills.length > 1 ? skillName(skill.name) : 'Know-how'}>
		<p class="description">{skill.description || 'No description.'}</p>
		<div class="status">
			<span title="Work that explicitly chose this know-how" class="quiet">Used on {uses} Work</span
			>
			<span
				class="quiet"
				title={`Version ${skill.digest}${skill.origin_ref ? `, from commit ${skill.origin_ref}` : ''}${skill.origin_url ? ` of ${skill.origin_url}` : ''}. ${skill.added_by ? `Added by ${skill.added_by}.` : ''}`}
				>Version {shortDigest(skill.digest)}{skill.origin_ref
					? ` · commit ${skill.origin_ref.slice(0, 7)}`
					: ''} · {formatDay(skill.updated_at)}</span
			>
			{#if skill.has_scripts}<span
					class="badge"
					title="Ships scripts. They run with an agent's ordinary computer access, never extra authority."
					>Scripts</span
				>{/if}
			<span class="spacer"></span>
			{#if !owner}
				<!-- An admin reads the know-how; accepting, giving or removing it is the owner's. -->
			{:else if skill.disposition === 'candidate'}
				<button
					class="btn small primary"
					type="button"
					disabled={!!busy}
					title={read && 'error' in read
						? 'Its files could not be read here. Accept only if you trust where it came from.'
						: 'Agents follow it from their next use. Read it below first.'}
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
				{#if teams.length || agents.length}
					<select
						aria-label={`Give ${skillName(skill.name)} to a team or agent`}
						title="Give it to one team or agent directly. They can use it even while it is off for everyone."
						value=""
						disabled={!!busy}
						onchange={(event) => {
							const holder = event.currentTarget.value;
							event.currentTarget.value = '';
							void give(skill, holder);
						}}
					>
						<option value="" disabled>Give to…</option>
						{#if teams.length}
							<optgroup label="Teams">
								{#each teams as team (team.id)}
									<option
										value={`team:${team.id}`}
										disabled={givenTo(skill).some(
											(row) => row.scope === 'team' && row.scope_id === team.id
										)}>{team.name}</option
									>
								{/each}
							</optgroup>
						{/if}
						{#if agents.length}
							<optgroup label="Agents">
								{#each agents as person (person.actor_id)}
									<option
										value={`actor:${person.actor_id}`}
										disabled={givenTo(skill).some(
											(row) => row.scope === 'actor' && row.scope_id === person.actor_id
										)}>{person.display}</option
									>
								{/each}
							</optgroup>
						{/if}
					</select>
				{/if}
				{#if skill.source !== 'builtin'}<button
						class="btn small ghost"
						type="button"
						disabled={!!busy}
						onclick={() => void retire(skill)}>Remove</button
					>{/if}
			{:else if skill.disposition === 'retired'}
				<button
					class="btn small primary"
					type="button"
					disabled={!!busy}
					onclick={() =>
						void decide(skill, 'accepted', `${skillName(skill.name)} is company know-how again.`)}
					>Restore</button
				>
			{/if}
			{#if editable(skill) && read && 'package' in read && !edit}
				<button
					class="btn small ghost"
					type="button"
					disabled={!!busy}
					title="Change its instructions. Work that already chose it keeps the version it chose."
					onclick={() =>
						(editing[skill.name] = { text: read.package.skill_md, digest: read.package.digest })}
					>Edit</button
				>
			{/if}
		</div>
		{#if skill.disposition === 'accepted' && (offForEveryone(skill) || givenTo(skill).length)}
			<div class="status holders">
				{#if offForEveryone(skill) && !givenTo(skill).length}
					<span class="quiet"
						>Off. No agent uses it until you turn it on or give it to someone.</span
					>
				{:else}
					<span class="quiet">{offForEveryone(skill) ? 'Only' : 'Also given to'}</span>
					{#each givenTo(skill) as row (`${row.scope}:${row.scope_id}`)}
						<span class="badge holder"
							>{holderName(row.scope, row.scope_id)}{#if owner}<button
									type="button"
									class="unassign"
									aria-label={`Take ${skillName(skill.name)} back from ${holderName(row.scope, row.scope_id)}`}
									title="Take it back. They then follow the setting for everyone."
									disabled={!!busy}
									onclick={() =>
										void act(`give:${skill.name}`, () =>
											assignSkill(
												companyId,
												skill.name,
												row.scope as 'team' | 'actor',
												row.scope_id,
												null
											)
										)}>×</button
								>{/if}</span
						>
					{/each}
				{/if}
			</div>
		{/if}
		{#if edit}
			<form
				class="skill-edit"
				onsubmit={(event) => {
					event.preventDefault();
					void saveSkill(skill);
				}}
			>
				<textarea
					aria-label={`${skillName(skill.name)} SKILL.md`}
					rows="16"
					spellcheck="true"
					bind:value={edit.text}
					disabled={!!busy}></textarea>
				<div class="edit-row">
					<button class="btn small primary" type="submit" disabled={!!busy}
						>{busy === `edit:${skill.name}` ? 'Saving…' : 'Save'}</button
					>
					<button class="btn small ghost" type="button" onclick={() => delete editing[skill.name]}
						>Cancel</button
					>
				</div>
			</form>
		{:else if read && 'error' in read}
			<p class="quiet inset" role="status">{read.error}</p>
		{:else if read}
			<Fold
				label={skill.disposition === 'candidate' ? 'Read it before accepting' : 'Instructions'}
				count={read.package.files.length ? `${read.package.files.length + 1} files` : null}
				open={skill.disposition === 'candidate'}
			>
				{#if read.changed}<p class="quiet inset">
						The files in the company computer changed since this version was recorded.
					</p>{/if}
				<div class="skill-body"><Markdown text={body(read.package.skill_md)} /></div>
				{#each read.package.files as file (file.path)}
					<details class="file">
						<summary
							><span>{file.path}</span><span class="quiet">{file.bytes.toLocaleString()} bytes</span
							></summary
						>
						{#if file.text != null}<pre>{file.text}</pre>{:else}<p class="quiet inset">
								Not shown: binary or larger than 32 KB.
							</p>{/if}
					</details>
				{/each}
			</Fold>
		{/if}
	</Section>
{/snippet}

{#if !apps}
	{#if failure}<Notice tone="danger" title="This app could not be read" details={failure} />
	{:else}<Skeleton label="Reading the app…" variant="page" count={3} />{/if}
{:else if !app && !pendingLink}
	{#if partial}
		<Page title="App">
			<Notice
				tone="warning"
				title={partial}
				details="This app may be among what did not load. Nothing was removed."
				actions={retry}
			/>
			<a class="btn small" href={appsHref}>All apps</a>
		</Page>
	{:else}
		<Page title="App not found">
			<Notice tone="warning" title="This app is not here" details="It may have been removed." />
			<a class="btn small" href={appsHref}>All apps</a>
		</Page>
	{/if}
{:else}
	<Page
		title={app?.name ?? linkName}
		info={app ? `${app.description} ${app.howTip}` : `An MCP server at ${pendingLink}.`}
	>
		{#snippet leading()}
			<AppMark
				name={app?.name ?? linkName}
				catalogueKey={app?.catalogue?.key}
				knowHow={app?.category === 'Know-how'}
				size={32}
			/>
		{/snippet}
		{#snippet actions()}
			{#if owner && app && (app.connections.some((connection) => connection.status !== 'disconnected') || (app.kind === 'bundle' && app.skills.some((skill) => skill.disposition !== 'retired')))}
				<button
					class="btn small ghost"
					type="button"
					disabled={!!busy}
					title="Signs out, removes the stored credential and stops every agent using it. You confirm first."
					onclick={() => void remove()}>Remove</button
				>
			{/if}
		{/snippet}

		<a class="back" href={appsHref}>Apps</a>
		{#if signedInReturn}<Notice tone="success" title="Signed in">
				{#snippet actions()}<a class="btn small primary" href={signedInReturn}
						>Back to where you started</a
					>{/snippet}
			</Notice>{/if}
		{#if failure}<Notice tone="danger" title="That change was not made" details={failure} />{/if}
		{#if partial}<Notice tone="warning" title={partial} actions={retry} />{/if}
		{#if notice}<Notice tone="success" title={notice} />{/if}
		{#if request}
			<div
				class="request"
				title={`${request.work_title ? `For “${request.work_title}”. ` : ''}Allowing it resumes that work on its own.`}
			>
				<p class="request-line">
					<span class="asked">{request.asker} asked</span><span class="reason"
						>{request.reason}</span
					>{#if owner}<button
							type="button"
							class="link-button"
							aria-expanded={declining}
							title={`Not needed. ${request.asker} stops waiting on it; you can say why or what to use instead.`}
							disabled={!!busy}
							onclick={() => (declining = !declining)}>Not needed</button
						>{/if}
				</p>
				{#if declining}
					<form
						class="edit-row"
						onsubmit={(event) => {
							event.preventDefault();
							const asked = request;
							if (!asked) return;
							void act(
								'dismiss',
								async () => {
									await declineAppRequest(companyId, asked, declineReason);
									request = null;
									declining = false;
									declineReason = '';
								},
								'Set aside. Kept under Set aside in your Inbox.'
							);
						}}
					>
						<input
							aria-label="Why not, or what to use instead"
							placeholder="Why not, or what to use instead (optional)"
							title={`Sent to ${request.asker} so it can change course instead of asking again.`}
							autocomplete="off"
							bind:value={declineReason}
						/>
						<button class="btn small" type="submit" disabled={!!busy}>Not needed</button>
					</form>
				{/if}
			</div>
		{/if}

		{#if app && app.state === 'available'}
			<Section title="Add">
				<div class="status">
					<span title={app.howTip}>{app.how}</span>
					<span class="spacer"></span>
					{#if !owner}
						<span class="quiet" title="Adding an app is the owner's.">Not added</span>
					{:else if app.registrySkill}
						<button
							class="btn small primary"
							type="button"
							disabled={!!busy}
							title={app.registrySkill.runsScripts
								? 'Installs it as a candidate. It ships scripts, so you review them before the company uses it.'
								: 'Installs it, ready for the company to use.'}
							onclick={() => void install()}>{busy === 'add' ? 'Installing…' : 'Install'}</button
						>
					{:else if app.catalogue?.auth === 'token'}
						<TokenPicker
							{companyId}
							app={app.catalogue.key}
							action="Add"
							busy={busy === 'add'}
							onchoose={(credential) => add(credential)}
						/>
					{:else}
						<button
							class="btn small primary"
							type="button"
							disabled={!!busy}
							title="Adds it and checks it answers. You then sign in if it needs one, and choose what it may do."
							onclick={() => void add()}>{busy === 'add' ? 'Adding…' : 'Add'}</button
						>
					{/if}
				</div>
			</Section>
		{:else if !app && pendingLink}
			<Section title="Add">
				<div class="status">
					<span class="quiet">{pendingLink}</span>
					<span class="spacer"></span>
					{#if owner}<button
							class="btn small primary"
							type="button"
							disabled={!!busy}
							onclick={() => void add()}>{busy === 'add' ? 'Adding…' : 'Add'}</button
						>{/if}
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
		display: grid;
		gap: 8px;
		margin: 0;
		padding: 12px 14px;
		border: 1px solid color-mix(in srgb, var(--intent-authority) 30%, transparent);
		border-radius: var(--radius-lg);
		background: var(--intent-authority-soft);
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.5;
	}
	.last-output {
		margin: 0 14px 12px;
		padding: 8px 10px;
		max-height: 12em;
		overflow: auto;
		border-radius: var(--radius-control);
		background: var(--wash-hover);
		color: var(--text-secondary);
		font: var(--t-label) / 1.5 var(--font-mono, ui-monospace, monospace);
		white-space: pre-wrap;
	}
	.request-line {
		display: flex;
		gap: 10px;
		align-items: baseline;
		margin: 0;
	}
	.request .reason {
		flex: 1;
	}
	.edit-row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		align-items: center;
		padding: 6px 16px 10px;
	}
	.edit-row input {
		flex: 1;
		min-width: min(240px, 100%);
	}
	.request .edit-row {
		padding: 0;
	}
	.use {
		display: grid;
		flex: 1;
		min-width: 0;
	}
	.purpose {
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.more {
		margin: 0 10px 10px;
	}
	.inset {
		margin: 0;
		padding: 6px 16px;
	}
	.skill-body {
		max-width: 72ch;
		padding: 4px 16px 12px;
		font-size: var(--t-body);
	}
	.skill-edit {
		display: grid;
		gap: 4px;
		padding: 6px 14px 0;
	}
	.skill-edit textarea {
		width: 100%;
		font: var(--t-label) / 1.55 var(--font-mono, ui-monospace, monospace);
		resize: vertical;
	}
	.skill-edit .edit-row {
		padding: 0 0 10px;
	}
	.file summary {
		display: flex;
		justify-content: space-between;
		gap: var(--space-2);
		padding: 6px 16px;
		font: var(--t-label) / 1.5 var(--font-mono, ui-monospace, monospace);
		cursor: pointer;
	}
	.file pre {
		max-height: 24em;
		margin: 0 16px 10px;
		padding: 8px 10px;
		overflow: auto;
		border-radius: var(--radius-control);
		background: var(--wash-hover);
		font: var(--t-label) / 1.5 var(--font-mono, ui-monospace, monospace);
		white-space: pre-wrap;
	}
	.link-button {
		flex-shrink: 0;
		padding: 0;
		border: 0;
		background: none;
		color: var(--text-tertiary);
		font: inherit;
		cursor: pointer;
	}
	.link-button:hover {
		color: var(--ink);
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
	.holders {
		padding-top: 0;
	}
	.holder {
		display: inline-flex;
		gap: 4px;
		align-items: center;
	}
	.unassign {
		padding: 0 2px;
		border: 0;
		background: none;
		color: var(--text-tertiary);
		cursor: pointer;
		font: inherit;
	}
	.unassign:hover {
		color: var(--ink);
	}
	@container page (max-width: 560px) {
		.tools li {
			flex-direction: column;
			align-items: flex-start;
		}
	}
</style>
