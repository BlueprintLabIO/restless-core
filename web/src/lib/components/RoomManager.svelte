<script lang="ts">
	import Plus from '@lucide/svelte/icons/plus';
	import Users from '@lucide/svelte/icons/users';
	import X from '@lucide/svelte/icons/x';
	import { useQueryClient } from '@tanstack/svelte-query';
	import {
		createRoom,
		readRoomDraft,
		writeRoomDraft,
		roomDraftKey,
		addRoomParticipant,
		removeRoomParticipant,
		type Room,
		type RoomParticipant
	} from '$lib/model/rooms';
	import { roomQueryKeys } from '$lib/model/room-queries.svelte';
	let {
		companyId,
		actorId,
		people,
		room = null,
		participants = [],
		oncreated = () => {},
		onperson,
		contactIds = [],
		initialParticipants = [],
		initialContext = '',
		label
	}: {
		companyId: string;
		actorId: string;
		people: { actor_id: string; display: string }[];
		room?: Room | null;
		participants?: RoomParticipant[];
		oncreated?: (room: Room) => void;
		onperson?: (id: string) => void;
		contactIds?: string[];
		initialParticipants?: string[];
		initialContext?: string;
		label?: string;
	} = $props();
	const dialogId = $props.id();
	const client = useQueryClient();
	let dialog: HTMLDialogElement;
	let title = $state('');
	let context = $state('');
	let selected = $state<string[]>([]);
	let busyFor = $state('');
	let failure = $state('');
	const commandsByIntent = new Map<string, string>();
	const scopeKey = $derived(`${companyId}\u0000${actorId}\u0000${room?.id ?? 'new'}`);
	const busy = $derived(busyFor !== '' && busyFor === scopeKey);
	const canManage = $derived(
		room?.kind !== 'direct' &&
			participants.some((p) => p.actor_id === actorId && p.role === 'owner' && !p.left_at)
	);
	const active = $derived(participants.filter((p) => !p.left_at));
	const initialParticipantSet = $derived(new Set(initialParticipants));
	const availablePeople = $derived.by(() => {
		const candidates = new Map(
			people
				.filter((person) => person.actor_id !== actorId)
				.map((person) => [person.actor_id, person])
		);
		for (const participantId of initialParticipants) {
			if (participantId !== actorId && !candidates.has(participantId)) {
				candidates.set(participantId, { actor_id: participantId, display: participantId });
			}
		}
		return [...candidates.values()];
	});
	function uniqueParticipantIds(ids: string[]): string[] {
		const available = new Set(availablePeople.map((person) => person.actor_id));
		return [...new Set(ids)].filter((id) => available.has(id));
	}
	function displayFor(actorId: string): string {
		return people.find((person) => person.actor_id === actorId)?.display ?? actorId;
	}
	function commandFor(intent: string): string {
		const existing = commandsByIntent.get(intent);
		if (existing) return existing;
		const id = crypto.randomUUID();
		commandsByIntent.set(intent, id);
		return id;
	}
	function isCurrentScope(scope: string): boolean {
		return scopeKey === scope;
	}
	function open() {
		if (busy || dialog.open) return;
		failure = '';
		if (!room) {
			title = '';
			selected = uniqueParticipantIds(initialParticipants);
			context = initialContext;
		}
		dialog.showModal();
	}
	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		const requestedCompanyId = companyId;
		const requestedActorId = actorId;
		const requestedScope = scopeKey;
		const requestedParticipants = uniqueParticipantIds(selected);
		if (!requestedCompanyId || !requestedActorId || !requestedParticipants.length) return;
		const addingPeople = initialParticipants.length > 0;
		const includesAdditionalPerson = requestedParticipants.some(
			(id) => !initialParticipantSet.has(id)
		);
		if (addingPeople && !includesAdditionalPerson) {
			failure = 'Choose at least one more person for the new group.';
			return;
		}
		const directContact =
			!addingPeople &&
			requestedParticipants.length === 1 &&
			Boolean(onperson) &&
			contactIds.includes(requestedParticipants[0]);
		if (directContact) {
			dialog.close();
			onperson?.(requestedParticipants[0]);
			return;
		}
		const kind = addingPeople || requestedParticipants.length > 1 ? 'group' : 'direct';
		const name = title.trim() || requestedParticipants.map(displayFor).join(', ').slice(0, 160);
		const intent = JSON.stringify({
			companyId: requestedCompanyId,
			actorId: requestedActorId,
			kind,
			title: name,
			participants: [...requestedParticipants].sort()
		});
		const command = commandFor(intent);
		const contextToCarry = context.trim();
		const createdCallback = oncreated;
		busyFor = requestedScope;
		failure = '';
		try {
			const created = await createRoom(requestedCompanyId, {
				kind,
				title: name,
				participant_actor_ids: requestedParticipants,
				command_id: command
			});
			if (contextToCarry) {
				const key = roomDraftKey(requestedCompanyId, requestedActorId, created.id, null);
				if (!readRoomDraft(key).body) {
					writeRoomDraft(key, {
						body: contextToCarry,
						commandId: null,
						updatedAt: new Date().toISOString()
					});
				}
			}
			await client.invalidateQueries({ queryKey: roomQueryKeys.list(requestedCompanyId) });
			if (!isCurrentScope(requestedScope) || actorId !== requestedActorId) return;
			dialog.close();
			title = '';
			context = '';
			selected = [];
			createdCallback(created);
		} catch (error) {
			if (isCurrentScope(requestedScope) && actorId === requestedActorId) {
				failure = error instanceof Error ? error.message : 'Conversation could not be created.';
			}
		} finally {
			if (busyFor === requestedScope) busyFor = '';
		}
	}
	async function membership(person: string, add: boolean) {
		if (!room || busy) return;
		const requestedCompanyId = companyId;
		const requestedActorId = actorId;
		const requestedRoomId = room.id;
		const requestedScope = scopeKey;
		busyFor = requestedScope;
		failure = '';
		try {
			if (add) await addRoomParticipant(requestedCompanyId, requestedRoomId, person);
			else await removeRoomParticipant(requestedCompanyId, requestedRoomId, person);
			await client.invalidateQueries({
				queryKey: roomQueryKeys.participants(requestedCompanyId, requestedRoomId)
			});
		} catch (error) {
			if (
				isCurrentScope(requestedScope) &&
				actorId === requestedActorId &&
				room?.id === requestedRoomId
			) {
				failure = error instanceof Error ? error.message : 'Membership could not be updated.';
			}
		} finally {
			if (busyFor === requestedScope) busyFor = '';
		}
	}
</script>

<button
	class="btn small room-manage-trigger"
	type="button"
	onclick={open}
	title={label ?? (room ? 'Participants' : 'New conversation')}
	aria-label={label ?? (room ? 'Participants' : 'New conversation')}
>
	{#if room || initialParticipants.length}<Users size={15} aria-hidden="true" />{:else}<Plus
			size={16}
			aria-hidden="true"
		/>{/if}<span>{label ?? (room ? 'Participants' : 'New conversation')}</span>
</button>
<dialog
	bind:this={dialog}
	aria-labelledby={`${dialogId}-${room ? 'members' : 'create'}-title`}
	oncancel={(event) => {
		if (busy) event.preventDefault();
	}}
>
	<header>
		<h2 id={`${dialogId}-${room ? 'members' : 'create'}-title`}>
			{room ? 'Participants' : initialParticipants.length ? 'Add people' : 'New conversation'}
		</h2>
		<button type="button" aria-label="Close" disabled={busy} onclick={() => dialog.close()}
			><X size={17} aria-hidden="true" /></button
		>
	</header>
	{#if room}
		<div class="member-list">
			{#each active as member (member.actor_id)}
				<div class="member">
					<span>{displayFor(member.actor_id)}{member.actor_id === actorId ? ' (you)' : ''}</span>
					{#if member.role === 'owner'}<span class="muted">Owner</span>{:else if canManage}<button
							type="button"
							disabled={busy}
							onclick={() => membership(member.actor_id, false)}
							aria-label={`Remove ${people.find((p) => p.actor_id === member.actor_id)?.display ?? member.actor_id}`}
							>Remove</button
						>{/if}
				</div>
			{/each}
		</div>
		{#if canManage}
			<p class="history-note" id={`${dialogId}-history-note`}>
				People added here can read the conversation that is already in this room.
			</p>
			<label
				>Add a member<select
					aria-label="Add a member"
					aria-describedby={`${dialogId}-history-note`}
					disabled={busy}
					value=""
					onchange={(event) => {
						const value = event.currentTarget.value;
						event.currentTarget.value = '';
						if (value) void membership(value, true);
					}}
					><option value="">Choose a person…</option
					>{#each availablePeople.filter((p) => !active.some((m) => m.actor_id === p.actor_id)) as person}<option
							value={person.actor_id}>{person.display}</option
						>{/each}</select
				></label
			>
		{/if}
	{:else}
		<form onsubmit={create}>
			{#if initialParticipants.length}<p class="history-note">
					This starts a new group. Earlier messages stay in the original conversation.
				</p>{/if}
			<label
				>Conversation name (optional)<input
					bind:value={title}
					maxlength={160}
					autocomplete="off"
					disabled={busy}
				/></label
			>
			<fieldset disabled={busy}>
				<legend>Members</legend>
				<div class="member-list">
					{#each availablePeople as person}<label class="choice"
							><input
								type="checkbox"
								disabled={initialParticipantSet.has(person.actor_id)}
								bind:group={selected}
								value={person.actor_id}
							/>{person.display}</label
						>{/each}
				</div>
			</fieldset>
			{#if initialParticipants.length}<label
					>Context to carry forward (optional)<textarea
						bind:value={context}
						rows="5"
						disabled={busy}
						placeholder="Add context for the group…"></textarea></label
				>
				<p class="muted">
					Review this excerpt before starting. It opens as a draft; send it when you are ready.
				</p>{/if}
			<button
				class="create"
				type="submit"
				disabled={busy ||
					!selected.length ||
					(!!initialParticipants.length && selected.every((id) => initialParticipantSet.has(id)))}
				>{busy
					? 'Opening…'
					: initialParticipants.length
						? 'Start group conversation'
						: 'Start conversation'}</button
			>
		</form>
	{/if}
	{#if failure}<p class="failure" role="alert">{failure}</p>{/if}
</dialog>

<style>
	textarea {
		width: 100%;
		box-sizing: border-box;
		resize: vertical;
		padding: 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--ink);
		font: inherit;
		font-size: var(--t-body);
	}
	.room-manage-trigger {
		gap: 5px;
	}
	dialog {
		width: min(420px, calc(100vw - 32px));
		max-height: calc(100dvh - 48px);
		overflow: auto;
		padding: 20px;
		border: 1px solid var(--border);
		border-radius: 10px;
		background: var(--surface);
		color: var(--ink);
	}
	dialog::backdrop {
		background: color-mix(in srgb, var(--ink) 28%, transparent);
	}
	header,
	.member {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 12px;
	}
	header {
		margin-bottom: 18px;
	}
	h2 {
		margin: 0;
		font-size: var(--t-head);
	}
	button,
	input,
	select {
		font: inherit;
	}
	header button {
		background: none;
		border: 0;
		color: inherit;
	}
	label {
		display: grid;
		gap: 8px;
		margin-block: 12px;
		font-size: var(--t-body);
	}
	input:not([type='checkbox']),
	select {
		width: 100%;
		min-width: 0;
		padding: 8px;
		border: 1px solid var(--border);
		border-radius: 5px;
		background: var(--surface);
		color: inherit;
	}
	fieldset {
		min-width: 0;
		padding: 0;
		border: 0;
	}
	legend {
		font-size: var(--t-body);
	}
	.member-list {
		max-height: 260px;
		overflow: auto;
	}
	.choice {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.member {
		min-height: 38px;
		font-size: var(--t-body);
	}
	.member span:first-child {
		overflow-wrap: anywhere;
	}
	.member button,
	.create {
		border: 1px solid var(--border);
		border-radius: 5px;
		padding: 6px 10px;
		background: var(--surface);
		color: inherit;
		cursor: pointer;
	}
	.create {
		display: block;
		margin-left: auto;
		margin-top: 16px;
	}
	.muted {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.history-note {
		margin: 12px 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.45;
	}
	.failure {
		color: var(--intent-danger, #a22);
		font-size: var(--t-body);
		overflow-wrap: anywhere;
	}
	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	button:focus-visible,
	select:focus-visible,
	input:focus-visible,
	textarea:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
</style>
