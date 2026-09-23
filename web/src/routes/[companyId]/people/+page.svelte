<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import FileText from '@lucide/svelte/icons/file-text';
	import ConversationDocument from '$lib/components/ConversationDocument.svelte';
	import Search from '@lucide/svelte/icons/search';
	import Users from '@lucide/svelte/icons/users';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import LoaderCircle from '@lucide/svelte/icons/loader-circle';
	import MessageCircleQuestion from '@lucide/svelte/icons/message-circle-question';
	import Bell from '@lucide/svelte/icons/bell';
	import PersonConversation from '$lib/components/PersonConversation.svelte';
	import RoomConversation from '$lib/components/RoomConversation.svelte';
	import RoomManager from '$lib/components/RoomManager.svelte';
	import {
		companyPrincipalQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		attentionQuery
	} from '$lib/model/queries.svelte';
	import {
		roomsQuery,
		recentDirectConversationsQuery,
		roomMessageSearchQuery
	} from '$lib/model/room-queries.svelte';
	import { createRoom, type Room } from '$lib/model/rooms';

	const companyId = $derived(page.params.companyId ?? '');
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(principal.view?.membership_role === 'owner');
	const cockpit = $derived(cockpitQuery(companyId, () => owner));
	const collaboration = $derived(collaborationBootstrapQuery(companyId, () => principal.view));
	const attention = $derived(attentionQuery(companyId, () => owner));
	const people = $derived(
		(owner ? cockpit.view?.people : collaboration.view?.people)?.filter(
			(p) => p.kind !== 'system' && p.actor_id !== principal.view?.actor_id
		) ?? []
	);
	type DirectoryPerson = (typeof people)[number];
	const teams = $derived((owner ? cockpit.view?.teams : collaboration.view?.teams) ?? []);
	const contacts = $derived(
		people.filter((p) => p.kind === 'exec' || teams.some((t) => t.lead_actor_id === p.actor_id))
	);
	const rooms = $derived(roomsQuery(companyId));
	const recent = $derived(
		recentDirectConversationsQuery(companyId, () => !!principal.view?.actor_id)
	);
	const roomId = $derived(page.url.searchParams.get('room') ?? '');
	const personId = $derived(page.url.searchParams.get('person') ?? '');
	const selectedPerson = $derived(people.find((p) => p.actor_id === personId));
	const selectedStaff = $derived(
		selectedPerson?.kind === 'staff' &&
			!contacts.some((person) => person.actor_id === selectedPerson.actor_id)
	);
	const documentOpen = $derived(page.url.searchParams.has('document'));
	const linkedRoomId = $derived(
		roomId || rooms.rooms.find((r) => directPerson(r)?.actor_id === personId)?.id || ''
	);
	const selectedRoom = $derived(rooms.rooms.find((room) => room.id === roomId) ?? null);
	const selectedDirectActorId = $derived(
		selectedRoom ? (directPerson(selectedRoom)?.actor_id ?? '') : ''
	);
	function toggleDocument() {
		const url = new URL(page.url);
		if (documentOpen) url.searchParams.delete('document');
		else url.searchParams.set('document', '');
		void goto(url, { noScroll: true, keepFocus: true });
	}
	const explicitSelection = $derived(!!roomId || page.url.searchParams.has('person'));
	const requestedView = $derived(page.url.searchParams.get('view'));
	const directory = $derived(
		requestedView === 'people' ||
			(requestedView === null &&
				page.url.searchParams.has('person') &&
				owner &&
				selectedPerson?.kind === 'staff' &&
				!teams.some((team) => team.lead_actor_id === selectedPerson.actor_id))
	);
	let search = $state('');
	let debouncedSearch = $state('');
	$effect(() => {
		const value = search.trim();
		const timer = setTimeout(() => (debouncedSearch = value), 200);
		return () => clearTimeout(timer);
	});
	const messageSearch = $derived(roomMessageSearchQuery(companyId, debouncedSearch));
	function directPerson(room: Room) {
		if (room.kind !== 'direct' || !principal.view) return undefined;
		return people.find((p) => {
			const [a, b] = [p.actor_id, principal.view!.actor_id].sort();
			return (
				room.canonical_key ===
				`direct:${new TextEncoder().encode(a).length}:${a}:${new TextEncoder().encode(b).length}:${b}`
			);
		});
	}
	function attentionFor(actorId: string) {
		if (!owner || attention.status !== 'live') return [];
		if (actorId === 'exec') return attention.view?.items ?? [];
		const team = teams.find((team) => team.lead_actor_id === actorId);
		const members = new Set([
			actorId,
			...people
				.filter((person) => team && person.team_id === team.id)
				.map((person) => person.actor_id)
		]);
		return (attention.view?.items ?? []).filter((item) => {
			const sourceActor =
				item.responsibleActor?.id ?? item.runtimeAttach?.requestingActor ?? item.briefAuthor?.id;
			const workOwner = attention.view?.workGraph?.work.find(
				(work) => work.id === item.workId
			)?.owner_id;
			return (sourceActor && members.has(sourceActor)) || (workOwner && members.has(workOwner));
		});
	}
	function personStatus(actorId: string) {
		const person = people.find((candidate) => candidate.actor_id === actorId);
		const peopleAreLive = owner ? cockpit.status === 'live' : collaboration.status === 'live';
		const needs = attentionFor(actorId).filter((item) => !item.preparing);
		const need =
			needs.find((item) => item.source.kind === 'conversation_owner_need') ?? needs.at(0);
		return {
			working: peopleAreLive && person?.session_running === true,
			need: need?.source.kind === 'conversation_owner_need' ? 'reply' : need ? 'attention' : null
		} as const;
	}
	let openingPerson = $state('');
	let openingError = $state('');
	const pendingDirect = new Map<string, string>();
	async function openPerson(event: MouseEvent | null, id: string) {
		const person = people.find((p) => p.actor_id === id);
		event?.preventDefault();
		if (openingPerson || !principal.view) return;
		if (contacts.some((contact) => contact.actor_id === id)) {
			await goto(href(id));
			return;
		}
		const company = companyId,
			actor = principal.view.actor_id;
		const existing = rooms.rooms.find((r) => directPerson(r)?.actor_id === id);
		if (existing) {
			await goto(href('', existing.id));
			return;
		}
		openingPerson = id;
		openingError = '';
		const key = `${company}:${actor}:${id}`;
		const command = pendingDirect.get(key) ?? crypto.randomUUID();
		pendingDirect.set(key, command);
		try {
			const room = await createRoom(company, {
				kind: 'direct',
				title: person?.display ?? id,
				participant_actor_ids: [id],
				command_id: command
			});
			if (company !== companyId || actor !== principal.view?.actor_id) return;
			await rooms.refresh();
			pendingDirect.delete(key);
			await goto(href('', room.id));
		} catch (cause) {
			if (company === companyId)
				openingError = cause instanceof Error ? cause.message : 'Could not open conversation.';
		} finally {
			openingPerson = '';
		}
	}
	$effect(() => {
		if (
			!owner ||
			!roomId ||
			page.url.searchParams.has('thread') ||
			page.url.searchParams.has('message') ||
			page.url.searchParams.has('mention') ||
			page.url.searchParams.has('focus')
		)
			return;
		const room = rooms.rooms.find((r) => r.id === roomId);
		const person = room ? directPerson(room) : null;
		if (person && contacts.some((p) => p.actor_id === person.actor_id))
			void goto(href(person.actor_id), { replaceState: true, noScroll: true });
	});
	const rows = $derived.by(() => {
		const query = search.trim().toLocaleLowerCase();
		return recent.conversations.flatMap((conversation) => {
			const person = people.find((candidate) => candidate.actor_id === conversation.person_actor_id);
			if (
				!person ||
				(query && !`${person.display} ${person.role}`.toLocaleLowerCase().includes(query))
			)
				return [];
			const contact = owner && contacts.some((candidate) => candidate.actor_id === person.actor_id);
			return [
				{
					key: `room:${conversation.room_id}`,
					name: person.display,
					person: person.actor_id,
					room: contact ? '' : conversation.room_id,
					hint: teams.find((team) => team.id === person.team_id)?.name ?? person.role
				}
			];
		});
	});
	function matchesDirectory(value: { display: string; role: string }, query: string) {
		return !query || `${value.display} ${value.role}`.toLocaleLowerCase().includes(query);
	}
	const directoryTeams = $derived.by(() => {
		const query = search.trim().toLocaleLowerCase();
		return teams
			.map((team) => {
				const lead = people.find((person) => person.actor_id === team.lead_actor_id) ?? null;
				const members = people.filter(
					(person) => person.team_id === team.id && person.actor_id !== team.lead_actor_id
				);
				const teamMatches = `${team.name} ${team.brief}`.toLocaleLowerCase().includes(query);
				return {
					team,
					lead,
					members,
					teamMatches,
					matches:
						teamMatches ||
						(lead && matchesDirectory(lead, query)) ||
						members.some((member) => matchesDirectory(member, query))
				};
			})
			.filter((entry) => entry.matches);
	});
	const directoryExec = $derived(people.find((person) => person.kind === 'exec') ?? null);
	const directoryUnassigned = $derived.by(() => {
		const query = search.trim().toLocaleLowerCase();
		return people.filter(
			(person) =>
				person.kind === 'staff' &&
				!teams.some((team) => team.id === person.team_id) &&
				matchesDirectory(person, query)
		);
	});
	const directoryHumans = $derived.by(() => {
		const query = search.trim().toLocaleLowerCase();
		return people.filter((person) => person.kind === 'human' && matchesDirectory(person, query));
	});
	function setDirectory(next: boolean) {
		const url = new URL(page.url);
		if (next) url.searchParams.set('view', 'people');
		else url.searchParams.set('view', 'conversations');
		void goto(url, { noScroll: true, keepFocus: true });
	}
	function href(person: string, room = '') {
		const params = new URLSearchParams(room ? { room } : { person });
		if (directory) params.set('view', 'people');
		const document = page.url.searchParams.get('document');
		if (document !== null) params.set('document', document);
		return `/${encodeURIComponent(companyId)}/people?${params}`;
	}
	function created(room: Room) {
		void goto(href('', room.id));
	}
</script>

<svelte:head
	><title
		>Conversations — {cockpit.view?.company.name ??
			collaboration.view?.company.name ??
			companyId}</title
	></svelte:head
>
<div
	class="conversation-workspace"
	class:selected={explicitSelection}
	class:with-document={documentOpen}
>
	<aside class="conversation-index cockpit-pane" aria-label="Conversations">
		<header class="cockpit-pane-head">
			<h1>Conversations</h1>
			<RoomManager
				{companyId}
				actorId={principal.view?.actor_id ?? ''}
				{people}
				oncreated={created}
				onperson={owner ? (id) => goto(href(id)) : undefined}
				contactIds={contacts.map((p) => p.actor_id)}
			/>
		</header>
		<label class="search"
			><Search size={15} /><input
				aria-label="Search conversations and people"
				type="search"
				placeholder="Search"
				bind:value={search}
			/></label
		>
		<nav class="tabs" aria-label="Conversation list">
			<button class:active={!directory} onclick={() => setDirectory(false)}>Conversations</button
			><button class:active={directory} onclick={() => setDirectory(true)}>People</button>
		</nav>
		<div class="entries" aria-busy={!!openingPerson}>
			{#if openingError}<p class="empty" role="alert">{openingError}</p>{/if}
			{#snippet personStatuses(actorId: string, name: string)}
				{@const status = personStatus(actorId)}
				{#if status.working || status.need}<span
					class="row-statuses"
					class:multiple={status.working && status.need !== null}
					>{#if status.working}<span
							class="row-status working"
							title={`${name} is working`}
							aria-label={`${name} is working`}
							><LoaderCircle size={14} aria-hidden="true" /><span>Working</span></span
						>{/if}{#if status.need === 'reply'}<span
							class="row-status reply"
							title={`${name} is waiting for your reply`}
							aria-label={`${name} is waiting for your reply`}
							><MessageCircleQuestion size={14} aria-hidden="true" /><span>Reply</span></span
						>{:else if status.need === 'attention'}<span
							class="row-status attention"
							title={`${name} needs your attention`}
							aria-label={`${name} needs your attention`}
							><Bell size={14} aria-hidden="true" /><span>Needs you</span></span
						>{/if}</span
				>{/if}
			{/snippet}
			{#snippet directoryPerson(
				person: DirectoryPerson,
				cue: string,
				cueTitle: string,
				kind: string
			)}
				<a
					class="directory-person {kind}"
					href={href(person.actor_id)}
					onclick={(event) => void openPerson(event, person.actor_id)}
					aria-current={(!roomId && personId === person.actor_id) ||
					selectedDirectActorId === person.actor_id
						? 'page'
						: undefined}
				>
					<span class="avatar"
						>{person.display
							.split(/\s+/)
							.slice(0, 2)
							.map((part) => part[0])
							.join('')}</span
					>
					<span class="directory-person-copy"
						><span class="name">{person.display}</span>{@render personStatuses(
							person.actor_id,
							person.display
						)}<span class="directory-cue" title={cueTitle}
							>{cue}</span
						></span
					>
				</a>
			{/snippet}
			{#if directory}
				{#if directoryExec && matchesDirectory(directoryExec, search.trim().toLocaleLowerCase())}
					<section class="directory-section executive-section" aria-label="Executive">
						{@render directoryPerson(
							directoryExec,
							'Executive chat',
							'Open the executive conversation',
							'executive'
						)}
					</section>
				{/if}
				{#each directoryTeams as entry (entry.team.id)}
					<section class="directory-section team-section" aria-label={entry.team.name}>
						<header class="team-directory-head" title={entry.team.brief}>
							<span><Users size={14} aria-hidden="true" /> {entry.team.name}</span>
							{#if entry.lead}<a
									class="team-talk"
									href={href(entry.lead.actor_id)}
									onclick={(event) => void openPerson(event, entry.lead!.actor_id)}
									title={`Talk to ${entry.lead.display}`}>Talk to lead</a
								>{/if}
						</header>
						{#if entry.lead}{@render directoryPerson(
								entry.lead,
								'Lead chat',
								'Accountable team lead. Open their conversation.',
								'lead'
							)}{/if}
						{#each entry.members.filter((member) => entry.teamMatches || matchesDirectory(member, search
										.trim()
										.toLocaleLowerCase())) as member (member.actor_id)}
							{@render directoryPerson(
								member,
								'Chat',
								`Open a direct conversation to co-work with ${member.display}`,
								'member'
							)}
						{/each}
					</section>
				{/each}
				{#if directoryUnassigned.length}
					<section class="directory-section" aria-label="Unassigned staff">
						<header class="team-directory-head">
							<span>Unassigned</span>{#if directoryExec}<a
									class="team-talk"
									href={href(directoryExec.actor_id)}
									onclick={(event) => void openPerson(event, directoryExec!.actor_id)}
									title="Talk to the Exec, who is accountable for unassigned staff">Talk to Exec</a
								>{/if}
						</header>
						{#each directoryUnassigned as member (member.actor_id)}
							{@render directoryPerson(
								member,
								'Chat',
								`Open a direct conversation to co-work with ${member.display}`,
								'member'
							)}
						{/each}
					</section>
				{/if}
				{#if directoryHumans.length}
					<section class="directory-section" aria-label="Human colleagues">
						<header class="team-directory-head"><span>Colleagues</span></header>
						{#each directoryHumans as colleague (colleague.actor_id)}
							{@render directoryPerson(
								colleague,
								'Colleague',
								`Open a direct conversation with ${colleague.display}`,
								'colleague'
							)}
						{/each}
					</section>
				{/if}
				{#if !directoryExec && !directoryTeams.length && !directoryUnassigned.length && !directoryHumans.length}<p
						class="empty"
					>
						No people match.
					</p>{/if}
			{:else}
				{#each rows as row (row.key)}
					<div
						class="entry"
						class:current={row.room ? roomId === row.room : !roomId && personId === row.person}
					>
						<a
							onclick={(event) => {
								if (row.person && !row.room) void openPerson(event, row.person);
							}}
							href={href(row.person, row.room)}
							aria-current={(row.room ? roomId === row.room : !roomId && personId === row.person)
								? 'page'
								: undefined}
							title={row.hint}
						>
							<span class="avatar"
								>{row.name
										.split(/\s+/)
										.slice(0, 2)
										.map((s) => s[0])
										.join('')}</span
							><span class="name">{row.name}</span>{#if row.person}{@render personStatuses(
									row.person,
									row.name
								)}{/if}
						</a>
					</div>
				{:else}<p class="empty">
						{recent.status === 'unknown' ? 'Loading conversations…' : search.trim() ? 'No matches.' : 'No conversations yet. Find someone in People to start one.'}
					</p>{/each}
			{/if}
			{#if search.trim() && !directory}
				{#each messageSearch.messages.filter((message) => recent.conversations.some((conversation) => conversation.room_id === message.room_id)) as message (message.id)}
					<a
						class="search-result"
						href={`${href('', message.room_id)}${message.thread_root_message_id ? `&thread=${message.thread_root_message_id}` : ''}&focus=${message.id}`}
						><strong
							>{people.find((p) => p.actor_id === message.from_actor)?.display ??
								message.from_actor}</strong
						><span>{message.snippet}</span></a
					>
				{/each}
				{#if messageSearch.hasMore}<button class="more" onclick={() => messageSearch.loadMore()}
						>More messages</button
					>{/if}
				{/if}
			{#if recent.failure || messageSearch.failure}<p
					class="empty"
					role="status"
				>
					{recent.failure?.message ?? messageSearch.failure?.message}
				</p>{/if}
		</div>
	</aside>
	<section class="conversation-main" aria-label="Selected conversation">
		<a class="back" href={`/${companyId}/people${directory ? "?view=people" : ""}`}><ArrowLeft size={16} /> {directory ? "People" : "Conversations"}</a>
		{#if roomId}<RoomConversation
				>{#snippet actions()}<button
						class="document-toggle"
						title="Open a document alongside this conversation"
						aria-label="Open document"
						onclick={toggleDocument}><FileText size={17} /></button
					>{/snippet}</RoomConversation
			>{:else if selectedStaff}<div class="staff-direct-route">
				<p>{selectedPerson?.display}</p>
				<p>Open a direct conversation to co-work on a specific task.</p>
				<button
					class="more"
					disabled={!!openingPerson}
					onclick={() => selectedPerson && void openPerson(null, selectedPerson.actor_id)}
					>{openingPerson ? 'Opening…' : 'Open conversation'}</button
				>{#if openingError}<p role="alert">{openingError}</p>{/if}
			</div>{:else if owner && selectedPerson && selectedPerson.kind !== 'human'}<PersonConversation
				>{#snippet actions(context)}<button
						class="document-toggle"
						title="Open a document alongside this conversation"
						aria-label="Open document"
						onclick={toggleDocument}><FileText size={17} /></button
					>{#if selectedPerson}<RoomManager
							{companyId}
							actorId={principal.view?.actor_id ?? ''}
							{people}
							initialParticipants={[personId]}
							initialContext={context}
							label="Add people"
							oncreated={created}
						/>{/if}{/snippet}</PersonConversation
			>{:else if selectedPerson}<div class="empty">
				<p>{selectedPerson.display}</p>
				<button
					class="more"
					disabled={!!openingPerson}
					onclick={() => openPerson(null, selectedPerson.actor_id)}
					>{openingPerson ? 'Opening…' : 'Open conversation'}</button
				>{#if openingError}<p role="alert">{openingError}</p>{/if}
			</div>{:else}<div class="empty">Choose a conversation or start a new one.</div>{/if}
	</section>
	{#if documentOpen}<ConversationDocument
			{companyId}
			companyUuid={collaboration.view?.company.company_id ?? null}
			actorId={principal.view?.actor_id ?? ''}
			roomId={linkedRoomId}
			onclose={toggleDocument}
		/>{/if}
</div>

<style>
	.conversation-workspace {
		display: grid;
		grid-template-columns: 260px minmax(0, 1fr);
		gap: var(--pane-gap);
		width: 100%;
		height: 100%;
		min-height: 0;
		overflow: hidden;
	}
	.conversation-index,
	.conversation-main {
		min-height: 0;
		min-width: 0;
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}
	.conversation-main :global(.people-talk),
	.conversation-main :global(.rooms-screen) {
		flex: 1;
		min-height: 0;
	}
	.conversation-main {
		container: conversation / inline-size;
	}
	.conversation-index > header {
		flex-wrap: wrap;
		gap: 9px;
		padding: 12px;
	}
	.conversation-index > header :global(.room-manage-trigger) {
		width: 100%;
		height: 32px;
		justify-content: center;
	}
	.search {
		display: flex;
		gap: 8px;
		align-items: center;
		padding: 12px;
		color: var(--text-tertiary);
	}
	.search input {
		width: 100%;
		min-width: 0;
		background: transparent;
		border: 0;
		font: inherit;
		outline-offset: 3px;
		color: var(--ink);
	}
	.tabs {
		display: flex;
		gap: 4px;
		padding: 0 8px 8px;
		border-bottom: 1px solid var(--border);
	}
	.tabs button {
		flex: 1;
		padding: 7px 4px;
		border: 0;
		background: transparent;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		cursor: pointer;
		font: inherit;
		font-size: var(--t-label);
	}
	.tabs button.active {
		background: var(--intent-conversation-soft);
		color: var(--intent-conversation);
		font-weight: 600;
	}
	.entries {
		overflow-y: auto;
		overflow-x: hidden;
		min-height: 0;
		padding: 6px;
	}
	.directory-section {
		margin-bottom: 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		overflow: hidden;
	}
	.executive-section {
		border-color: color-mix(in srgb, var(--intent-conversation) 28%, var(--border));
	}
	.team-directory-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 7px 8px;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.team-directory-head > span {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.team-talk {
		flex: none;
		color: var(--intent-conversation);
		font-weight: 600;
		text-decoration: none;
	}
	.directory-person {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px;
		color: var(--ink);
		text-decoration: none;
	}
	.directory-person:hover,
	.directory-person:focus-visible,
	.directory-person[aria-current='page'] {
		background: var(--intent-conversation-soft);
	}
	.directory-person.member {
		padding-left: 28px;
	}
	.directory-person .avatar {
		width: 25px;
		height: 25px;
		border-radius: 7px;
	}
	.directory-person-copy {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 8px;
		min-width: 0;
		flex: 1;
	}
	.directory-cue {
		flex: none;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.directory-person.lead .directory-cue,
	.directory-person.executive .directory-cue {
		color: var(--intent-conversation);
		font-weight: 600;
	}
	.entry {
		display: flex;
		align-items: center;
		border-radius: var(--radius-control);
		margin-bottom: 2px;
	}
	.entry:hover,
	.entry.current {
		background: var(--intent-conversation-soft);
	}
	.entry a {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
		flex: 1;
		padding: 10px 7px;
		color: var(--ink);
		text-decoration: none;
	}
	.avatar {
		width: 30px;
		height: 30px;
		display: grid;
		place-items: center;
		flex: none;
		background: var(--surface-alt);
		border: 1px solid var(--border);
		border-radius: 8px;
		font-size: var(--t-label);
	}
	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--t-body);
	}
	.current .name {
		font-weight: 600;
	}
	.row-statuses {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		flex: none;
	}
	.row-statuses.multiple {
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
	}
	.row-status {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		padding: 3px 5px;
		border-radius: 999px;
		font-size: var(--t-label);
		font-weight: 650;
		line-height: 1;
		white-space: nowrap;
	}
	.row-status.working {
		color: var(--intent-conversation);
		background: var(--intent-conversation-soft);
	}
	.row-status.working :global(svg) {
		animation: status-spin 1.1s linear infinite;
	}
	.row-status.reply,
	.row-status.attention {
		color: var(--intent-authority);
		background: color-mix(in srgb, var(--intent-authority) 12%, transparent);
	}
	@keyframes status-spin {
		to {
			transform: rotate(360deg);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.row-status.working :global(svg) {
			animation: none;
		}
	}
	.empty {
		padding: 12px;
		font-size: var(--t-label);
		color: var(--text-secondary);
	}
	.staff-direct-route {
		padding: 18px;
		color: var(--text-secondary);
	}
	.staff-direct-route p:first-child {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 600;
	}
	.staff-direct-route p + p {
		margin: 5px 0 12px;
		font-size: var(--t-label);
	}
	.staff-direct-route .more {
		width: auto;
		padding: 8px 0;
	}
	.more {
		width: 100%;
		border: 0;
		background: transparent;
		color: var(--intent-conversation);
		padding: 12px;
		cursor: pointer;
	}
	.search-result {
		display: grid;
		gap: 4px;
		padding: 10px;
		color: var(--ink);
		text-decoration: none;
		border-top: 1px solid var(--border);
		font-size: var(--t-label);
	}
	.search-result span {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.back {
		display: none;
	}
	.with-document {
		grid-template-columns: 230px minmax(320px, 0.8fr) minmax(380px, 1.2fr);
	}
	.document-toggle {
		border: 0;
		background: transparent;
		padding: 8px;
		display: grid;
		place-items: center;
		color: var(--text-secondary);
		cursor: pointer;
	}
	@media (min-width: 761px) and (max-width: 1100px) {
		.with-document {
			grid-template-columns: minmax(300px, 1fr) minmax(360px, 1fr);
		}
		.with-document .conversation-index {
			display: none;
		}
	}
	@media (max-width: 760px) {
		.conversation-workspace {
			grid-template-columns: minmax(0, 1fr);
		}
		.conversation-workspace.with-document .conversation-main,
		.conversation-workspace.with-document .conversation-index {
			display: none;
		}
		.conversation-main {
			display: none;
		}
		.selected .conversation-index {
			display: none;
		}
		.selected .conversation-main {
			display: flex;
		}
		.back {
			display: flex;
			gap: 8px;
			padding: 10px 12px;
			align-items: center;
			color: var(--intent-conversation);
			text-decoration: none;
		}
		.pin {
			padding: 12px;
			opacity: 1;
		}
	}
</style>
