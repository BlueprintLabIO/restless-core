<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import FileText from '@lucide/svelte/icons/file-text';
	import ConversationDocument from '$lib/components/ConversationDocument.svelte';
	import Search from '@lucide/svelte/icons/search';
	import Users from '@lucide/svelte/icons/users';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Pin from '@lucide/svelte/icons/pin';
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
		roomTitleSearchQuery,
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
	const teams = $derived((owner ? cockpit.view?.teams : collaboration.view?.teams) ?? []);
	const contacts = $derived(
		people.filter((p) => p.kind === 'exec' || teams.some((t) => t.lead_actor_id === p.actor_id))
	);
	const rooms = $derived(roomsQuery(companyId));
	const roomId = $derived(page.url.searchParams.get('room') ?? '');
	const personId = $derived(
		page.url.searchParams.get('person') ??
			(roomId ? '' : (contacts.find((p) => p.kind === 'exec')?.actor_id ?? ''))
	);
	const selectedPerson = $derived(people.find((p) => p.actor_id === personId));
	const documentOpen = $derived(page.url.searchParams.has('document'));
	const linkedRoomId = $derived(
		roomId || rooms.rooms.find((r) => directPerson(r)?.actor_id === personId)?.id || ''
	);
	function toggleDocument() {
		const url = new URL(page.url);
		if (documentOpen) url.searchParams.delete('document');
		else url.searchParams.set('document', '');
		void goto(url, { noScroll: true, keepFocus: true });
	}
	const explicitSelection = $derived(!!roomId || page.url.searchParams.has('person'));
	let directory = $state(false);
	let search = $state('');
	let debouncedSearch = $state('');
	$effect(() => {
		const value = search.trim();
		const timer = setTimeout(() => (debouncedSearch = value), 200);
		return () => clearTimeout(timer);
	});
	const titleSearch = $derived(roomTitleSearchQuery(companyId, debouncedSearch));
	const messageSearch = $derived(roomMessageSearchQuery(companyId, debouncedSearch));
	let pinned = $state<string[]>([]);
	const pinKey = $derived(
		`restless:conversation-pins:${companyId}:${principal.view?.actor_id ?? ''}`
	);
	$effect(() => {
		const key = pinKey;
		if (!browser) return;
		try {
			const value = JSON.parse(localStorage.getItem(key) ?? '[]');
			pinned = Array.isArray(value) ? value.filter((v) => typeof v === 'string') : [];
		} catch {
			pinned = [];
		}
	});
	function togglePin(key: string) {
		pinned = pinned.includes(key) ? pinned.filter((p) => p !== key) : [...pinned, key];
		try {
			localStorage.setItem(pinKey, JSON.stringify(pinned));
		} catch {
			/* In-memory pins still work. */
		}
	}
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
	let openingPerson = $state('');
	let openingError = $state('');
	const pendingDirect = new Map<string, string>();
	async function openPerson(event: MouseEvent | null, id: string) {
		const person = people.find((p) => p.actor_id === id);
		if (owner && person?.kind !== 'human') return;
		event?.preventDefault();
		if (openingPerson || !principal.view) return;
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
		const entries = (directory ? people : owner ? contacts : [])
			.filter((p) => !query || `${p.display} ${p.role}`.toLocaleLowerCase().includes(query))
			.map((p) => ({
				key: `person:${p.actor_id}`,
				name: p.display,
				person: p.actor_id,
				room: '',
				group: false,
				hint: teams.find((t) => t.id === p.team_id)?.name ?? p.role
			}));
		if (!directory)
			for (const room of query ? titleSearch.rooms : rooms.rooms) {
				if (owner && contacts.some((p) => p.actor_id === directPerson(room)?.actor_id)) continue;
				entries.push({
					key: `room:${room.id}`,
					name: directPerson(room)?.display ?? room.title,
					person: '',
					room: room.id,
					group: room.kind !== 'direct',
					hint: room.kind === 'direct' ? 'Direct conversation' : 'Group conversation'
				});
			}
		return entries.sort((a, b) => Number(pinned.includes(b.key)) - Number(pinned.includes(a.key)));
	});
	function href(person: string, room = '') {
		const params = new URLSearchParams(room ? { room } : { person });
		const document = page.url.searchParams.get('document');
		if (document !== null) params.set('document', document);
		return `/${encodeURIComponent(companyId)}/people?${params}`;
	}
	function created(room: Room) {
		void goto(href('', room.id));
	}
	function needsYou(person: string) {
		return (
			owner &&
			attention.view?.items.some(
				(item) => item.responsibleActor?.id === person || item.briefAuthor?.id === person
			)
		);
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
			<button class:active={!directory} onclick={() => (directory = false)}>Conversations</button
			><button class:active={directory} onclick={() => (directory = true)}>People</button>
		</nav>
		<div class="entries" aria-busy={!!openingPerson}>
			{#if openingError}<p class="empty" role="alert">{openingError}</p>{/if}
			{#each rows as row (row.key)}
				<div
					class="entry"
					class:current={row.room ? roomId === row.room : !roomId && personId === row.person}
				>
					<a
						onclick={(event) => {
							if (row.person) void openPerson(event, row.person);
						}}
						href={href(row.person, row.room)}
						aria-current={(row.room ? roomId === row.room : !roomId && personId === row.person)
							? 'page'
							: undefined}
						title={row.hint}
					>
						<span class="avatar"
							>{#if row.group}<Users size={17} />{:else}{row.name
									.split(/\s+/)
									.slice(0, 2)
									.map((s) => s[0])
									.join('')}{/if}</span
						><span class="name">{row.name}</span>{#if needsYou(row.person)}<span
								class="needs"
								title="Needs you"
								aria-label="Needs you"
							></span>{/if}
					</a>
					<button
						class="pin"
						class:pinned={pinned.includes(row.key)}
						aria-label={`${pinned.includes(row.key) ? 'Unpin' : 'Pin'} ${row.name}`}
						title={pinned.includes(row.key) ? 'Unpin' : 'Pin'}
						onclick={() => togglePin(row.key)}><Pin size={13} /></button
					>
				</div>
			{:else}<p class="empty">
					{rooms.status === 'unknown' ? 'Loading conversations…' : 'No matches.'}
				</p>{/each}
			{#if search.trim() && !directory}
				{#each messageSearch.messages as message (message.id)}
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
				{#if titleSearch.hasMore}<button class="more" onclick={() => titleSearch.loadMore()}
						>More conversations</button
					>{/if}
			{:else if rooms.hasMore && !directory}<button class="more" onclick={() => rooms.loadMore()}
					>More conversations</button
				>{/if}
			{#if rooms.failure || messageSearch.failure || titleSearch.failure}<p
					class="empty"
					role="status"
				>
					{rooms.failure?.message ?? messageSearch.failure?.message ?? titleSearch.failure?.message}
				</p>{/if}
		</div>
	</aside>
	<section class="conversation-main" aria-label="Selected conversation">
		<a class="back" href={`/${companyId}/people`}><ArrowLeft size={16} /> Conversations</a>
		{#if roomId}<RoomConversation
				>{#snippet actions()}<button
						class="document-toggle"
						title="Open a document alongside this conversation"
						aria-label="Open document"
						onclick={toggleDocument}><FileText size={17} /></button
					>{/snippet}</RoomConversation
			>{:else if owner && selectedPerson?.kind !== 'human'}<PersonConversation
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
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--t-body);
	}
	.current .name {
		font-weight: 600;
	}
	.pin {
		padding: 8px;
		display: grid;
		place-items: center;
		border: 0;
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
		opacity: 0.35;
	}
	.pin.pinned,
	.entry:hover .pin,
	.pin:focus-visible {
		opacity: 1;
		color: var(--intent-conversation);
	}
	.needs {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--intent-authority);
		flex: none;
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
