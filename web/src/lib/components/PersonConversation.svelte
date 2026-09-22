<script lang="ts">
	import type { Snippet } from 'svelte';
	let { actions }: { actions?: Snippet<[string]> } = $props();
	/* People distinguishes accountable contacts from inspectable contributors.
	 * Teams, actor class and membership come from source-owned projections; the
	 * page never infers them from ids, role strings or Work titles. */

	import { tick, untrack } from 'svelte';
	import { readRoomDraft, writeRoomDraft, roomDraftKey } from '$lib/model/rooms';
	import AttentionCard from '$lib/components/AttentionCard.svelte';
	import type { AttentionItem } from '$lib/model/view';
	import IntelligencePopover from '$lib/components/IntelligencePopover.svelte';
	import type { OutcomeStandard } from '$lib/model/company';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import ConversationHistoryTools from '$lib/primitives/ConversationHistoryTools.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import { cockpitContextPath } from '$lib/model/attention';
	import type { CollaborationPerson, CollaborationTeam } from '$lib/model/collaboration';
	import {
		attentionQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		companyPrincipalQuery,
		conversationQuery
	} from '$lib/model/queries.svelte';
	import { personTone, type CockpitPerson, type CockpitTeam } from '$lib/model/cockpit';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const principalProjection = $derived(companyPrincipalQuery(companyId));
	const ownerAccess = $derived(principalProjection.view?.membership_role === 'owner');
	const cockpitProjection = $derived(cockpitQuery(companyId, () => ownerAccess));
	const attentionProjection = $derived(attentionQuery(companyId, () => ownerAccess));
	const collaborationProjection = $derived(
		collaborationBootstrapQuery(companyId, () => principalProjection.view)
	);
	const cockpit = $derived(cockpitProjection.view);
	const attention = $derived(attentionProjection.view);
	function attentionFor(actorId: string): AttentionItem[] {
		if (!ownerAccess) return [];
		if (actorId === 'exec') return attention?.items ?? [];
		const team = teams.find((team) => team.lead_actor_id === actorId);
		const members = new Set([
			actorId,
			...people
				.filter((person) => team && person.team_id === team.id)
				.map((person) => person.actor_id)
		]);
		return (attention?.items ?? []).filter((item) => {
			const sourceActor =
				item.responsibleActor?.id ?? item.runtimeAttach?.requestingActor ?? item.briefAuthor?.id;
			const workOwner = attention?.workGraph?.work.find(
				(work) => work.id === item.workId
			)?.owner_id;
			return (sourceActor && members.has(sourceActor)) || (workOwner && members.has(workOwner));
		});
	}
	function needsYou(actorId: string) {
		return attentionFor(actorId).find((item) => !item.preparing);
	}

	const collaboration = $derived(collaborationProjection.view);
	type Person = CockpitPerson | CollaborationPerson;
	type Team = CockpitTeam | CollaborationTeam;
	let selectedId = $state('');
	const chatAttention = $derived(
		attentionFor(selectedId).filter((item) => item.source.kind !== 'conversation_owner_need')
	);

	async function selectPerson(id: string) {
		const url = new URL(page.url);
		url.searchParams.set('person', id);
		url.searchParams.delete('room');
		url.searchParams.delete('thread');
		url.searchParams.delete('mention');
		const need = needsYou(id);
		if (need?.source.kind === 'conversation_owner_need')
			url.searchParams.set('message', need.source.reference);
		else url.searchParams.delete('message');
		await goto(url, { replaceState: true, noScroll: true, keepFocus: true });
		await tick();
		if (need && need.source.kind !== 'conversation_owner_need') {
			scrollEl
				?.querySelector(`[data-attention-id="${CSS.escape(need.id)}"]`)
				?.scrollIntoView({ block: 'center' });
		}
	}
	let standardSaving = $state(false);
	let standardError = $state('');
	async function changeStandard(control: HTMLSelectElement) {
		const standard = control.value as OutcomeStandard;
		if (!ownerAccess || !selectedTeam || !isOwnerTeam(selectedTeam) || standardSaving) return;
		const team = selectedTeam;
		standardSaving = true;
		standardError = '';
		try {
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/teams/${team.id}/outcome-standard`,
				{
					method: 'POST',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ standard, expected_standard: team.outcome_standard })
				}
			);
			if (!response.ok) {
				const failure = await response.json();
				throw new Error(failure.message ?? 'Could not save the quality target.');
			}
		} catch (cause) {
			control.value = team.outcome_standard;
			standardError = cause instanceof Error ? cause.message : 'Could not save the quality target.';
		} finally {
			try {
				await cockpitProjection.refresh();
			} finally {
				standardSaving = false;
			}
		}
	}
	let composer = $state('');
	let composerFiles = $state<File[]>([]);
	let sendError = $state('');
	let sendNotice = $state('');
	let sendingFor = $state('');
	let scrollEl = $state<HTMLDivElement | undefined>();
	let anchoredTurnId = $state<number | null>(null);
	let transcriptTailHeight = $state(0);
	let initiallyScrolledFor = $state('');
	const selectedConversation = $derived(
		selectedId && isContact(selectedId)
			? conversationQuery(
					companyId,
					selectedId,
					undefined,
					undefined,
					true,
					principalProjection.view?.actor_id ?? 'owner',
					() => ownerAccess
				)
			: null
	);
	$effect(() => selectedConversation?.attach());
	const messages = $derived(selectedConversation?.messages ?? []);
	const visibleMessages = $derived(messages);
	const turn = $derived(selectedConversation?.activeTurn ?? null);

	$effect(() => {
		const nextPeople = people;
		if (!ownerAccess && !collaboration && !collaborationProjection.failure) return;
		if (ownerAccess && !cockpit && !cockpitProjection.failure) return;
		const requestedPerson = page.url.searchParams.get('person');
		if (requestedPerson && nextPeople.some((person) => person.actor_id === requestedPerson)) {
			selectedId = requestedPerson;
		} else if (!nextPeople.some((person) => person.actor_id === selectedId)) {
			selectedId =
				(requestedPerson && nextPeople.some((person) => person.actor_id === requestedPerson)
					? requestedPerson
					: null) ??
				nextPeople.find((person) => person.kind === 'exec')?.actor_id ??
				nextPeople.find((person) => person.kind === 'staff')?.actor_id ??
				'';
		}
	});

	const draftKey = $derived(
		principalProjection.view?.actor_id && selectedId
			? roomDraftKey(companyId, principalProjection.view.actor_id, `person:${selectedId}`, null)
			: ''
	);
	let activeDraftKey = $state<string | null>(null);
	const sending = $derived(sendingFor !== '' && sendingFor === draftKey);
	$effect(() => {
		const nextKey = draftKey;
		untrack(() => {
			const previousKey = activeDraftKey;
			if (!nextKey || nextKey === previousKey) return;
			const preserveUnscopedDraft =
				previousKey === null && (composer.length > 0 || composerFiles.length > 0);
			if (previousKey) {
				writeRoomDraft(previousKey, {
					body: composer,
					commandId: null,
					updatedAt: new Date().toISOString()
				});
			}
			activeDraftKey = nextKey;
			if (preserveUnscopedDraft) return;
			composer = readRoomDraft(nextKey).body;
			composerFiles = [];
			sendError = '';
			sendNotice = '';
			anchoredTurnId = null;
			transcriptTailHeight = 0;
		});
	});
	$effect(() => {
		const key = activeDraftKey;
		if (!key) return;
		const target = { key, body: composer };
		const save = () =>
			writeRoomDraft(target.key, {
				body: target.body,
				commandId: null,
				updatedAt: new Date().toISOString()
			});
		const timer = setTimeout(save, 120);
		return () => {
			clearTimeout(timer);
			save();
		};
	});
	function flushDraft() {
		const key = activeDraftKey;
		if (!key) return;
		writeRoomDraft(key, { body: composer, commandId: null, updatedAt: new Date().toISOString() });
	}

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

	let jumpedToRequest = $state('');
	$effect(() => {
		const id = page.url.searchParams.get('message');
		const key = `${companyId}:${selectedId}:${id}`;
		if (!id || key === jumpedToRequest || !messages.some((message) => message.id === id)) return;
		jumpedToRequest = key;
		void tick().then(() => jumpToMessage(id));
	});

	async function submitMessage(event: SubmitEvent) {
		event.preventDefault();
		const text = composer.trim();
		const conversation = selectedConversation;
		const key = draftKey;
		const participant = selected;
		if (!text || sending || !canSend || !conversation || !key || !participant) return;
		sendingFor = key;
		sendError = '';
		sendNotice = '';
		const sent = composer;
		const files = composerFiles;
		const sentCompanyId = companyId;
		const sentActorId = principalProjection.view?.actor_id ?? '';
		const sentPersonId = selectedId;
		const participantName = participant.display;
		const sentAsOwner = ownerAccess;
		composer = '';
		try {
			const contextPath = cockpitContextPath(companyId, page.url);
			const result = await conversation.send(text, files, contextPath, false, !!turn);
			const stillViewingSubmission =
				draftKey === key &&
				companyId === sentCompanyId &&
				principalProjection.view?.actor_id === sentActorId &&
				selectedId === sentPersonId;
			if (sentAsOwner && stillViewingSubmission) void attentionProjection.refresh();
			if (!stillViewingSubmission) return;
			composerFiles = [];
			if (result.interrupted) {
				sendNotice = `${participantName} was interrupted and your new direction is queued.`;
			} else if (!contextPath || result.contextOmitted) {
				sendNotice = 'Message sent without the current-screen link.';
			}
		} catch (cause) {
			const stillViewingSubmission =
				draftKey === key &&
				companyId === sentCompanyId &&
				principalProjection.view?.actor_id === sentActorId &&
				selectedId === sentPersonId;
			if (stillViewingSubmission) {
				composer = sent;
				sendError = cause instanceof Error ? cause.message : 'Your message was not delivered.';
			} else {
				writeRoomDraft(key, { body: sent, commandId: null, updatedAt: new Date().toISOString() });
			}
		} finally {
			if (sendingFor === key) sendingFor = '';
		}
	}

	const people = $derived(
		(ownerAccess ? (cockpit?.people ?? []) : (collaboration?.people ?? [])).filter(
			(person) => person.kind !== 'owner' && person.kind !== 'system'
		)
	);
	const exec = $derived(people.find((person) => person.kind === 'exec') ?? null);
	const teams = $derived(ownerAccess ? (cockpit?.teams ?? []) : (collaboration?.teams ?? []));
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
	const graph = $derived(
		ownerAccess ? (attention?.workGraph ?? null) : (collaboration?.work_graph ?? null)
	);
	const selectedWork = $derived(
		selected ? (graph?.work ?? []).filter((work) => work.owner_id === selected.actor_id) : []
	);

	const canSend = $derived(
		(ownerAccess
			? cockpit?.source_health.orgintel === 'available'
			: collaboration?.source_health.orgintel === 'available') &&
			selected !== null &&
			isContact(selected.actor_id)
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

	function exceptionalState(person: Person): string | null {
		if ('model_cooldown' in person && person.model_cooldown) return 'cooling down';
		if (person.session_running) return 'working';
		return null;
	}

	function isContact(actorId: string): boolean {
		const person = people.find((candidate) => candidate.actor_id === actorId);
		return person?.kind === 'exec' || teams.some((team) => team.lead_actor_id === actorId);
	}

	function teamState(team: Team): string {
		const members = `${team.member_count} member${team.member_count === 1 ? '' : 's'}`;
		const moving = `${team.in_motion_count} in motion`;
		const blocked = team.blocked_count ? ` · ${team.blocked_count} blocked` : '';
		return `${members} · ${moving}${blocked}`;
	}

	function isOwnerTeam(team: Team): team is CockpitTeam {
		return 'outcome_standard_source' in team;
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

	function selectTeam(team: Team) {
		const lead = people.find((person) => person.actor_id === team.lead_actor_id);
		if (lead) selectPerson(lead.actor_id);
	}
	// Follow newly arrived action cards only while the reader is already at the end.
	$effect.pre(() => {
		chatAttention.map((item) => item.id).join(':');
		const scroller = scrollEl;
		if (scroller && scroller.scrollHeight - scroller.clientHeight - scroller.scrollTop < 48) {
			void tick().then(() => scroller.scrollTo({ top: scroller.scrollHeight }));
		}
	});
</script>

<svelte:window onpagehide={flushDraft} />
<section class="people-talk cockpit-pane">
	{#if selected}
		<header class="talk-head">
			<div class="talk-profile">
				<IntelligencePopover
					{companyId}
					actorId={selected.actor_id}
					label={selected.display}
					align="start"
				>
					{#snippet children(tooltipId)}
						<button
							type="button"
							class="talk-identity"
							aria-label={`${selected.display} intelligence settings`}
							aria-describedby={tooltipId}
							title={`${roleLabel(selected.role)} · ${selected.actor_id}`}
						>
							<span class="person-avatar tone-{personTone({ actor_id: selected.actor_id })}"
								>{initials(selected.display)}</span
							>
							<span class="talk-who"><strong>{selected.display}</strong></span>
						</button>
					{/snippet}
				</IntelligencePopover>
				{#if selectedTeam && isOwnerTeam(selectedTeam)}
					<select
						class="team-standard"
						aria-label="Team quality target"
						title="Quality target for future team coordination and new work. This is separate from model thinking effort."
						value={selectedTeam.outcome_standard}
						disabled={!ownerAccess || standardSaving}
						onchange={(event) => void changeStandard(event.currentTarget)}
					>
						<option value="fast">Fast</option><option value="thorough">Thorough</option><option
							value="exceptional">Exceptional</option
						><option value="frontier">Frontier</option>
					</select>
				{/if}
				{#if standardError}<p class="standard-error" role="alert">{standardError}</p>{/if}
			</div>
			{#if canSend && visibleMessages.length}
				<ConversationHistoryTools
					messages={visibleMessages}
					participantName={selected.actor_id === 'exec' ? 'Exec' : selected.display}
					onjump={jumpToMessage}
				/>
			{/if}
			{#if exceptionalState(selected)}
				<span
					class="profile-presence {selected.session_running ? 'working' : ''}"
					title={exceptionalState(selected) ?? undefined}
					aria-label={exceptionalState(selected) ?? undefined}
				>
					<MatrixGlyph
						rows={selected.session_running ? GLYPHS.dots : GLYPHS.ring}
						size={9}
						glow={selected.session_running}
					/>
				</span>
			{/if}
			{@render actions?.(visibleMessages.at(-1)?.text ?? '')}
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
				{#each chatAttention as item (item.id)}
					<div class="chat-attention"><AttentionCard {companyId} {item} inChat /></div>
				{/each}

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
					actionLabel={turn ? 'Queue direction' : 'Send'}
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
								onclick={() => selectPerson(selectedTeamLead!.actor_id)}
							>
								Talk to {selectedTeamLead.display} ▸
							</button>
						{:else if exec}
							<p>This specialist is currently unassigned; the Exec is accountable.</p>
							<button
								type="button"
								class="contact-route"
								onclick={() => selectPerson(exec!.actor_id)}
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

<style>
	.chat-attention {
		margin: 12px 16px;
		min-width: 0;
	}
	.talk-head {
		height: 56px;
		box-sizing: border-box;
		padding: 8px 12px;
		gap: 8px;
	}
	.talk-profile {
		min-width: 0;
		flex: 1;
		display: flex;
		align-items: center;
		gap: 12px;
		position: relative;
	}
	.talk-profile :global(.intelligence-hover) {
		min-width: 0;
	}
	.talk-profile :global(.intelligence-popover) {
		max-width: calc(100vw - 80px);
	}
	.talk-identity .person-avatar {
		width: 26px;
		height: 26px;
		flex: 0 0 26px;
	}
	.talk-identity {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
		max-width: 100%;
		padding: 0;
		border: 0;
		background: transparent;
		color: inherit;
		text-align: left;
		font: inherit;
		cursor: help;
	}
	.talk-identity:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 3px;
	}
	.team-standard {
		flex: 0 0 auto;
		max-width: 112px;
		height: 28px;
		padding: 2px 6px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--intent-work);
		font: 500 var(--t-label) var(--font-ui);
	}
	.standard-error {
		position: absolute;
		z-index: 101;
		top: 100%;
		left: 0;
		background: var(--surface-pane);
		border: 1px solid var(--border);
		margin: 6px 0 0;
		padding: 6px 12px;
		color: var(--state-danger);
		font-size: var(--t-label);
	}

	@media (max-width: 600px) {
		.talk-profile {
			gap: 8px;
		}
		.talk-identity {
			gap: 6px;
		}
		.team-standard {
			max-width: 104px;
		}
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
