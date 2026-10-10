<script lang="ts">
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { initials, personName, teamName } from '$lib/model/initials';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Users from '@lucide/svelte/icons/users';
	import MessagesSquare from '@lucide/svelte/icons/messages-square';
	import UserPlus from '@lucide/svelte/icons/user-plus';
	import SidebarSearch from '$lib/ui/views/SidebarSearch.svelte';
	import SidebarEmpty from '$lib/ui/views/SidebarEmpty.svelte';
	import type { SidebarTone } from '$lib/ui/views/SidebarRow.svelte';
	import { getCoreMembers, getIssuerMembers, type IssuerInvitation } from '$lib/model/members';
	import RoomConversation from '$lib/components/RoomConversation.svelte';
	import RoomManager from '$lib/components/RoomManager.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/ui/glyph/MatrixGlyph.svelte';
	import {
		companyPrincipalQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		attentionQuery
	} from '$lib/model/queries.svelte';
	import {
		roomsQuery,
		recentDirectConversationsQuery,
		recentGroupConversationsQuery,
		roomMessageSearchQuery
	} from '$lib/model/room-queries.svelte';
	import { messagePreview, type Room } from '$lib/model/rooms';
	import { seenThrough, markSeen } from '$lib/model/conversation-seen';
	import { getContext, type Snippet } from 'svelte';

	/* The Exec and team leads open in the same conversation as the rail, at
	 * full width; the layout owns it. */
	const wide = getContext<{ actor: string; render: () => Snippet<[boolean]> }>('wide-conversation');

	const companyId = $derived(page.params.companyId ?? '');
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(principal.view?.membership_role === 'owner');
	const cockpit = $derived(cockpitQuery(companyId, () => owner));
	const collaboration = $derived(
		collaborationBootstrapQuery(companyId, () => (owner ? null : principal.view))
	);
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
	const selectedRoom = $derived(rooms.rooms.find((room) => room.id === roomId) ?? null);
	const selectedDirectActorId = $derived(
		selectedRoom ? (directPerson(selectedRoom)?.actor_id ?? '') : ''
	);
	const explicitSelection = $derived(!!roomId || page.url.searchParams.has('person'));
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
	/* Each person's second line: what they need from the owner, what is stuck, or what they are
	 * producing right now (their own Work, or for a lead, their team's), coloured by which. Someone
	 * with none of these shows their role. */
	function personLine(actorId: string): { text: string; tone: SidebarTone } {
		const person = people.find((candidate) => candidate.actor_id === actorId);
		const fallback = {
			text:
				person?.kind === 'exec'
					? 'Ready when you are'
					: person?.kind === 'human'
						? 'Colleague'
						: (person?.role ?? ''),
			tone: '' as SidebarTone
		};
		if (!owner) return fallback;
		const status = personStatus(actorId);
		if (status.need === 'reply') return { text: 'Waiting for your reply', tone: 'wait' };
		if (status.need === 'attention') return { text: 'Needs your attention', tone: 'wait' };
		const team = teams.find((candidate) => candidate.lead_actor_id === actorId);
		const members = new Set(
			team
				? people.filter((member) => member.team_id === team.id).map((member) => member.actor_id)
				: [actorId]
		);
		const work = (attention.view?.workGraph?.work ?? [])
			.filter((item) => members.has(item.owner_id))
			.toSorted((a, b) => Date.parse(b.updated_at) - Date.parse(a.updated_at));
		const blocked = work.find((item) => item.status === 'blocked');
		if (blocked) return { text: `Blocked: ${blocked.title}`, tone: 'block' };
		const active = work.filter((item) => item.status === 'active');
		if (active.length)
			return {
				text: active.length > 1 ? `${active[0].title} · +${active.length - 1}` : active[0].title,
				tone: 'work'
			};
		if (status.working) return { text: 'Working', tone: 'work' };
		return fallback;
	}

	/* Invitations still waiting, from the account plane that issues them. A local company has none;
	 * without an account session there is nothing to show, not an error. */
	let invitations = $state<IssuerInvitation[]>([]);
	let invitationsFor = '';
	$effect(() => {
		const target = companyId;
		if (!owner || invitationsFor === target) return;
		invitationsFor = target;
		void getCoreMembers(target)
			.then((core) =>
				core.issuer && core.company_id ? getIssuerMembers(core.issuer, core.company_id) : null
			)
			.then((issuer) => {
				if (target === companyId) invitations = issuer?.invitations ?? [];
			})
			.catch(() => {
				if (target === companyId) invitations = [];
			});
	});
	let seenVersion = $state(0);
	$effect(() => {
		const bump = () => (seenVersion += 1);
		window.addEventListener('restless:seen', bump);
		return () => window.removeEventListener('restless:seen', bump);
	});
	/* The conversation in view is seen through its newest message. */
	$effect(() => {
		const open = recent.conversations.find(
			(conversation) =>
				conversation.room_id === roomId || (!roomId && conversation.person_actor_id === personId)
		);
		if (open) markSeen(companyId, open.room_id, open.last_message_id);
	});
	function hasNew(actorId: string): boolean {
		void seenVersion;
		const conversation = recent.conversations.find((entry) => entry.person_actor_id === actorId);
		return (
			!!conversation && conversation.last_message_id > seenThrough(companyId, conversation.room_id)
		);
	}
	function openPerson(event: MouseEvent | null, id: string) {
		event?.preventDefault();
		return goto(href(id));
	}
	const rows = $derived.by(() => {
		const query = search.trim().toLocaleLowerCase();
		return recent.conversations.flatMap((conversation) => {
			const person = people.find(
				(candidate) => candidate.actor_id === conversation.person_actor_id
			);
			if (
				!person ||
				(query && !`${person.display} ${person.role}`.toLocaleLowerCase().includes(query))
			)
				return [];
			const contact = owner && contacts.some((candidate) => candidate.actor_id === person.actor_id);
			return [
				{
					key: `room:${conversation.room_id}`,
					name: personName(person.display),
					person: person.actor_id,
					room: contact ? '' : conversation.room_id,
					hint: ((name) => (name ? teamName(name, companyId) : person.role))(
						teams.find((team) => team.id === person.team_id)?.name
					)
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
	function href(person: string, room = '') {
		const params = new URLSearchParams(room ? { room } : person ? { person } : {});
		return `/${encodeURIComponent(companyId)}/people?${params}`;
	}
	function indexHref() {
		const url = new URL(page.url);
		for (const key of ['room', 'person', 'thread', 'message', 'mention', 'focus'])
			url.searchParams.delete(key);
		url.searchParams.delete('view');
		return `${url.pathname}${url.search}${url.hash}`;
	}
	$effect(() => {
		if (explicitSelection || !owner || recent.status === 'unknown') return;
		if (!window.matchMedia('(min-width: 761px)').matches) return;
		/* Reopen the latest conversation; with none yet, the welcome invites the first one. */
		const latest = rows[0]?.person;
		if (latest) void goto(href(latest), { replaceState: true, noScroll: true, keepFocus: true });
	});
	/* A hit in a conversation with the Exec or a lead opens that conversation
	 * (the full-width rail) at the message; anything else opens its room. */
	function searchHref(message: {
		id: number;
		room_id: string;
		thread_root_message_id: number | null;
	}): string {
		const contact = recent.conversations.find(
			(conversation) =>
				conversation.room_id === message.room_id &&
				contacts.some((person) => person.actor_id === conversation.person_actor_id)
		);
		if (owner && contact && !message.thread_root_message_id)
			return `${href(contact.person_actor_id)}&focus=${message.id}`;
		return `${href('', message.room_id)}${message.thread_root_message_id ? `&thread=${message.thread_root_message_id}` : ''}&focus=${message.id}`;
	}
	function created(room: Room) {
		void goto(href('', room.id));
	}

	/* Two views of the same column, as before the 4 October merge: People is the directory with
	 * what each person is doing; Chats is every conversation, newest first. The choice is
	 * remembered per company; an old ?view= link still picks one. */
	type PeopleView = 'people' | 'chats';
	let preferredView = $state<PeopleView>('people');
	$effect(() => {
		try {
			preferredView =
				localStorage.getItem(`restless:${companyId}:people-view`) === 'chats' ? 'chats' : 'people';
		} catch {
			preferredView = 'people';
		}
	});
	const view = $derived<PeopleView>(
		(() => {
			const requested = page.url.searchParams.get('view');
			if (requested === 'chats' || requested === 'conversations') return 'chats';
			if (requested === 'people') return 'people';
			return preferredView;
		})()
	);
	function chooseView(next: PeopleView) {
		preferredView = next;
		try {
			localStorage.setItem(`restless:${companyId}:people-view`, next);
		} catch {
			/* The choice still holds for this visit. */
		}
		if (page.url.searchParams.has('view')) {
			const url = new URL(page.url);
			url.searchParams.delete('view');
			void goto(`${url.pathname}${url.search}`, {
				replaceState: true,
				noScroll: true,
				keepFocus: true
			});
		}
	}
	let manager = $state<{ open: () => void } | null>(null);

	const recentGroups = $derived(
		recentGroupConversationsQuery(companyId, () => !!principal.view?.actor_id)
	);
	/* Who wrote a message, as the start of its preview: "You: …", "Ines: …". */
	function speaker(actorId: string | undefined): string {
		if (!actorId) return '';
		if (actorId === principal.view?.actor_id) return 'You';
		return personName(people.find((person) => person.actor_id === actorId)?.display ?? '') || '';
	}
	/* Every conversation by its last message, each with what was said. A group room nobody has
	 * written in yet sorts by when it began. */
	const chats = $derived.by(() => {
		void seenVersion;
		const query = search.trim().toLocaleLowerCase();
		const direct = recent.conversations.flatMap((conversation) => {
			const person = people.find(
				(candidate) => candidate.actor_id === conversation.person_actor_id
			);
			if (!person) return [];
			const contact = owner && contacts.some((candidate) => candidate.actor_id === person.actor_id);
			const from = conversation.last_message_from === principal.view?.actor_id ? 'You: ' : '';
			return [
				{
					key: `room:${conversation.room_id}`,
					name: personName(person.display),
					mark: initials(person.display),
					group: false,
					person: person.actor_id,
					room: contact ? '' : conversation.room_id,
					roomId: conversation.room_id,
					at: conversation.last_message_at,
					preview: `${from}${messagePreview(conversation.last_message_preview)}`,
					unread:
						conversation.last_message_id > seenThrough(companyId, conversation.room_id) &&
						personId !== person.actor_id
				}
			];
		});
		const spoken = new Map(recentGroups.conversations.map((group) => [group.room_id, group]));
		const groups = rooms.rooms
			.filter((room) => room.kind !== 'direct' && !room.archived_at)
			.map((room) => {
				const latest = spoken.get(room.id);
				const who = speaker(latest?.last_message_from);
				return {
					key: `room:${room.id}`,
					name: room.title,
					mark: '#',
					group: true,
					person: '',
					room: room.id,
					roomId: room.id,
					at: latest?.last_message_at ?? room.created_at,
					preview: latest
						? `${who ? `${who}: ` : ''}${messagePreview(latest.last_message_preview)}`
						: 'Nothing said yet',
					unread:
						!!latest &&
						latest.last_message_id > seenThrough(companyId, room.id) &&
						roomId !== room.id &&
						latest.last_message_from !== principal.view?.actor_id
				};
			});
		return [...direct, ...groups]
			.filter(
				(chat) => !query || `${chat.name} ${chat.preview}`.toLocaleLowerCase().includes(query)
			)
			.toSorted((a, b) => Date.parse(b.at) - Date.parse(a.at));
	});
	/* An open group room is seen through its newest message, as a direct one is. */
	$effect(() => {
		const open = recentGroups.conversations.find((group) => group.room_id === roomId);
		if (open) markSeen(companyId, open.room_id, open.last_message_id);
	});
	const unreadChats = $derived(chats.filter((chat) => chat.unread).length);
	/* "14:05" today, "Tue" this week, "3 Oct" before that. */
	function chatTime(iso: string): string {
		const at = new Date(iso);
		if (Number.isNaN(at.getTime())) return '';
		const now = new Date();
		if (at.toDateString() === now.toDateString())
			return at.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
		if (now.getTime() - at.getTime() < 6 * 86_400_000)
			return at.toLocaleDateString(undefined, { weekday: 'short' });
		return at.toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
	}
	/* A company with only its Exec so far: offer the next useful steps instead of empty space. */
	const sparse = $derived(
		!search.trim() &&
			!directoryTeams.length &&
			!directoryUnassigned.length &&
			!directoryHumans.length &&
			!invitations.length
	);
	const shownInvitations = $derived(
		invitations.filter(
			(invite) =>
				!search.trim() ||
				invite.email.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase())
		)
	);
	const inviteExpiry = (iso: string) =>
		new Date(iso).toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
</script>

<svelte:head
	><title
		>Conversations — {cockpit.view?.company.name ??
			collaboration.view?.company.name ??
			companyId}</title
	></svelte:head
>
<div class="conversation-workspace" class:selected={explicitSelection}>
	<aside class="conversation-index" aria-label="Conversations">
		<div class="people-head">
			<h1>People</h1>
			<RoomManager
				bind:this={manager}
				{companyId}
				actorId={principal.view?.actor_id ?? ''}
				{people}
				oncreated={created}
				onperson={owner ? (id) => goto(href(id)) : undefined}
				contactIds={contacts.map((p) => p.actor_id)}
			/>
		</div>
		<div class="people-search">
			<SidebarSearch
				bind:value={search}
				placeholder={view === 'chats' ? 'Search conversations' : 'Search people'}
				label="Search conversations and people"
			/>
		</div>
		<div class="people-views" role="tablist" aria-label="Show">
			<button
				type="button"
				role="tab"
				aria-selected={view === 'people'}
				onclick={() => chooseView('people')}><Users size={14} aria-hidden="true" />People</button
			>
			<button
				type="button"
				role="tab"
				aria-selected={view === 'chats'}
				onclick={() => chooseView('chats')}
				><MessagesSquare size={14} aria-hidden="true" />Chats{#if unreadChats}<b
						aria-label={`${unreadChats} unread`}>{unreadChats}</b
					>{/if}</button
			>
		</div>
		<div class="entries">
			{#snippet directoryPerson(person: DirectoryPerson, kind: string)}
				{@const line = personLine(person.actor_id)}
				{@const isLead = kind === 'lead'}
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
						>{person.kind === 'exec' ? 'E' : initials(person.display)}{#if line.tone === 'work'}<i
								class="avatar-working"
								title={`${personName(person.display)} is working`}
								aria-label="Working"
							></i>{/if}</span
					>
					<span class="person-lines"
						><span class="name"
							>{personName(person.display)}{#if isLead}<span class="role-tag">
									· Lead</span
								>{/if}</span
						>{#if line.text}<small class="doing {line.tone}" title={line.text}>{line.text}</small
							>{/if}</span
					>{#if hasNew(person.actor_id) && personId !== person.actor_id}<span
							class="new-dot"
							title="Something new since you last looked"
							aria-label="New messages"
						></span>{/if}
				</a>
			{/snippet}
			{#if view === 'chats'}
				{#each chats as chat (chat.key)}
					<a
						class="directory-person chat"
						class:group={chat.group}
						href={href(chat.person, chat.room)}
						onclick={(event) => {
							if (chat.person && !chat.room) void openPerson(event, chat.person);
						}}
						aria-current={(chat.room ? roomId === chat.room : !roomId && personId === chat.person)
							? 'page'
							: undefined}
					>
						<span class="avatar">{chat.mark}</span>
						<span class="person-lines"
							><span class="name" class:strong={chat.unread}>{chat.name}</span><small
								class="doing"
								title={chat.preview}>{chat.preview}</small
							></span
						><span class="chat-meta"
							><time class="chat-time" datetime={chat.at}>{chatTime(chat.at)}</time
							>{#if chat.unread}<span
									class="new-dot"
									title="New since you last looked"
									aria-label="Unread"
								></span>{/if}</span
						>
					</a>
				{:else}
					{#if search.trim()}<p class="empty">No conversations match.</p>{:else}
						<SidebarEmpty
							text="No conversations yet. Talk to the Exec, or start a group with people you choose."
						>
							{#if directoryExec}<a class="primary" href={href(directoryExec.actor_id)}
									>Message the Exec</a
								>{/if}
							<button type="button" onclick={() => manager?.open()}
								><MessagesSquare size={15} strokeWidth={1.8} aria-hidden="true" />New group chat</button
							>
						</SidebarEmpty>
					{/if}
				{/each}
			{:else}
				{#if directoryExec && matchesDirectory(directoryExec, search.trim().toLocaleLowerCase())}
					<section class="directory-section executive-section" aria-label="Executive">
						{@render directoryPerson(directoryExec, 'executive')}
					</section>
				{/if}
				{#each directoryTeams as entry (entry.team.id)}
					<section
						class="directory-section team-section"
						aria-label={teamName(entry.team.name, companyId)}
					>
						<header class="team-directory-head" title={entry.team.brief}>
							<span>{teamName(entry.team.name, companyId)}</span><em
								>{entry.members.length + (entry.lead ? 1 : 0)}</em
							>
						</header>
						{#if entry.lead}{@render directoryPerson(entry.lead, 'lead')}{/if}
						{#each entry.members.filter((member) => entry.teamMatches || matchesDirectory(member, search
										.trim()
										.toLocaleLowerCase())) as member (member.actor_id)}
							{@render directoryPerson(member, 'member')}
						{/each}
					</section>
				{/each}
				{#if directoryUnassigned.length}
					<section class="directory-section" aria-label="Unassigned staff">
						<header class="team-directory-head">
							<span>Unassigned</span><em>{directoryUnassigned.length}</em>
						</header>
						{#each directoryUnassigned as member (member.actor_id)}
							{@render directoryPerson(member, 'member')}
						{/each}
					</section>
				{/if}
				{#if directoryHumans.length || shownInvitations.length}
					<section class="directory-section" aria-label="Human colleagues">
						<header class="team-directory-head">
							<span>Colleagues</span><em>{directoryHumans.length + shownInvitations.length}</em>
						</header>
						{#each directoryHumans as colleague (colleague.actor_id)}
							{@render directoryPerson(colleague, 'colleague')}
						{/each}
						{#each shownInvitations as invite (invite.id)}
							<a
								class="directory-person colleague invited"
								href={`/${encodeURIComponent(companyId)}/company/members`}
								title="Manage the invitation in Members"
							>
								<span class="avatar">{invite.email.slice(0, 1).toUpperCase()}</span>
								<span class="person-lines"
									><span class="name">{invite.email}</span><small class="doing"
										>Invited · link expires {inviteExpiry(invite.expires_at)}</small
									></span
								>
							</a>
						{/each}
					</section>
				{/if}
				{#if !directoryExec && !directoryTeams.length && !directoryUnassigned.length && !directoryHumans.length && !shownInvitations.length}<p
						class="empty"
					>
						No people match.
					</p>{/if}
				{#if sparse}
					<SidebarEmpty
						text="It's just you and the Exec so far. Leads and staff appear here as the Exec builds the team."
					>
						{#if owner}<a href={`/${encodeURIComponent(companyId)}/company/members`}
								><UserPlus size={15} strokeWidth={1.8} aria-hidden="true" />Invite a colleague</a
							>{/if}
						<button type="button" onclick={() => manager?.open()}
							><MessagesSquare size={15} strokeWidth={1.8} aria-hidden="true" />New group chat</button
						>
					</SidebarEmpty>
				{/if}
			{/if}
			{#if search.trim()}
				{#each messageSearch.messages.filter( (message) => recent.conversations.some((conversation) => conversation.room_id === message.room_id) ) as message (message.id)}
					<a class="search-result" href={searchHref(message)}
						><strong
							>{personName(people.find((p) => p.actor_id === message.from_actor)?.display ?? '') ||
								message.from_actor}</strong
						><span>{message.snippet}</span></a
					>
				{/each}
				{#if messageSearch.hasMore}<button class="more" onclick={() => messageSearch.loadMore()}
						>More messages</button
					>{/if}
			{/if}
			{#if recent.failure || messageSearch.failure}
				<FailureNotice
					error={recent.failure ?? messageSearch.failure}
					subject="conversations"
					onretry={() => (recent.failure ? recent.refresh() : messageSearch.refresh())}
				/>
			{/if}
		</div>
	</aside>
	<section class="conversation-main" aria-label="Selected conversation">
		<a class="back" href={indexHref()}><ArrowLeft size={16} /> People</a>
		{#if wide.actor && wide.actor === personId && !roomId}
			{@render wide.render()(true)}
		{:else if roomId || personId}
			<RoomConversation />
		{:else}
			<div class="conversation-empty cockpit-pane">
				<span class="conversation-empty-mark" aria-hidden="true">
					<MatrixGlyph rows={GLYPHS.group} size={10} />
				</span>
				<h2>Talk to anyone in the company</h2>
				<p>Pick a lead or teammate on the left, or start with the Exec.</p>
				{#if directoryExec}
					<a class="btn primary" href={href(directoryExec.actor_id)}>
						Message {personName(directoryExec.display)}
					</a>
				{/if}
			</div>
		{/if}
	</section>
</div>

<style>
	.conversation-workspace {
		display: grid;
		grid-template-columns: var(--sidebar-width, 264px) minmax(0, 1fr);
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
	.conversation-index {
		padding: 4px 8px 0 6px;
	}
	/* The same head as every sidebar: the title, and the main action as a raised button. */
	.people-head {
		display: flex;
		flex: none;
		align-items: center;
		justify-content: space-between;
		min-height: 40px;
		padding: 0 2px 0 8px;
	}
	.people-head h1 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.people-head :global(.room-manage-trigger) {
		position: relative;
		display: inline-grid;
		place-items: center;
		width: 30px;
		height: 30px;
		padding: 0;
		border: 1px solid var(--edge-control);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
		box-shadow: var(--control-depth), var(--bevel);
		color: var(--ink);
	}
	.people-head :global(.room-manage-trigger span) {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
	.people-search {
		flex: none;
		padding: 6px 0 10px;
	}
	/* A switch between two modes of the list, not an item in it: a two-part toggle with a raised
	 * thumb, so it never reads as a second selection beside the open conversation. */
	.people-views {
		display: grid;
		flex: none;
		grid-template-columns: 1fr 1fr;
		gap: 2px;
		/* The track grows with its buttons, which take a larger tap target on touch screens. */
		min-height: 34px;
		margin-bottom: 8px;
		padding: 3px;
		border-radius: var(--radius-lg);
		background: var(--wash-hover);
	}
	.people-views button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		min-height: 28px;
		padding: 0 8px;
		border: 0;
		border-radius: var(--radius-pane);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-body);
		font-weight: 500;
		cursor: pointer;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.people-views button:hover {
		color: var(--ink);
	}
	.people-views button[aria-selected='true'] {
		background: var(--segment-thumb);
		box-shadow:
			var(--shadow-soft),
			var(--bevel),
			0 0 0 1px var(--border);
		color: var(--ink);
		font-weight: 600;
	}
	.people-views button:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	.people-views b {
		min-width: 18px;
		height: 18px;
		padding: 0 5px;
		border-radius: 9px;
		background: var(--intent-conversation);
		color: var(--on-primary);
		font-size: var(--t-label);
		font-weight: 600;
		line-height: 18px;
	}
	.entries {
		flex: 1 1 auto;
		overflow-y: auto;
		overflow-x: hidden;
		min-height: 0;
		padding: 1px 2px 8px;
	}
	.directory-section {
		display: grid;
		gap: 1px;
	}
	.directory-section + .directory-section {
		margin-top: 12px;
	}
	/* A section label you can read: body size, weight 600, with its count. */
	.team-directory-head {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 30px;
		padding: 0 4px 0 8px;
		color: var(--text-secondary);
		font-size: var(--t-body);
		font-weight: 600;
	}
	.team-directory-head > span {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.team-directory-head em {
		color: var(--text-tertiary);
		font-style: normal;
		font-weight: 400;
	}
	/* A person: their name, then what they are doing in its colour; the selected one raised. */
	.directory-person {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		min-height: 48px;
		padding: 6px 8px;
		border-radius: var(--radius-pane);
		color: var(--ink);
		text-decoration: none;
		transition:
			background-color var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard);
	}
	.directory-person:hover {
		background: var(--wash-hover);
	}
	.directory-person[aria-current='page'] {
		background: var(--surface-raised);
		box-shadow:
			var(--shadow-soft),
			var(--bevel),
			0 0 0 1px var(--border);
	}
	.directory-person[aria-current='page'] .name {
		font-weight: 600;
	}
	.directory-person:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	.person-lines {
		display: grid;
		flex: 1;
		min-width: 0;
	}
	.name {
		min-width: 0;
		overflow: hidden;
		font-size: var(--t-body);
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.name.strong {
		font-weight: 650;
	}
	.role-tag {
		color: var(--text-tertiary);
		font-weight: 400;
	}
	.doing {
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-body);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.doing.work {
		color: var(--intent-feedback);
	}
	.doing.wait {
		color: var(--intent-authority);
	}
	.doing.block {
		color: var(--state-danger);
	}
	.chat-meta {
		display: grid;
		flex: none;
		justify-items: end;
		gap: 4px;
		padding-top: 2px;
	}
	.chat-time {
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
	}
	.new-dot {
		flex: none;
		align-self: center;
		width: 8px;
		height: 8px;
		border-radius: 999px;
		background: var(--intent-conversation);
	}
	.chat-meta .new-dot {
		align-self: auto;
	}
	/* Avatars are tinted by who they are, so the Exec, the leads and their staff, and human
	 * colleagues read apart at a glance; a green mark says someone is working. */
	.avatar {
		position: relative;
		display: grid;
		flex: none;
		place-items: center;
		width: 26px;
		height: 26px;
		margin-top: 1px;
		border-radius: 7px;
		background: color-mix(in srgb, var(--intent-conversation) 15%, var(--surface-raised));
		box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--intent-conversation) 18%, transparent);
		color: var(--intent-conversation);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.directory-person.executive .avatar {
		background: color-mix(in srgb, var(--intent-direction) 15%, var(--surface-raised));
		box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--intent-direction) 20%, transparent);
		color: var(--intent-direction);
	}
	.directory-person.colleague .avatar {
		border-radius: 50%;
		background: var(--surface-raised);
		box-shadow: inset 0 0 0 1px var(--border-strong);
		color: var(--text-secondary);
	}
	.directory-person.chat.group .avatar {
		background: transparent;
		box-shadow: inset 0 0 0 1px var(--border-strong);
		color: var(--text-tertiary);
	}
	.directory-person.invited .avatar {
		box-shadow: inset 0 0 0 1px var(--border-strong);
		border: 1px dashed var(--border-strong);
		background: transparent;
	}
	.avatar-working {
		position: absolute;
		right: -2px;
		bottom: -2px;
		width: 9px;
		height: 9px;
		border: 2px solid var(--bg-app);
		border-radius: 50%;
		background: var(--intent-feedback);
	}
	.conversation-empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		padding: var(--space-6);
		text-align: center;
		animation: bridge-disclosure-in var(--motion-disclosure) var(--ease-out) both;
	}
	.conversation-empty-mark {
		display: grid;
		place-items: center;
		width: 44px;
		height: 44px;
		margin-bottom: var(--space-2);
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--intent-conversation-soft);
		color: var(--intent-conversation);
	}
	.conversation-empty h2 {
		font-size: var(--t-head);
	}
	.conversation-empty p {
		max-width: 320px;
		margin: 0 0 var(--space-3);
		color: var(--text-secondary);
	}
	.empty {
		padding: 12px 8px;
		font-size: var(--t-body);
		color: var(--text-secondary);
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
	@media (max-width: 760px) {
		.conversation-workspace {
			grid-template-columns: minmax(0, 1fr);
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
