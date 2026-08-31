<script lang="ts">
	/* People distinguishes accountable contacts from inspectable contributors.
	 * Teams, actor class and membership come from source-owned projections; the
	 * page never infers them from ids, role strings or Work titles. */

	import { tick } from 'svelte';
	import { page } from '$app/state';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import ConversationHistoryTools from '$lib/primitives/ConversationHistoryTools.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import { cockpitContextPath } from '$lib/model/attention';
	import { attentionQuery, cockpitQuery, conversationQuery } from '$lib/model/queries.svelte';
	import {
		personTone,
		type CockpitPerson,
		type CockpitTeam,
		type CockpitView
	} from '$lib/model/cockpit';
	import { mergeAdjacentAgentMessages } from '$lib/model/view';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const cockpitProjection = $derived(cockpitQuery(companyId));
	const attentionProjection = $derived(attentionQuery(companyId));
	const cockpit = $derived(cockpitProjection.view);
	const attention = $derived(attentionProjection.view);
	let selectedId = $state('');
	let error = $state('');

	let loadedFor = $state('');
	let composer = $state('');
	let composerFiles = $state<File[]>([]);
	let sendError = $state('');
	let sendNotice = $state('');
	let sending = $state(false);
	let scrollEl = $state<HTMLDivElement | undefined>();
	let anchoredTurnId = $state<number | null>(null);
	let transcriptTailHeight = $state(0);
	let initiallyScrolledFor = $state('');
	const selectedConversation = $derived(
		selectedId && isContact(cockpit, selectedId) ? conversationQuery(companyId, selectedId) : null
	);
	$effect(() => selectedConversation?.attach());
	const messages = $derived(selectedConversation?.messages ?? []);
	const visibleMessages = $derived(mergeAdjacentAgentMessages(messages));
	const turn = $derived(selectedConversation?.activeTurn ?? null);

	$effect(() => {
		const nextCockpit = cockpit;
		if (!nextCockpit) return;
		error = cockpitProjection.failure?.message ?? attentionProjection.failure?.message ?? '';
		if (!nextCockpit.people.some((person) => person.actor_id === selectedId)) {
			const requestedPerson = page.url.searchParams.get('person');
			selectedId =
				(requestedPerson && nextCockpit.people.some((person) => person.actor_id === requestedPerson)
					? requestedPerson
					: null) ??
				nextCockpit.people.find((person) => person.kind === 'exec')?.actor_id ??
				nextCockpit.people.find((person) => person.kind === 'staff')?.actor_id ??
				'';
		}
	});

	/* Selecting a different person is a different conversation: the transcript and
	 * the half-typed message both belong to the person they were meant for. */
	$effect(() => {
		const id = selectedId;
		if (!id || id === loadedFor) return;
		loadedFor = id;
		composer = '';
		composerFiles = [];
		sendError = '';
		sendNotice = '';
		anchoredTurnId = null;
		transcriptTailHeight = 0;
	});

	async function anchorSubmittedMessage(messageId: number) {
		await tick();
		const scroller = scrollEl;
		const message = document.getElementById(messageDomId(String(messageId)));
		if (!scroller || !message || anchoredTurnId !== messageId) return;

		transcriptTailHeight = Math.max(0, scroller.clientHeight - message.offsetHeight);
		await tick();
		const top =
			message.getBoundingClientRect().top -
			scroller.getBoundingClientRect().top +
			scroller.scrollTop;
		const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		scroller.scrollTo({ top, behavior: reduceMotion ? 'auto' : 'smooth' });
	}

	$effect(() => {
		const messageId = turn?.triggerMessageId ?? null;
		if (messageId === null || anchoredTurnId === messageId) return;
		const firstMessageId = messages[0]?.id;
		if (firstMessageId) initiallyScrolledFor = `${companyId}:${selectedId}:${firstMessageId}`;
		anchoredTurnId = messageId;
		void anchorSubmittedMessage(messageId);
	});

	$effect(() => {
		const firstMessageId = messages[0]?.id;
		if (!firstMessageId || turn) return;
		const conversationKey = `${companyId}:${selectedId}:${firstMessageId}`;
		if (initiallyScrolledFor === conversationKey) return;
		initiallyScrolledFor = conversationKey;
		anchoredTurnId = null;
		transcriptTailHeight = 0;
		void tick().then(() => scrollEl?.scrollTo({ top: scrollEl.scrollHeight }));
	});

	async function submitMessage(event: SubmitEvent) {
		event.preventDefault();
		const text = composer.trim();
		if (!text || sending || !canSend || !selectedConversation) return;
		sending = true;
		sendError = '';
		sendNotice = '';
		const sent = composer;
		const files = composerFiles;
		composer = '';
		try {
			const contextPath = cockpitContextPath(companyId, page.url);
			const result = await selectedConversation.send(text, files, contextPath, false, !!turn);
			composerFiles = [];
			if (result.interrupted) {
				sendNotice = `${selected.display} was interrupted and your new direction is queued.`;
			} else if (!contextPath || result.contextOmitted) {
				sendNotice = 'Message sent without the current-screen link.';
			}
		} catch (cause) {
			composer = sent;
			sendError = cause instanceof Error ? cause.message : 'Your message was not delivered.';
		} finally {
			sending = false;
		}
	}

	const people = $derived(
		cockpit?.people.filter((person) => person.kind !== 'owner' && person.kind !== 'system') ?? []
	);
	const exec = $derived(people.find((person) => person.kind === 'exec') ?? null);
	const teams = $derived(cockpit?.teams ?? []);
	const teamGroups = $derived(
		teams.map((team) => ({
			team,
			lead: people.find((person) => person.actor_id === team.lead_actor_id) ?? null,
			members: people.filter(
				(person) =>
					person.team_id === team.id &&
					person.actor_id !== team.lead_actor_id &&
					person.kind === 'staff'
			)
		}))
	);
	const activeTeamIds = $derived(new Set(teams.map((team) => team.id)));
	const unassigned = $derived(
		people.filter(
			(person) =>
				person.kind === 'staff' && (person.team_id === null || !activeTeamIds.has(person.team_id))
		)
	);
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
			selected !== null &&
			isContact(cockpit, selected.actor_id)
	);
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

	function exceptionalState(person: CockpitPerson): string | null {
		if (person.model_cooldown) return 'cooling down';
		if (person.session_running) return 'working';
		return null;
	}

	function isContact(view: CockpitView | null, actorId: string): boolean {
		const person = view?.people.find((candidate) => candidate.actor_id === actorId);
		return (
			person?.kind === 'exec' ||
			(view?.teams.some((team) => team.lead_actor_id === actorId) ?? false)
		);
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

	function messageDomId(messageId: string): string {
		return `people-message-${companyId}-${selectedId}-${messageId.replaceAll(':', '-')}`;
	}

	function jumpToMessage(messageId: string) {
		const message = document.getElementById(messageDomId(messageId));
		if (!message) return;
		const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		message.scrollIntoView({ behavior: reduceMotion ? 'auto' : 'smooth', block: 'center' });
		if (!reduceMotion) {
			message.animate(
				[
					{
						boxShadow:
							'inset 2px 0 0 color-mix(in srgb, var(--intent-conversation) 0%, transparent)'
					},
					{ boxShadow: 'inset 2px 0 0 var(--intent-conversation)' },
					{
						boxShadow:
							'inset 2px 0 0 color-mix(in srgb, var(--intent-conversation) 0%, transparent)'
					}
				],
				{ duration: 900, easing: 'cubic-bezier(0.23, 1, 0.32, 1)' }
			);
		}
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
			{#if exec}
				<section class="exec-anchor" aria-label="Company executive">
					{@render personRow(exec, 'exec')}
				</section>
			{/if}

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
						{@render personRow(person, 'member')}
					{/each}
				</section>
			{/if}

			{#if people.length === 0}
				<p class="empty-state">No company roles are recorded yet.</p>
			{/if}
		</div>
		{#if selected}
			<section class="people-focus-summary">
				<strong>{selected.display}'s current focus</strong>
				<p>{focusWork?.title ?? 'No active Work is assigned.'}</p>
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
					<small title={`Actor ${selected.actor_id}`}
						>{roleLabel(selected.role)} · {selected.actor_id}</small
					>
				</div>
				{#if canSend && visibleMessages.length}
					<ConversationHistoryTools
						messages={visibleMessages}
						participantName={selected.actor_id === 'exec' ? 'Exec' : selected.display}
						onjump={jumpToMessage}
					/>
				{/if}
				{#if exceptionalState(selected)}
					<span class="profile-presence {selected.session_running ? 'working' : ''}">
						<MatrixGlyph
							rows={selected.session_running ? GLYPHS.dots : GLYPHS.ring}
							size={9}
							glow={selected.session_running}
						/>
						{exceptionalState(selected)}
					</span>
				{/if}
			</header>

			{#if canSend}
				<div class="talk-msgs exr-msgs" bind:this={scrollEl}>
					{#each visibleMessages as message, i (message.id)}
						{#if i === 0 || dayOf(message.createdAt) !== dayOf(visibleMessages[i - 1].createdAt)}
							<div class="day-sep" aria-hidden="true">
								<span>{dayLabel(message.createdAt)}</span>
							</div>
						{/if}
						<ConversationMessage
							domId={messageDomId(message.id)}
							sender={message.from === 'you' ? 'owner' : 'agent'}
							author={message.from === 'you' ? 'You' : message.author || selected.display}
							text={message.text}
							createdAt={message.createdAt}
							details={message.details}
							intent={message.intent}
							attachments={message.attachments}
							hrefFor={attachmentHref}
						/>
					{:else}
						<div class="exr-empty">
							<p class="exr-empty-h">Nothing said yet.</p>
							<p class="exr-empty-p">Start a conversation with this accountable company contact.</p>
						</div>
					{/each}
					{#if turn}
						<ConversationTurnDock
							participantName={selected.actor_id === 'exec' ? 'Exec' : selected.display}
							{turn}
						/>
					{/if}
					{#if anchoredTurnId !== null}
						<div
							class="conversation-tail"
							style:height={`${transcriptTailHeight}px`}
							aria-hidden="true"
						></div>
					{/if}
				</div>
				<form class="talk-composer" onsubmit={submitMessage}>
					<Composer
						bind:value={composer}
						bind:files={composerFiles}
						actionLabel={turn ? 'Interrupt & send' : 'Send'}
						disabled={sending}
						minlength={1}
						placeholder={turn
							? `Interrupt ${selected.display} with new direction…`
							: `Ask ${selected.display}, redirect, or make a judgement…`}
						ariaLabel={turn
							? `Interrupt and message ${selected.display}`
							: `Message ${selected.display}`}
					/>
					{#if sendError}<p class="exr-error" role="alert">{sendError}</p>{/if}
					{#if sendNotice}<p class="exr-notice" role="status">{sendNotice}</p>{/if}
				</form>
			{:else}
				<div class="member-inspection">
					<section class="inspection-route">
						<div>
							<strong class="inspection-route-title">
								<SemanticMark meaning="people" size="small" />
								{selected.display} contributes through accountable Work.
							</strong>
							{#if selectedTeamLead && selectedTeamLead.actor_id !== selected.actor_id}
								<p>{selectedTeamLead.display} is accountable for {selectedTeam?.name}.</p>
								<button
									type="button"
									class="contact-route"
									onclick={() => (selectedId = selectedTeamLead!.actor_id)}
								>
									Talk to {selectedTeamLead.display} ▸
								</button>
							{:else if exec}
								<p>This specialist is currently unassigned; the Exec is accountable.</p>
								<button
									type="button"
									class="contact-route"
									onclick={() => (selectedId = exec!.actor_id)}
								>
									Talk to {exec.display} ▸
								</button>
							{/if}
						</div>
					</section>

					<section class="inspection-work">
						<header>
							<h2>Current Work</h2>
							<span>{selectedWork.length}</span>
						</header>
						{#each selectedWork as item (item.id)}
							<a href={`/${companyId}/work/${item.id}`} class="inspection-work-row">
								<div><strong>{item.title}</strong><small>Revision {item.revision}</small></div>
								<span class:blocked={item.status === 'blocked'}>{item.status}</span>
							</a>
						{:else}
							<p class="inspection-empty">No Work is currently assigned.</p>
						{/each}
					</section>
				</div>
			{/if}
		{:else}
			<p class="empty-state">Select a person to open their record.</p>
		{/if}
	</section>
</div>

{#snippet personRow(person: CockpitPerson, level: 'exec' | 'lead' | 'member')}
	<button
		type="button"
		class="person-row {level}"
		class:selected={selected?.actor_id === person.actor_id}
		title={`${person.display} · ${roleLabel(person.role)} · ${person.actor_id}`}
		aria-label={level === 'member' ? `Inspect ${person.display}` : `Contact ${person.display}`}
		onclick={() => (selectedId = person.actor_id)}
	>
		<span class="person-avatar tone-{personTone({ actor_id: person.actor_id })}"
			>{initials(person.display)}</span
		>
		<span class="person-copy">
			<strong>{person.display}</strong>
		</span>
		{#if exceptionalState(person)}
			<span class="person-state {person.session_running ? 'working' : ''}">
				<i></i>{exceptionalState(person)}
			</span>
		{/if}
	</button>
{/snippet}

<style>
	.team-group {
		border-bottom: 1px solid var(--border);
	}

	.exec-anchor {
		padding: 8px;
		border-bottom: 1px solid var(--border-strong);
		background:
			linear-gradient(
				135deg,
				color-mix(in srgb, var(--intent-conversation) 10%, transparent),
				transparent 58%
			),
			var(--surface-alt);
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

	.person-row.exec {
		border: 1px solid color-mix(in srgb, var(--intent-conversation) 28%, var(--border));
		background: rgba(255, 255, 255, 0.58);
		box-shadow: var(--bevel-subtle);
	}

	.person-row.member {
		grid-template-columns: 22px minmax(0, 1fr) auto;
		gap: 8px;
		padding: 6px 12px 6px 31px;
		color: var(--text-secondary);
	}

	.person-row.exec:hover,
	.person-row.lead:hover,
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

	.person-row.member :global(.person-avatar) {
		width: 22px;
		height: 22px;
		border-color: var(--border);
		font-size: var(--t-label);
		opacity: 0.82;
	}

	.person-row.member .person-copy strong {
		font-size: var(--t-label);
		font-weight: 560;
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

	.conversation-tail {
		width: 1px;
		flex: 0 0 auto;
		pointer-events: none;
	}

	.member-inspection {
		min-height: 0;
		overflow: auto;
		padding: 18px;
	}

	.inspection-route {
		padding: 15px 16px;
		border: 1px solid color-mix(in srgb, var(--intent-conversation) 22%, var(--border));
		background: color-mix(in srgb, var(--intent-conversation) 5%, var(--surface));
		box-shadow: var(--bevel-subtle);
	}

	.inspection-route-title {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.inspection-route strong {
		display: block;
		font-size: var(--t-body);
	}

	.inspection-route p,
	.inspection-empty {
		margin: 5px 0 0;
		font-size: var(--t-body);
		line-height: 1.5;
		color: var(--text-secondary);
	}

	.inspection-work {
		margin-top: 18px;
		border: 1px solid var(--border);
		background: rgba(255, 255, 255, 0.35);
	}

	.inspection-work > header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 12px;
		border-bottom: 1px solid var(--border);
	}

	.inspection-work h2 {
		margin: 0;
		font-size: var(--t-body);
	}

	.inspection-work > header span,
	.inspection-work-row > span {
		font: 500 var(--t-label) var(--font-mono);
		text-transform: uppercase;
		color: var(--text-tertiary);
	}

	.inspection-work-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 12px;
		border-bottom: 1px solid var(--border);
		color: var(--ink);
		text-decoration: none;
	}

	.inspection-work-row:last-child {
		border-bottom: 0;
	}

	.inspection-work-row:hover {
		background: rgba(255, 255, 255, 0.58);
	}

	.inspection-work-row strong,
	.inspection-work-row small {
		display: block;
	}

	.inspection-work-row small {
		margin-top: 4px;
		color: var(--text-tertiary);
	}

	.inspection-work-row > span.blocked {
		color: var(--danger);
	}

	.inspection-empty {
		padding: 14px 12px;
	}
</style>
