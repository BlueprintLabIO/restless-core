<script lang="ts">
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { initials, personName, teamName } from '$lib/model/initials';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import Search from '@lucide/svelte/icons/search';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import LoaderCircle from '@lucide/svelte/icons/loader-circle';
	import MessageCircleQuestion from '@lucide/svelte/icons/message-circle-question';
	import Users from '@lucide/svelte/icons/users';
	import MessagesSquare from '@lucide/svelte/icons/messages-square';
	import UserPlus from '@lucide/svelte/icons/user-plus';
	import SidebarRow from '$lib/ui/views/SidebarRow.svelte';
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
		roomMessageSearchQuery
	} from '$lib/model/room-queries.svelte';
	import { type Room } from '$lib/model/rooms';
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
	/* What someone is producing right now, in a few words: their own active
	 * Work, or for a lead, their team's. */
	function doing(actorId: string): string {
		if (!owner) return '';
		const work = (attention.view?.workGraph?.work ?? []).filter(
			(item) => item.status === 'active' || item.status === 'blocked'
		);
		const team = teams.find((candidate) => candidate.lead_actor_id === actorId);
		const members = new Set(
			team
				? people.filter((person) => person.team_id === team.id).map((person) => person.actor_id)
				: [actorId]
		);
		const current = work
			.filter((item) => members.has(item.owner_id))
			.toSorted((a, b) => Date.parse(b.updated_at) - Date.parse(a.updated_at));
		if (!current.length) return '';
		return current.length > 1 ? `${current[0].title} · +${current.length - 1}` : current[0].title;
	}
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
		const latest = rows[0]?.person ?? directoryExec?.actor_id;
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

	/* Every conversation: direct ones by their last message, group rooms by when they began
	 * (rooms carry no last-message time). Only direct conversations can say what is unread. */
	const chats = $derived.by(() => {
		void seenVersion;
		const query = search.trim().toLocaleLowerCase();
		const direct = recent.conversations.flatMap((conversation) => {
			const person = people.find(
				(candidate) => candidate.actor_id === conversation.person_actor_id
			);
			if (!person) return [];
			const contact = owner && contacts.some((candidate) => candidate.actor_id === person.actor_id);
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
					unread:
						conversation.last_message_id > seenThrough(companyId, conversation.room_id) &&
						personId !== person.actor_id
				}
			];
		});
		const groups = rooms.rooms
			.filter((room) => room.kind !== 'direct' && !room.archived_at)
			.map((room) => ({
				key: `room:${room.id}`,
				name: room.title,
				mark: '#',
				group: true,
				person: '',
				room: room.id,
				roomId: room.id,
				at: room.created_at,
				unread: false
			}));
		return [...direct, ...groups]
			.filter((chat) => !query || chat.name.toLocaleLowerCase().includes(query))
			.toSorted((a, b) => Date.parse(b.at) - Date.parse(a.at));
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
			!directoryHumans.length
	);
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
		<h1 class="sr-only">People</h1>
		<div class="people-top">
			<label class="search"
				><Search size={15} /><input
					aria-label="Search conversations and people"
					type="search"
					placeholder="Search"
					bind:value={search}
				/></label
			>
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
			{#snippet personStatuses(actorId: string, name: string)}
				{@const status = personStatus(actorId)}
				{#if status.working || status.need}<span class="row-statuses"
						>{#if status.working}<span
								class="row-status working"
								title={`${name} is working`}
								aria-label={`${name} is working`}
								><LoaderCircle size={14} aria-hidden="true" /></span
							>{/if}{#if status.need === 'reply'}<span
								class="row-status reply"
								title={`${name} is waiting for your reply`}
								aria-label={`${name} is waiting for your reply`}
								><MessageCircleQuestion size={14} aria-hidden="true" /></span
							>{:else if status.need === 'attention'}<span
								class="row-status attention"
								title={`${name} needs your attention`}
								aria-label={`${name} needs your attention`}
								><i class="attention-dot" aria-hidden="true"></i></span
							>{/if}</span
					>{/if}
			{/snippet}
			{#snippet directoryPerson(person: DirectoryPerson, kind: string)}
				<a
					class="directory-person {kind}"
					href={href(person.actor_id)}
					onclick={(event) => void openPerson(event, person.actor_id)}
					aria-current={(!roomId && personId === person.actor_id) ||
					selectedDirectActorId === person.actor_id
						? 'page'
						: undefined}
					class:has-doing={!!doing(person.actor_id)}
				>
					<span class="avatar">{initials(person.display)}</span>
					<span class="directory-person-copy"
						><span class="person-lines"
							><span class="name">{personName(person.display)}</span
							>{#if doing(person.actor_id)}<small class="doing" title={doing(person.actor_id)}
									>{doing(person.actor_id)}</small
								>{/if}</span
						>{#if hasNew(person.actor_id) && personId !== person.actor_id}<span
								class="new-dot"
								title="Something new since you last looked"
								aria-label="New messages"
							></span>{/if}{@render personStatuses(
							person.actor_id,
							personName(person.display)
						)}</span
					>
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
						<span class="directory-person-copy"
							><span class="name">{chat.name}</span>{#if chat.unread}<span
									class="new-dot"
									title="New since you last looked"
									aria-label="Unread"
								></span>{/if}<time class="chat-time" datetime={chat.at}>{chatTime(chat.at)}</time
							></span
						>
					</a>
				{:else}
					<p class="empty">{search.trim() ? 'No conversations match.' : 'No conversations yet.'}</p>
				{/each}
				{#if !search.trim()}
					<div class="people-steps">
						<SidebarRow
							label="New group chat"
							icon={MessagesSquare}
							onclick={() => manager?.open()}
						/>
					</div>
				{/if}
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
							<span>{teamName(entry.team.name, companyId)}</span>
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
							<span>Unassigned</span>
						</header>
						{#each directoryUnassigned as member (member.actor_id)}
							{@render directoryPerson(member, 'member')}
						{/each}
					</section>
				{/if}
				{#if directoryHumans.length}
					<section class="directory-section" aria-label="Human colleagues">
						<header class="team-directory-head"><span>Colleagues</span></header>
						{#each directoryHumans as colleague (colleague.actor_id)}
							{@render directoryPerson(colleague, 'colleague')}
						{/each}
					</section>
				{/if}
				{#if !directoryExec && !directoryTeams.length && !directoryUnassigned.length && !directoryHumans.length}<p
						class="empty"
					>
						No people match.
					</p>{/if}
				{#if sparse}
					<div class="people-steps">
						{#if owner}<SidebarRow
								label="Invite a colleague"
								icon={UserPlus}
								href={`/${encodeURIComponent(companyId)}/company/members`}
							/>{/if}
						<SidebarRow
							label="New group chat"
							icon={MessagesSquare}
							onclick={() => manager?.open()}
						/>
					</div>
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
		grid-template-columns: var(--sidebar-width, 220px) minmax(0, 1fr);
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
	/* Search is the first row, with a quiet "+" beside it whose name lives in its tooltip. */
	.people-top {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 6px 6px 0;
	}
	.people-top :global(.room-manage-trigger) {
		width: 28px;
		height: 28px;
		padding: 0;
		justify-content: center;
	}
	.people-top :global(.room-manage-trigger span) {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
	.search {
		display: flex;
		flex: 1 1 auto;
		gap: 8px;
		align-items: center;
		min-width: 0;
		height: 28px;
		padding: 0 8px;
		border-radius: var(--radius-control);
		color: var(--text-tertiary);
	}
	.search:hover,
	.search:focus-within {
		background: var(--wash-hover);
	}
	.search input {
		width: 100%;
		min-width: 0;
		background: transparent;
		border: 0;
		font: inherit;
		outline: none;
		color: var(--ink);
	}
	/* A switch between two modes of the list, not an item in it: a two-part toggle, so it never
	 * reads as a second selection beside the open conversation. */
	.people-views {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 2px;
		margin: 6px 6px 4px;
		padding: 2px;
		border-radius: var(--radius-control);
		background: var(--wash-hover);
	}
	.people-views button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		height: 24px;
		padding: 0 8px;
		border: 0;
		border-radius: calc(var(--radius-control) - 2px);
		background: transparent;
		color: var(--text-tertiary);
		font: inherit;
		font-size: var(--t-body);
		cursor: pointer;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.people-views button:hover {
		color: var(--ink);
	}
	.people-views button[aria-selected='true'] {
		background: var(--surface-pane);
		box-shadow: var(--shadow-soft);
		color: var(--ink);
		font-weight: 500;
	}
	.people-views button:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	.people-views b {
		min-width: 16px;
		padding: 0 4px;
		border-radius: 8px;
		background: color-mix(in srgb, var(--surface-attention) 22%, transparent);
		color: var(--ink);
		font-size: var(--t-label);
		font-weight: 500;
		line-height: 16px;
	}
	.people-steps {
		display: grid;
		gap: 1px;
		margin-top: 14px;
	}
	.entries {
		overflow-y: auto;
		overflow-x: hidden;
		min-height: 0;
		padding: 2px 6px 8px;
	}
	/* The same grammar as every left sidebar (SidebarGroup, SidebarRow): small muted section
	 * labels, every row's mark in one column, spacing between sections rather than rules. */
	.directory-section {
		display: grid;
		gap: 1px;
	}
	.directory-section + .directory-section {
		margin-top: 14px;
	}
	.team-directory-head {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 26px;
		padding: 0 4px 0 8px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
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
	.directory-person {
		display: flex;
		align-items: center;
		box-sizing: border-box;
		height: 28px;
		gap: 8px;
		padding: 0 8px;
		overflow: hidden;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		text-decoration: none;
		transition:
			background-color var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.directory-person:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.directory-person[aria-current='page'] {
		background: var(--wash-press);
		color: var(--ink);
	}
	.directory-person[aria-current='page'] .name {
		font-weight: 500;
	}
	.directory-person:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	/* A person's live work is the point of this list, so it stays visible: their row takes a
	 * second line rather than hiding it in a tooltip. */
	.directory-person.has-doing {
		height: 40px;
	}
	.person-lines {
		display: grid;
		min-width: 0;
		flex: 1;
	}
	.doing {
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		line-height: 1.35;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.chat-time {
		flex: none;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
	}
	.directory-person.group .avatar {
		color: var(--text-tertiary);
	}
	/* The avatar sits in the icon column every sidebar row shares. */
	.directory-person .avatar {
		width: 18px;
		height: 18px;
		border-radius: 5px;
		font-size: 9px;
		font-weight: 600;
	}
	.directory-person.executive .avatar,
	.directory-person.lead .avatar {
		border-color: color-mix(in srgb, var(--intent-conversation) 26%, var(--border));
		background: var(--intent-conversation-soft);
		color: var(--intent-conversation);
	}
	.directory-person-copy {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 4px;
		min-width: 0;
		flex: 1;
		overflow: hidden;
	}
	.new-dot {
		width: 6px;
		height: 6px;
		flex: none;
		border-radius: 999px;
		background: var(--intent-conversation);
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
		min-width: 4ch;
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
		gap: 2px;
		flex: none;
	}
	.row-status {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 18px;
		height: 18px;
		padding: 0;
		border-radius: 999px;
		line-height: 1;
	}
	.row-status.working {
		color: var(--intent-conversation);
		background: var(--intent-conversation-soft);
	}
	.row-status.working :global(svg) {
		animation: status-spin 1.1s linear infinite;
	}
	.row-status.reply {
		color: var(--intent-authority);
		background: color-mix(in srgb, var(--intent-authority) 12%, transparent);
	}
	.attention-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--intent-authority);
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
		padding: 12px;
		font-size: var(--t-label);
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
