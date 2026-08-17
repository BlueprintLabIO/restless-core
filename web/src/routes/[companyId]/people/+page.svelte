<script lang="ts">
	/* People is a conversation surface (S06-T2/T6). The centre is the transcript
	 * with the selected person; their operating evidence sits beside it rather
	 * than in front of it. There is no Executive rail on this route because a
	 * second permanent conversation would compete with the one already open.
	 *
	 * Teams and membership come from the cockpit projection. This page does not
	 * infer hierarchy from role strings or Work titles. The Exec and active team
	 * leads are addressable; other Staff receive owner input through their Work. */

	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import AttachmentList from '$lib/primitives/AttachmentList.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import IntentReceipt from '$lib/primitives/IntentReceipt.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import {
		getActorConversation,
		getAttention,
		sendActorMessage,
		type AttentionView
	} from '$lib/model/attention';
	import {
		getCockpit,
		personTone,
		type CockpitPerson,
		type CockpitTeam,
		type CockpitView
	} from '$lib/model/cockpit';
	import type { ThreadMessage } from '$lib/model/view';

	const companyId = $derived(page.params.companyId ?? 'aris');
	let cockpit = $state<CockpitView | null>(null);
	let attention = $state<AttentionView | null>(null);
	let selectedId = $state('');
	let error = $state('');

	let messages = $state<ThreadMessage[]>([]);
	let loadedFor = $state('');
	let composer = $state('');
	let composerFiles = $state<File[]>([]);
	let sendError = $state('');
	let sending = $state(false);
	let scrollEl = $state<HTMLDivElement | undefined>();

	onMount(() => {
		void refresh();
		const timer = window.setInterval(() => void refresh(false), 8_000);
		return () => window.clearInterval(timer);
	});

	async function refresh(showError = true) {
		try {
			const [nextCockpit, nextAttention] = await Promise.all([
				getCockpit(companyId),
				getAttention(companyId)
			]);
			cockpit = nextCockpit;
			attention = nextAttention;
			error = '';
			if (!nextCockpit.people.some((person) => person.actor_id === selectedId)) {
				const firstLead = nextCockpit.teams
					.map((team) => team.lead_actor_id)
					.find((actorId) =>
						nextCockpit.people.some(
							(person) =>
								person.actor_id === actorId &&
								!['owner', 'exec', 'world', 'daemon'].includes(person.actor_id)
						)
					);
				selectedId =
					firstLead ??
					nextCockpit.people.find((person) => person.actor_id === 'exec')?.actor_id ??
					nextCockpit.people.find((person) => person.actor_id !== 'owner')?.actor_id ??
					'';
			}
			if (selectedId) await loadConversation(selectedId);
		} catch (cause) {
			if (showError) error = cause instanceof Error ? cause.message : 'People are unavailable.';
		}
	}

	/* Selecting a different person is a different conversation: the transcript and
	 * the half-typed message both belong to the person they were meant for. */
	$effect(() => {
		const id = selectedId;
		if (!id || id === loadedFor) return;
		loadedFor = id;
		messages = [];
		composer = '';
		composerFiles = [];
		sendError = '';
		void loadConversation(id);
	});

	$effect(() => {
		if (messages.length === 0) return;
		scrollEl?.scrollTo({ top: scrollEl.scrollHeight });
	});

	async function loadConversation(actorId: string) {
		try {
			const conversation = await getActorConversation(companyId, actorId);
			/* A slower request for a person the owner has already moved on from must
			 * not overwrite the transcript now on screen. */
			if (actorId !== selectedId) return;
			messages = conversation.messages.map((message) => ({
				id: String(message.id),
				from: message.from_actor === 'owner' ? 'you' : 'agent',
				author: message.from_actor === 'owner' ? 'You' : conversation.actor.display,
				text: message.body,
				createdAt: message.created_at,
				replyToMessageId: null,
				assetId: null,
				runId: null,
				attachments: message.attachments ?? [],
				intent: message.intent ?? null,
				contextPath: message.context_path ?? null
			}));
		} catch {
			/* Preserve the last observed transcript when the live source drops. */
		}
	}

	async function submitMessage(event: SubmitEvent) {
		event.preventDefault();
		const text = composer.trim();
		if (!text || sending || !canSend) return;
		sending = true;
		sendError = '';
		const sent = composer;
		const files = composerFiles;
		const target = selectedId;
		composer = '';
		try {
			await sendActorMessage(companyId, target, text, undefined, files, page.url.pathname);
			composerFiles = [];
			await loadConversation(target);
		} catch (cause) {
			composer = sent;
			sendError = cause instanceof Error ? cause.message : 'Your message was not delivered.';
		} finally {
			sending = false;
		}
	}

	const people = $derived(cockpit?.people.filter((person) => person.actor_id !== 'owner') ?? []);
	const teams = $derived(cockpit?.teams ?? []);
	const teamGroups = $derived(
		teams.map((team) => ({
			team,
			lead:
				people.find(
					(person) => person.actor_id === team.lead_actor_id && !isStandingActor(person)
				) ?? null,
			members: people.filter(
				(person) =>
					person.team_id === team.id &&
					person.actor_id !== team.lead_actor_id &&
					!isStandingActor(person)
			)
		}))
	);
	const activeTeamIds = $derived(new Set(teams.map((team) => team.id)));
	const unassigned = $derived(
		people.filter(
			(person) =>
				!isStandingActor(person) && (person.team_id === null || !activeTeamIds.has(person.team_id))
		)
	);
	const standingActors = $derived(people.filter(isStandingActor));
	const selected = $derived(
		people.find((person) => person.actor_id === selectedId) ?? people[0] ?? null
	);
	const selectedTeam = $derived(
		selected
			? (teams.find((team) => team.lead_actor_id === selected.actor_id) ??
					(selected.team_id ? (teams.find((team) => team.id === selected.team_id) ?? null) : null))
			: null
	);
	const selectedTeamLead = $derived(
		selectedTeam
			? (people.find((person) => person.actor_id === selectedTeam.lead_actor_id) ?? null)
			: null
	);
	const graph = $derived(attention?.workGraph ?? null);
	const selectedWork = $derived(
		selected ? (graph?.work ?? []).filter((work) => work.owner_id === selected.actor_id) : []
	);
	const activeWork = $derived(selectedWork.filter((work) => work.status === 'active'));
	const waitingWork = $derived(selectedWork.filter((work) => work.status === 'blocked'));
	const focusWork = $derived(activeWork[0] ?? waitingWork[0] ?? selectedWork[0] ?? null);

	const canSend = $derived(
		cockpit?.source_health.orgintel === 'available' &&
			(selected?.actor_id === 'exec' ||
				(selected !== null &&
					!isStandingActor(selected) &&
					teams.some((team) => team.lead_actor_id === selected.actor_id)))
	);
	const waiting = $derived(canSend && messages.length > 0 && messages.at(-1)!.from === 'you');

	const attachmentHref = (attachment: { uploadId: string }) =>
		`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`;

	function initials(name: string): string {
		return name
			.split(/\s+/)
			.filter(Boolean)
			.slice(0, 2)
			.map((part) => part[0]?.toUpperCase() ?? '')
			.join('');
	}

	function stateOf(person: CockpitPerson): string {
		if (person.model_cooldown) return 'cooling down';
		if (person.session_running) return 'working';
		return 'ready';
	}

	function isStandingActor(person: CockpitPerson): boolean {
		return ['exec', 'world', 'daemon'].includes(person.actor_id);
	}

	function teamState(team: CockpitTeam): string {
		const members = `${team.member_count} member${team.member_count === 1 ? '' : 's'}`;
		const moving = `${team.in_motion_count} in motion`;
		const blocked = team.blocked_count ? ` · ${team.blocked_count} blocked` : '';
		return `${members} · ${moving}${blocked}`;
	}

	function roleLabel(value: string): string {
		return value.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());
	}

	function dayOf(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		return Number.isNaN(date.getTime()) ? '' : date.toDateString();
	}

	function dayLabel(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		const today = new Date();
		if (date.toDateString() === today.toDateString()) return 'Today';
		const yesterday = new Date();
		yesterday.setDate(today.getDate() - 1);
		if (date.toDateString() === yesterday.toDateString()) return 'Yesterday';
		return date.toLocaleDateString(undefined, { month: 'long', day: 'numeric' });
	}

	function timeLabel(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		return date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
	}

	function selectTeam(team: CockpitTeam) {
		const lead = people.find((person) => person.actor_id === team.lead_actor_id);
		if (lead) selectedId = lead.actor_id;
	}
</script>

<svelte:head><title>People — {cockpit?.company.name ?? companyId}</title></svelte:head>

<div class="cockpit-screen people-screen">
	{#if error}<div class="cockpit-error">{error}</div>{/if}

	<section class="people-index cockpit-pane">
		<header class="cockpit-pane-head">
			<h1>People</h1>
			<span class="pane-count">{people.length}</span>
		</header>
		<div class="people-list">
			{#each teamGroups as group (group.team.id)}
				<section class="team-group" aria-label={group.team.name}>
					<button
						type="button"
						class="team-heading"
						disabled={!group.lead}
						onclick={() => selectTeam(group.team)}
					>
						<span><i>▾</i>{group.team.name}</span>
						<small>{teamState(group.team)}</small>
					</button>
					{#if group.lead}
						{@render personRow(group.lead, 'lead')}
					{:else}
						<p class="team-missing">Lead {group.team.lead_actor_id} is unavailable.</p>
					{/if}
					{#each group.members as member (member.actor_id)}
						{@render personRow(member, 'member')}
					{/each}
				</section>
			{/each}

			{#if unassigned.length}
				<section class="team-group roster-remainder" aria-label="Unassigned people">
					<div class="team-heading static">
						<span>Unassigned</span><small>{unassigned.length}</small>
					</div>
					{#each unassigned as person (person.actor_id)}
						{@render personRow(person, 'root')}
					{/each}
				</section>
			{/if}

			{#if standingActors.length}
				<section class="team-group roster-remainder" aria-label="Company and system actors">
					<div class="team-heading static"><span>Not people</span></div>
					{#each standingActors as person (person.actor_id)}
						{@render personRow(person, 'root')}
					{/each}
				</section>
			{/if}

			{#if people.length === 0}
				<p class="empty-state">No company roles are recorded yet.</p>
			{/if}
		</div>
		{#if selected}
			<section class="people-focus-summary">
				<strong>{selected.display} is working on</strong>
				<p>{focusWork?.outcome ?? focusWork?.title ?? 'No active outcome is assigned.'}</p>
				<a href={`/${companyId}/work`}>Open linked Work →</a>
			</section>
		{/if}
	</section>

	<section class="people-talk cockpit-pane">
		{#if selected}
			<header class="talk-head">
				<span class="person-avatar tone-{personTone({ actor_id: selected.actor_id })}"
					>{initials(selected.display)}</span
				>
				<div class="talk-who">
					<strong>{selected.display}</strong>
					<small>{roleLabel(selected.role)}</small>
				</div>
				<span class="profile-presence {selected.session_running ? 'working' : ''}">
					<MatrixGlyph
						rows={selected.session_running ? GLYPHS.dots : GLYPHS.ring}
						size={9}
						glow={selected.session_running}
					/>
					{stateOf(selected)}
				</span>
			</header>

			<div class="talk-msgs exr-msgs" bind:this={scrollEl}>
				{#each messages as message, i (message.id)}
					{#if i === 0 || dayOf(message.createdAt) !== dayOf(messages[i - 1].createdAt)}
						<div class="day-sep" aria-hidden="true"><span>{dayLabel(message.createdAt)}</span></div>
					{/if}
					<article class="people-message {message.from === 'you' ? 'you' : 'agent'}">
						<header class="people-message-author">
							<span>
								{#if message.from === 'agent'}
									<MatrixGlyph rows={GLYPHS.p} size={7} />
								{/if}
								<strong
									>{message.from === 'you' ? 'You' : message.author || selected.display}</strong
								>
								{#if message.from === 'agent'}<small>{roleLabel(selected.role)}</small>{/if}
							</span>
							<time>{timeLabel(message.createdAt)}</time>
						</header>
						<div class="people-message-body">
							{#if message.from === 'agent'}
								<Markdown text={message.text} />
								<AttachmentList attachments={message.attachments} hrefFor={attachmentHref} />
								{#if message.intent}<IntentReceipt intent={message.intent} />{/if}
							{:else}
								<p>{message.text}</p>
								<AttachmentList attachments={message.attachments} hrefFor={attachmentHref} />
								{#if message.contextPath}
									<span class="message-context-ref">
										<MatrixGlyph rows={GLYPHS.work} size={7} />
										Sent from {message.contextPath}
									</span>
								{/if}
							{/if}
						</div>
					</article>
				{:else}
					<div class="exr-empty">
						<p class="exr-empty-h">Nothing said yet.</p>
						<p class="exr-empty-p">
							This is the company record of what has passed between you and {selected.display}.
						</p>
					</div>
				{/each}
				{#if waiting}
					<div class="exr-thinking" aria-label="Preparing an answer"><i></i><i></i><i></i></div>
				{/if}
			</div>

			{#if canSend}
				<form class="talk-composer" onsubmit={submitMessage}>
					<Composer
						bind:value={composer}
						bind:files={composerFiles}
						disabled={sending}
						placeholder={`Ask ${selected.display}, redirect, or make a judgement…`}
						ariaLabel={`Message ${selected.display}`}
					>
						{#snippet controls()}
							<div class="people-composer-context">
								<MatrixGlyph rows={GLYPHS.work} size={8} />
								<strong>Message {selected.display}</strong>
								<span>Linked · People</span>
							</div>
						{/snippet}
					</Composer>
					<div class="composer-foot">
						<span>
							{selected.actor_id === 'exec'
								? 'Exec interprets intent and confirms consequential changes'
								: `${selected.display} speaks for ${selectedTeam?.name ?? 'their team'} and can change its Work`}
						</span>
						<span>⌘ ↵ send</span>
					</div>
					{#if sendError}<p class="exr-error" role="alert">{sendError}</p>{/if}
				</form>
			{:else}
				<div class="talk-closed">
					<SemanticMark meaning="unavailable" size="small" />
					<div>
						<strong>{selected.display} is not a team contact.</strong>
						<p>Direct conversation is available with the Exec and accountable team leads.</p>
						{#if selectedTeamLead && selectedTeamLead.actor_id !== selected.actor_id}
							<button
								type="button"
								class="contact-route"
								onclick={() => (selectedId = selectedTeamLead!.actor_id)}
							>
								Talk to {selectedTeamLead.display} for {selectedTeam?.name} ▸
							</button>
						{:else}
							<a href={`/${companyId}`}>Talk to the Exec ▸</a>
						{/if}
					</div>
				</div>
			{/if}
		{:else}
			<p class="empty-state">Select a person to open their record.</p>
		{/if}
	</section>
</div>

{#snippet personRow(person: CockpitPerson, level: 'lead' | 'member' | 'root')}
	<button
		type="button"
		class="person-row {level}"
		class:selected={selected?.actor_id === person.actor_id}
		onclick={() => (selectedId = person.actor_id)}
	>
		<span class="person-avatar tone-{personTone({ actor_id: person.actor_id })}"
			>{initials(person.display)}</span
		>
		<span class="person-copy">
			<strong>{person.display}</strong>
			<small>{level === 'lead' ? 'Team lead' : roleLabel(person.role)}</small>
		</span>
		<span class="person-state {person.session_running ? 'working' : ''}">
			<i></i>{stateOf(person)}
		</span>
	</button>
{/snippet}

<style>
	.team-group {
		border-bottom: 1px solid var(--border);
	}

	.team-heading {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 10px 12px 8px;
		border: 0;
		background: var(--surface-alt);
		color: var(--ink);
		text-align: left;
		cursor: pointer;
	}

	.team-heading:disabled,
	.team-heading.static {
		cursor: default;
	}

	.team-heading > span {
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font: 600 var(--t-label) var(--font-mono);
		letter-spacing: 0.08em;
		text-transform: uppercase;
	}

	.team-heading > span i {
		margin-right: 6px;
		font-style: normal;
		color: var(--intent-conversation);
	}

	.team-heading > small {
		flex: none;
		font: 500 var(--t-label) var(--font-mono);
		letter-spacing: 0.02em;
		color: var(--text-tertiary);
	}

	.person-row {
		width: 100%;
		display: grid;
		grid-template-columns: 28px minmax(0, 1fr) auto;
		align-items: center;
		gap: 9px;
		padding: 9px 12px;
		border: 0;
		border-top: 1px solid var(--border);
		background: transparent;
		color: var(--ink);
		text-align: left;
		cursor: pointer;
	}

	.person-row.member {
		padding-left: 29px;
	}

	.person-row:hover,
	.person-row.selected {
		background: rgba(255, 255, 255, 0.56);
	}

	.person-row.selected {
		box-shadow: inset 2px 0 0 var(--intent-conversation);
	}

	.person-row :global(.person-avatar) {
		width: 28px;
		height: 28px;
		font-size: var(--t-label);
	}

	.roster-remainder {
		margin-top: 7px;
	}

	.team-missing {
		margin: 0;
		padding: 9px 12px;
		border-top: 1px solid var(--border);
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}

	.contact-route {
		display: inline-block;
		margin-top: 7px;
		padding: 0;
		border: 0;
		background: transparent;
		font: 500 var(--t-label) var(--font-mono);
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--intent-conversation);
		cursor: pointer;
	}
</style>
