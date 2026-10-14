<script lang="ts">
	import {
		DIRECTORY_FILTERS,
		directoryFilter,
		statusWord,
		type DirectoryFilter,
		type StatusWord
	} from '$lib/model/people-directory';
	import { listArchivedAgents, type ArchivedAgent } from '$lib/model/agent-staffing';
	import WorkDetail from '$lib/components/WorkDetail.svelte';
	import { resizePane } from '$lib/actions/resize-pane';
	import { setTeamOwner as recordTeamOwner, disbandTeam } from '$lib/model/company-actions';
	import TeamEditor from '$lib/components/TeamEditor.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { initials, personName, teamName } from '$lib/model/initials';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import BellOff from '@lucide/svelte/icons/bell-off';
	import UserPlus from '@lucide/svelte/icons/user-plus';
	import SidebarSearch from '$lib/ui/views/SidebarSearch.svelte';
	import SidebarEmpty from '$lib/ui/views/SidebarEmpty.svelte';
	import type { SidebarTone } from '$lib/ui/views/SidebarRow.svelte';
	import {
		AlreadyMember,
		changeMembership,
		getCoreMembers,
		getIssuerMembers,
		inviteToCompany,
		IssuerSessionMissing,
		mayManage,
		type CoreMembersView,
		type IssuerInvitation,
		type IssuerMember,
		type IssuerMembersView
	} from '$lib/model/members';
	import { mayManageCompanySetup } from '$lib/model/company-access';
	import { parseInviteEmails } from '$lib/model/invite-emails';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import PersonProfile from '$lib/components/PersonProfile.svelte';
	import { askExec, staffDraft, type StaffAsk } from '$lib/model/ask-exec';
	import { projectOfficeMembers } from '$lib/office/projection';
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
		archivedRoomsQuery,
		roomQueryKeys,
		roomsQuery,
		recentDirectConversationsQuery,
		recentGroupConversationsQuery,
		roomMessageSearchQuery,
		roomTitleSearchQuery,
		muteRoom
	} from '$lib/model/room-queries.svelte';
	import { conversationUnread, messagePreview, restoreRoom, type Room } from '$lib/model/rooms';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { presentNow } from '$lib/model/presence.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { getContext, type Snippet } from 'svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

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
	/* People own teams; agents run them. The owner says who, here or by telling Exec. */
	function teamOwnerId(team: (typeof teams)[number]): string | null {
		return ('owner_person' in team ? team.owner_person : team.responsible_actor_id) ?? null;
	}
	const everyone = $derived((owner ? cockpit.view?.people : collaboration.view?.people) ?? []);
	const humanColleagues = $derived(everyone.filter((person) => person.kind === 'human'));
	/* Colleagues with the cockpit open right now, from the shell's presence heartbeat. */
	const hereNow = $derived(new Set(presentNow.people.map((person) => person.actor_id)));
	function teamOwnerName(id: string): string {
		if (id === principal.view?.actor_id) return 'you';
		return personName(everyone.find((person) => person.actor_id === id)?.display ?? 'a colleague');
	}
	/* Changing who owns a team acts at once and can be undone; the line under the team header
	 * shows only what the server holds, so a refused change never looks made. */
	let teamOwnerBusy = $state('');
	async function setTeamOwner(team: (typeof teams)[number], person: string, undoing = false) {
		const previous = teamOwnerId(team) ?? '';
		if (person === previous || teamOwnerBusy) return;
		const label = teamName(team.name, companyId);
		teamOwnerBusy = team.id;
		try {
			await recordTeamOwner(
				companyId,
				team.id,
				person || null,
				person ? 'Appointed by the owner from People' : 'Handed back to the owner from People'
			);
			await cockpit.refresh();
			toast(
				person ? `${label} is now owned by ${teamOwnerName(person)}.` : `You own ${label} again.`,
				undoing ? {} : { undo: () => void setTeamOwner(team, previous, true) }
			);
		} catch (cause) {
			toast(failureSentence(cause, `Who owns ${label} did not change. Try again.`), {
				tone: 'failure'
			});
			void cockpit.refresh();
		} finally {
			teamOwnerBusy = '';
		}
	}
	/* The owner reshapes the organisation directly: one editor open at a time, under the team it
	 * changes or, for a new team, at the foot of the directory. Disbanding asks first. */
	let teamEdit = $state<{ mode: 'new' | 'rename' | 'lead'; team: string } | null>(null);
	const agents = $derived(everyone.filter((person) => person.kind === 'staff'));
	async function teamEdited(message: string) {
		teamEdit = null;
		await cockpit.refresh();
		toast(message);
	}
	async function disband(team: (typeof teams)[number]) {
		const label = teamName(team.name, companyId);
		const yes = await confirmAction({
			title: `Disband ${label}?`,
			body: 'Its members become unassigned, and anything it still owed you falls to Exec.',
			action: 'Disband',
			danger: true
		});
		if (!yes) return;
		try {
			await disbandTeam(companyId, team.id);
			await cockpit.refresh();
			toast(`${label} disbanded`);
		} catch (cause) {
			toast(failureSentence(cause, `${label} was not disbanded. Try again.`), { tone: 'failure' });
		}
	}
	const contacts = $derived(
		people.filter((p) => p.kind === 'exec' || teams.some((t) => t.lead_actor_id === p.actor_id))
	);
	const rooms = $derived(roomsQuery(companyId));
	/* Groups you archived wait under Chats until you restore them; nobody else sees them. */
	const archived = $derived(archivedRoomsQuery(companyId, () => !!principal.view?.actor_id));
	const roomClient = useQueryClient();
	let restoring = $state('');
	async function restore(room: Room) {
		if (restoring) return;
		restoring = room.id;
		try {
			await restoreRoom(companyId, room.id);
			await Promise.all([
				roomClient.invalidateQueries({ queryKey: roomQueryKeys.list(companyId) }),
				roomClient.invalidateQueries({ queryKey: roomQueryKeys.recentGroups(companyId) })
			]);
			toast(`${room.title} is back in Chats.`);
			void goto(href('', room.id));
		} catch (error) {
			toast(failureSentence(error, `${room.title} could not be restored.`), { tone: 'failure' });
		} finally {
			restoring = '';
		}
	}
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
	/* A piece of Work opens in a side sheet beside its agent's chat (S85-15). Opened without a
	 * chat named, the chat is the agent doing the Work, once the sheet knows who that is. */
	const workId = $derived(page.url.searchParams.get('work') ?? '');
	function openOwnerChat(ownerId: string) {
		if (roomId || page.url.searchParams.has('person') || !ownerId) return;
		const url = new URL(page.url);
		url.searchParams.set('person', ownerId);
		void goto(`${url.pathname}${url.search}`, {
			replaceState: true,
			keepFocus: true,
			noScroll: true
		});
	}
	function closeWork() {
		const url = new URL(page.url);
		url.searchParams.delete('work');
		void goto(`${url.pathname}${url.search}`, { keepFocus: true, noScroll: true });
	}
	let search = $state('');
	let debouncedSearch = $state('');
	$effect(() => {
		const value = search.trim();
		const timer = setTimeout(() => (debouncedSearch = value), 200);
		return () => clearTimeout(timer);
	});
	const messageSearch = $derived(roomMessageSearchQuery(companyId, debouncedSearch));
	/* Chats loads group rooms a page at a time; a search also asks the server, so a group on a
	 * page not loaded yet is still found. */
	const roomTitleSearch = $derived(roomTitleSearchQuery(companyId, debouncedSearch));
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
	/* The office floor's reading of each agent: presence, current Work and step. People shows the
	 * same, so the two never disagree about what someone is doing. */
	const officeMembers = $derived(
		owner && cockpit.view && attention.view?.workGraph
			? projectOfficeMembers(companyId, cockpit.view, attention.view.workGraph)
			: []
	);
	function officeMember(actorId: string) {
		return officeMembers.find((member) => member.actorId === actorId) ?? null;
	}
	/* Each person's second line: what they need from the owner, what is stuck, or what they are
	 * producing right now (their own Work, or for a lead, their team's), coloured by which. Someone
	 * with none of these shows their role; a lead shows what its team is for. */
	function personLine(actorId: string): { text: string; tone: SidebarTone; hint?: string } {
		const person = people.find((candidate) => candidate.actor_id === actorId);
		const ledTeam = teams.find((candidate) => candidate.lead_actor_id === actorId);
		const fallback = {
			text:
				person?.kind === 'exec'
					? 'Ready when you are'
					: person?.kind === 'human'
						? hereNow.has(actorId)
							? 'Here now'
							: 'Colleague'
						: ledTeam
							? ledTeam.brief.trim() || `Leads ${teamName(ledTeam.name, companyId)}`
							: (person?.role ?? ''),
			tone: '' as SidebarTone
		};
		if (!owner) return fallback;
		const status = personStatus(actorId);
		if (status.need === 'reply') return { text: 'Waiting for your reply', tone: 'wait' };
		if (status.need === 'attention') return { text: 'Needs your attention', tone: 'wait' };
		const member = ledTeam ? null : officeMember(actorId);
		if (member) {
			const hint = [member.presenceLabel, member.work?.title, member.currentStep]
				.filter(Boolean)
				.join(' · ');
			if (member.presence === 'waiting' && member.work)
				return { text: `Blocked: ${member.work.title}`, tone: 'block', hint };
			if ((member.presence === 'observed' || member.presence === 'in-motion') && member.work)
				return { text: member.work.title, tone: 'work', hint };
			if (member.presence === 'observed') return { text: 'Working', tone: 'work', hint };
			return { ...fallback, hint };
		}
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

	/* Who is in the company and who is invited, from the account plane that issues access. Owners
	 * and admins manage both here, with the same rules as Company › Members (members.ts). A local
	 * company has no issuer; without an account session there is nothing to show, not an error. */
	const canManageMembers = $derived(mayManageCompanySetup(principal.view));
	let coreMembers = $state<CoreMembersView | null>(null);
	let issuerMembers = $state<IssuerMembersView | null>(null);
	let membersFor = '';
	async function loadMembers(target = companyId) {
		try {
			const core = await getCoreMembers(target);
			const issuer =
				core.mode === 'network' && core.issuer && core.company_id
					? await getIssuerMembers(core.issuer, core.company_id).catch(() => null)
					: null;
			if (target !== companyId) return;
			coreMembers = core;
			issuerMembers = issuer;
		} catch {
			if (target === companyId) issuerMembers = null;
		}
	}
	$effect(() => {
		const target = companyId;
		if (!canManageMembers || membersFor === target) return;
		membersFor = target;
		void loadMembers(target);
	});
	const invitations = $derived<IssuerInvitation[]>(issuerMembers?.invitations ?? []);
	function membershipFor(actorId: string): IssuerMember | null {
		const core = coreMembers?.members.find((member) => member.actor_id === actorId);
		if (!core || !issuerMembers) return null;
		const found = issuerMembers.members.find(
			(member) => member.membership_id === core.membership_id && member.status !== 'removed'
		);
		return found && mayManage(issuerMembers.viewer, found) ? found : null;
	}
	let memberBusy = $state(false);
	async function memberChange(
		rest: string,
		body: Record<string, unknown>,
		done: string,
		undo?: () => unknown
	): Promise<void> {
		if (!coreMembers?.issuer || !coreMembers.company_id || memberBusy) return;
		memberBusy = true;
		try {
			const result = await changeMembership<{ ending?: boolean }>(
				coreMembers.issuer,
				coreMembers.company_id,
				rest,
				body
			);
			toast(result.ending ? 'Ending access. The company confirms it shortly.' : done, { undo });
		} catch (cause) {
			toast(
				cause instanceof IssuerSessionMissing
					? 'Sign in to your account under Company › Members, then try again.'
					: failureSentence(cause, 'That change was not made.'),
				{ tone: 'failure' }
			);
		} finally {
			memberBusy = false;
			await loadMembers();
		}
	}
	function setRole(member: IssuerMember, role: 'admin' | 'member') {
		const path = `members/${member.membership_id}/role`;
		const said = (next: 'admin' | 'member') =>
			`${member.name} is ${next === 'admin' ? 'now an admin' : 'now a member'}.`;
		const previous = member.role === 'admin' ? 'admin' : 'member';
		void memberChange(path, { role }, said(role), () =>
			memberChange(path, { role: previous }, said(previous))
		);
	}
	async function removeMember(member: IssuerMember) {
		const yes = await confirmAction({
			title: `Remove ${member.name}’s access?`,
			body: 'They lose access at once. What they made stays with the company, and you can invite them again.',
			action: 'Remove access',
			danger: true
		});
		if (yes)
			void memberChange(
				`members/${member.membership_id}/remove`,
				{},
				`${member.name} no longer has access.`
			);
	}
	function mayChangeInvitation(invite: IssuerInvitation): boolean {
		return issuerMembers?.viewer.role === 'owner' || invite.role === 'member';
	}
	async function copyInvitation(invite: IssuerInvitation) {
		try {
			await navigator.clipboard.writeText(invite.link);
			toast(`Invitation link for ${invite.email} copied.`);
		} catch {
			toast('The link could not be copied. Open Company › Members to copy it.', {
				tone: 'failure'
			});
		}
	}
	async function cancelInvitation(invite: IssuerInvitation) {
		const yes = await confirmAction({
			title: `Cancel the invitation to ${invite.email}?`,
			body: 'The link they were sent stops working. You can invite them again later.',
			action: 'Cancel invitation',
			danger: true
		});
		if (yes) void memberChange(`invitations/${invite.id}/cancel`, {}, 'Invitation cancelled.');
	}
	/* Invite from People: one or several addresses, as Members does. Where the company is not shared
	 * yet, Invite goes to Members, which sets sharing up. */
	const invitesReady = $derived(coreMembers?.mode === 'network' && !!issuerMembers);
	let inviting = $state(false);
	let inviteEmails = $state('');
	let inviteBusy = $state(false);
	let inviteFailure = $state('');
	async function sendInvites(event: SubmitEvent) {
		event.preventDefault();
		if (inviteBusy) return;
		const { emails, invalid } = parseInviteEmails(inviteEmails);
		if (!emails.length) {
			inviteFailure = invalid.length ? `Not an email address: ${invalid.join(', ')}.` : '';
			return;
		}
		inviteBusy = true;
		inviteFailure = '';
		const kept: string[] = [];
		let sent = 0;
		for (const email of emails) {
			try {
				await inviteToCompany(companyId, email);
				sent += 1;
			} catch (cause) {
				if (cause instanceof AlreadyMember) toast(cause.message);
				else {
					kept.push(email);
					inviteFailure ||= failureSentence(cause, 'The invitation was not sent.');
				}
			}
		}
		inviteBusy = false;
		inviteEmails = [...kept, ...invalid].join(', ');
		if (invalid.length) inviteFailure ||= `Not an email address: ${invalid.join(', ')}.`;
		if (sent) toast(sent === 1 ? 'Invitation sent.' : `${sent} invitations sent.`);
		if (!kept.length && !invalid.length) inviting = false;
		await loadMembers();
	}
	/* The Exec hires, briefs, pauses and retires Staff; People hands it a draft to finish. */
	let profileId = $state('');
	function askAbout(ask: StaffAsk, person: DirectoryPerson) {
		const team = teams.find((candidate) => candidate.id === person.team_id);
		profileId = '';
		askExec(
			staffDraft(
				ask,
				{ display: personName(person.display), role: person.role },
				team ? teamName(team.name, companyId) : ''
			)
		);
	}
	/* Unread comes from the server read cursor, so it is the same on every device. The open
	 * conversation advances that cursor itself (RoomConversation, or the rail for the Exec and
	 * leads). */
	const viewerId = $derived(principal.view?.actor_id ?? '');
	function hasNew(actorId: string): boolean {
		const conversation = recent.conversations.find((entry) => entry.person_actor_id === actorId);
		return !!conversation && conversationUnread(conversation, viewerId);
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
	/* Names, roles and what each person is working on (S88-02). */
	function matchesDirectory(
		value: { display: string; role: string; actor_id?: string },
		query: string
	) {
		if (!query) return true;
		const work = value.actor_id ? currentWorkTitle(value.actor_id) : '';
		return `${value.display} ${value.role} ${work}`.toLocaleLowerCase().includes(query);
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
	/* One test for whether the Exec row shows, so "No people match" appears exactly when nothing
	 * does, including when the Exec exists but does not match the search. */
	const execShown = $derived(
		!!directoryExec && matchesDirectory(directoryExec, search.trim().toLocaleLowerCase())
	);
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
		/* The company's owner is a colleague too, for everyone but the owner (never listed as themself). */
		return people.filter(
			(person) =>
				(person.kind === 'human' || person.kind === 'owner') && matchesDirectory(person, query)
		);
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
		/* Opening a piece of Work names its own chat: the agent doing it. */
		if (explicitSelection || workId || !owner || recent.status === 'unknown') return;
		if (!window.matchMedia('(min-width: 761px)').matches) return;
		/* Reopen the latest conversation; with none yet, the welcome invites the first one. */
		const latest = rows[0]?.person;
		if (latest)
			goto(href(latest), { replaceState: true, noScroll: true, keepFocus: true }).catch(() => {
				/* A navigation the owner started meanwhile wins. */
			});
	});
	/* A hit opens its room at the message, the Exec's and leads' included: the room view pages
	 * back through the whole history to find it, where the rail holds only the recent end. */
	function searchHref(message: {
		id: number;
		room_id: string;
		thread_root_message_id: number | null;
	}): string {
		return `${href('', message.room_id)}${message.thread_root_message_id ? `&thread=${message.thread_root_message_id}` : ''}&focus=${message.id}`;
	}
	/* Where a hit was said, when the name alone does not say: a group's title. */
	function searchPlace(roomId: string): string {
		const room = rooms.rooms.find((candidate) => candidate.id === roomId);
		return room && room.kind !== 'direct' ? room.title : '';
	}
	function created(room: Room) {
		void goto(href('', room.id));
	}

	/* One directory (S88-02): everyone, grouped, with group chats at the end. A filter narrows it
	 * to who is working, who needs the person, or who was archived; ?show= keeps it in links. */
	const filter = $derived<DirectoryFilter>(directoryFilter(page.url.searchParams.get('show')));
	function chooseFilter(next: DirectoryFilter) {
		const url = new URL(page.url);
		if (next === 'everyone') url.searchParams.delete('show');
		else url.searchParams.set('show', next);
		void goto(`${url.pathname}${url.search}`, {
			replaceState: true,
			noScroll: true,
			keepFocus: true
		});
	}
	/* Arrow keys move between the filters, as between radios. */
	function switchFilterByKey(event: KeyboardEvent) {
		const step = { ArrowLeft: -1, ArrowUp: -1, ArrowRight: 1, ArrowDown: 1 }[event.key];
		if (!step) return;
		event.preventDefault();
		const at = DIRECTORY_FILTERS.findIndex((candidate) => candidate.key === filter);
		const next =
			DIRECTORY_FILTERS[(at + step + DIRECTORY_FILTERS.length) % DIRECTORY_FILTERS.length];
		chooseFilter(next.key);
		const group = (event.currentTarget as HTMLElement).parentElement;
		requestAnimationFrame(() =>
			group?.querySelector<HTMLButtonElement>('[aria-checked="true"]')?.focus()
		);
	}
	/* What each agent is on: their own Work in motion, or for a lead, their team's. */
	function currentWorkTitle(actorId: string): string {
		const member = officeMember(actorId);
		if (member?.work) return member.work.title;
		const team = teams.find((candidate) => candidate.lead_actor_id === actorId);
		const owners = new Set([
			actorId,
			...(team ? people.filter((p) => p.team_id === team.id).map((p) => p.actor_id) : [])
		]);
		return (
			(attention.view?.workGraph?.work ?? [])
				.filter(
					(item) =>
						owners.has(item.owner_id) && (item.status === 'active' || item.status === 'blocked')
				)
				.toSorted((x, y) => Date.parse(y.updated_at) - Date.parse(x.updated_at))[0]?.title ?? ''
		);
	}
	/* A row: one status word, and one line, the current Work or else the role (S88-02). A
	 * colleague keeps their presence line and has no status word. */
	function directoryLine(actorId: string): { word: StatusWord | ''; text: string; hint: string } {
		const person = people.find((candidate) => candidate.actor_id === actorId);
		const line = personLine(actorId);
		if (!person || person.kind === 'human')
			return { word: '', text: line.text, hint: line.hint || line.text };
		const ledTeam = teams.find((candidate) => candidate.lead_actor_id === actorId);
		const role =
			person.kind === 'exec'
				? 'Runs the company'
				: ledTeam
					? `Leads ${teamName(ledTeam.name, companyId)}`
					: person.role;
		const text = currentWorkTitle(actorId) || role || line.text;
		return { word: owner ? statusWord(line.tone) : '', text, hint: line.hint || line.text };
	}
	const filtered = $derived.by(() => {
		if (filter === 'everyone' || filter === 'archived') return [];
		const query = search.trim().toLocaleLowerCase();
		return people
			.filter((person) => person.kind !== 'human')
			.filter((person) =>
				filter === 'working'
					? personLine(person.actor_id).tone === 'work'
					: personStatus(person.actor_id).need !== null
			)
			.filter((person) => matchesDirectory(person, query));
	});
	const needsCount = $derived(
		owner
			? people.filter(
					(person) => person.kind !== 'human' && personStatus(person.actor_id).need !== null
				).length
			: 0
	);
	/* Archived agents keep their history; their page offers Re-hire (S88-01, S88-03). */
	let archivedAgents = $state<ArchivedAgent[]>([]);
	$effect(() => {
		if (!owner || filter !== 'archived') return;
		let live = true;
		void listArchivedAgents(companyId).then((agents) => {
			if (live) archivedAgents = agents;
		});
		return () => {
			live = false;
		};
	});
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
					muted: !!conversation.muted,
					unread:
						!conversation.muted &&
						conversationUnread(conversation, viewerId) &&
						personId !== person.actor_id
				}
			];
		});
		const spoken = new Map(recentGroups.conversations.map((group) => [group.room_id, group]));
		const loaded = new Set(rooms.rooms.map((room) => room.id));
		const groups = [...rooms.rooms, ...roomTitleSearch.rooms.filter((room) => !loaded.has(room.id))]
			.filter((room) => room.kind !== 'direct' && !room.archived_at)
			.map((room) => {
				const latest = spoken.get(room.id);
				const who = speaker(latest?.last_message_from);
				return {
					key: `room:${room.id}`,
					name: latest?.title || room.title,
					mark: '#',
					group: true,
					person: '',
					room: room.id,
					roomId: room.id,
					at: latest?.last_message_at ?? room.created_at,
					preview: latest
						? `${who ? `${who}: ` : ''}${messagePreview(latest.last_message_preview)}`
						: 'Nothing said yet',
					muted: !!latest?.muted,
					unread:
						!!latest && !latest.muted && conversationUnread(latest, viewerId) && roomId !== room.id
				};
			});
		return [...direct, ...groups]
			.filter(
				(chat) =>
					(chat.group || query || roomId === chat.roomId || !chatHidden(chat.roomId, chat.at)) &&
					(!query || `${chat.name} ${chat.preview}`.toLocaleLowerCase().includes(query))
			)
			.toSorted((a, b) => Date.parse(b.at) - Date.parse(a.at));
	});
	/* Hiding a direct chat is this viewer's tidying, kept on this device: it comes back by itself
	 * when someone writes again, and People still reaches the person. */
	let hiddenChats = $state<Record<string, string>>({});
	const hiddenKey = $derived(`restless:${companyId}:hidden-chats`);
	$effect(() => {
		try {
			hiddenChats = JSON.parse(localStorage.getItem(hiddenKey) ?? '{}') ?? {};
		} catch {
			hiddenChats = {};
		}
	});
	function keepHidden(next: Record<string, string>) {
		hiddenChats = next;
		try {
			localStorage.setItem(hiddenKey, JSON.stringify(next));
		} catch {
			/* Hidden for this visit only. */
		}
	}
	function hideChat(room: string, at: string, name: string) {
		keepHidden({ ...hiddenChats, [room]: at });
		toast(`Chat with ${name} hidden until someone writes again.`, {
			undo: () => {
				const rest = { ...hiddenChats };
				delete rest[room];
				keepHidden(rest);
			}
		});
	}
	function chatHidden(room: string, at: string): boolean {
		const since = hiddenChats[room];
		return !!since && Date.parse(at) <= Date.parse(since);
	}
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
	holdQueries(() => [
		principal,
		cockpit,
		collaboration,
		attention,
		rooms,
		recent,
		messageSearch,
		recentGroups
	]);
</script>

<svelte:head
	><title
		>People — {cockpit.view?.company.name ?? collaboration.view?.company.name ?? companyId}</title
	></svelte:head
>
<div
	class="conversation-workspace"
	class:selected={explicitSelection}
	class:with-work={!!workId}
	use:resizePane={{
		key: `${companyId}:sidebar:people`,
		label: 'Resize the People sidebar',
		target: '.conversation-index',
		variable: '--sidebar-width',
		min: 200,
		minOther: 420,
		defaultSize: 264
	}}
>
	<aside class="conversation-index" aria-label="Conversations">
		<div class="people-head">
			<h1>People</h1>
			{#if canManageMembers}
				{#if invitesReady}<button
						type="button"
						class="btn small room-manage-trigger invite-trigger"
						aria-expanded={inviting}
						title="Invite people to the company"
						onclick={() => (inviting = !inviting)}
						><UserPlus size={15} aria-hidden="true" /><span>Invite people</span></button
					>{:else}<a
						class="btn small room-manage-trigger invite-trigger"
						href={`/${encodeURIComponent(companyId)}/company/members`}
						title="Invite people to the company: sharing is set up under Members first"
						><UserPlus size={15} aria-hidden="true" /><span>Invite people</span></a
					>{/if}
			{/if}
			<RoomManager
				bind:this={manager}
				{companyId}
				actorId={principal.view?.actor_id ?? ''}
				{people}
				teams={teams.map((team) => ({ id: team.id, name: teamName(team.name, companyId) }))}
				oncreated={created}
				onperson={owner ? (id) => goto(href(id)) : undefined}
				contactIds={contacts.map((p) => p.actor_id)}
			/>
		</div>
		{#if inviting && invitesReady}
			<form class="people-invite" onsubmit={sendInvites}>
				<input
					aria-label="Emails to invite"
					type="text"
					inputmode="email"
					autocomplete="off"
					spellcheck="false"
					placeholder="colleague@example.com"
					title="Paste several addresses, separated by commas or spaces. They join as members; change access afterwards."
					required
					bind:value={inviteEmails}
					{@attach (input) => input.focus()}
					onkeydown={(event) => {
						if (event.key === 'Escape') inviting = false;
					}}
				/>
				<button class="btn small primary" type="submit" disabled={inviteBusy}
					>{inviteBusy ? 'Sending…' : 'Invite'}</button
				>
				{#if inviteFailure}<p class="empty" role="alert">{inviteFailure}</p>{/if}
			</form>
		{/if}
		<div class="people-search">
			<SidebarSearch
				bind:value={search}
				placeholder="Search people and Work"
				label="Search conversations and people"
			/>
		</div>
		<!-- Filters over one list: radio semantics and arrow keys. -->
		<div class="people-views" role="radiogroup" aria-label="Show">
			{#each DIRECTORY_FILTERS as choice (choice.key)}
				{#if choice.key !== 'needs' || owner}{#if choice.key !== 'archived' || owner}<button
							type="button"
							role="radio"
							aria-checked={filter === choice.key}
							tabindex={filter === choice.key ? 0 : -1}
							onkeydown={switchFilterByKey}
							onclick={() => chooseFilter(choice.key)}
							>{choice.label}{#if choice.key === 'needs' && needsCount}<b aria-hidden="true"
									>{needsCount}</b
								><span class="sr-only">, {needsCount}</span>{/if}</button
						>{/if}{/if}
			{/each}
		</div>
		<div class="entries">
			{#snippet groupChats()}
				{#each chats.filter((chat) => chat.group) as chat (chat.key)}
					<div class="person-row">
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
							<span class="avatar" aria-hidden="true">{chat.mark}</span>
							<span class="person-lines"
								><span class="name" class:strong={chat.unread}>{chat.name}</span><small
									class="doing"
									title={chat.preview}>{chat.preview}</small
								></span
							><span class="chat-meta"
								>{#if chat.muted}<BellOff
										size={12}
										class="chat-muted"
										aria-label="Muted"
										role="img"
									/>{/if}<time class="chat-time" datetime={chat.at}>{chatTime(chat.at)}</time
								>{#if chat.unread}<span
										class="new-dot"
										title="New since you last looked"
										role="img"
										aria-label="Unread"
									></span>{/if}</span
							>
						</a>
						<span class="person-menu"
							><ActionMenu label={`More for the chat with ${chat.name}`}
								><button
									type="button"
									title="Muted: no unread count or notification. A mention still reaches you."
									onclick={() =>
										void muteRoom(roomClient, companyId, chat.roomId, !chat.muted, chat.name)}
									>{chat.muted ? 'Unmute' : 'Mute'}</button
								>{#if !chat.group}<button
										type="button"
										onclick={() => hideChat(chat.roomId, chat.at, chat.name)}>Hide chat</button
									>{/if}</ActionMenu
							></span
						>
					</div>
				{:else}
					<p class="empty">
						{search.trim() ? 'No group chats match.' : 'No group chats yet. Start one with +.'}
					</p>
				{/each}
				{#if rooms.hasMore && !search.trim()}
					<button
						type="button"
						class="btn small ghost more-chats"
						disabled={rooms.loadingMore}
						{@attach (button) => {
							/* Older groups load as the list scrolls to its end; the button stays for keys. */
							const observer = new IntersectionObserver((entries) => {
								if (entries.some((entry) => entry.isIntersecting) && !rooms.loadingMore)
									void rooms.loadMore();
							});
							observer.observe(button);
							return () => observer.disconnect();
						}}
						onclick={() => void rooms.loadMore()}
						>{rooms.loadingMore ? 'Loading…' : 'Show older groups'}</button
					>
				{/if}
				{#if archived.rooms.length && !search.trim()}
					<details class="archived-chats">
						<summary
							class="team-directory-head"
							title="Archived groups you are still in. Open one to read it; its owner can restore it for everyone."
							><span>Archived</span><em>{archived.rooms.length}</em></summary
						>
						{#each archived.rooms as room (room.id)}
							<div class="archived-chat">
								<span class="avatar" aria-hidden="true">#</span>
								<a
									class="name"
									href={href('', room.id)}
									aria-current={roomId === room.id ? 'page' : undefined}
									title="Read it as it was left">{room.title}</a
								>
								{#if archived.owns(room.id)}<button
										type="button"
										class="btn small ghost"
										disabled={!!restoring}
										onclick={() => void restore(room)}
										>{restoring === room.id ? 'Restoring…' : 'Restore'}</button
									>{/if}
							</div>
						{/each}
					</details>
				{/if}
			{/snippet}
			{#snippet personMenu(person: DirectoryPerson)}
				{@const work = officeMember(person.actor_id)?.work}
				{@const membership = person.kind === 'human' ? membershipFor(person.actor_id) : null}
				<span class="person-menu">
					<ActionMenu label={`More for ${personName(person.display)}`}>
						<button type="button" onclick={() => (profileId = person.actor_id)}>Profile</button>
						{#if work}<a
								href={`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(work.id)}`}
								title={work.title}>Open current Work</a
							>{/if}
						{#if owner && person.kind === 'staff'}
							<button type="button" onclick={() => askAbout('rebrief', person)}
								>Ask Exec to re-brief…</button
							>
							<button type="button" onclick={() => askAbout('pause', person)}
								>Ask Exec to pause…</button
							>
							<button type="button" onclick={() => askAbout('retire', person)}
								>Ask Exec to retire…</button
							>
							<button type="button" onclick={() => askAbout('hire-like', person)}
								>Ask Exec to add someone like this…</button
							>
						{/if}
						{#if membership && issuerMembers}
							{#if issuerMembers.viewer.role === 'owner' && membership.status === 'active'}
								<button
									type="button"
									disabled={memberBusy}
									onclick={() =>
										setRole(membership, membership.role === 'admin' ? 'member' : 'admin')}
									>{membership.role === 'admin' ? 'Make a member' : 'Make an admin'}</button
								>
							{/if}
							<button
								type="button"
								disabled={memberBusy || membership.ending}
								onclick={() => void removeMember(membership)}>Remove access…</button
							>
						{/if}
					</ActionMenu>
				</span>
			{/snippet}
			{#snippet directoryPerson(person: DirectoryPerson, kind: string)}
				{@const line = personLine(person.actor_id)}
				{@const row = directoryLine(person.actor_id)}
				{@const isLead = kind === 'lead'}
				<div class="person-row">
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
									role="img"
									aria-label="Working"
								></i>{:else if person.kind === 'human' && hereNow.has(person.actor_id)}<i
									class="avatar-working here"
									title={`${personName(person.display)} has the cockpit open`}
									role="img"
									aria-label="Here now"
								></i>{/if}</span
						>
						<span class="person-lines"
							><span class="name-line"
								><span class="name"
									>{personName(person.display)}{#if isLead}<span class="role-tag">{' · Lead'}</span
										>{/if}</span
								>{#if row.word}<span class="status-word {row.word.toLowerCase()}">{row.word}</span
									>{/if}</span
							>{#if row.text}<small class="doing" title={row.hint}>{row.text}</small>{/if}</span
						>{#if hasNew(person.actor_id) && personId !== person.actor_id}<span
								class="new-dot"
								title="Something new since you last looked"
								role="img"
								aria-label="New messages"
							></span>{/if}
					</a>
					{@render personMenu(person)}
				</div>
			{/snippet}
			{#if filter === 'everyone'}
				{#if execShown}
					<section class="directory-section executive-section" aria-label="Executive">
						{@render directoryPerson(directoryExec!, 'executive')}
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
							>{#if owner}<div class="team-menu">
									<ActionMenu label={`Change ${teamName(entry.team.name, companyId)}`}>
										<button
											type="button"
											onclick={() => (teamEdit = { mode: 'rename', team: entry.team.id })}
											>Rename…</button
										>
										<button
											type="button"
											disabled={agents.length < 2}
											onclick={() => (teamEdit = { mode: 'lead', team: entry.team.id })}
											>Change lead…</button
										>
										<button type="button" onclick={() => void disband(entry.team)}>Disband…</button>
										{#each humanColleagues.length ? [{ actor_id: '', display: 'You' }, ...humanColleagues] : [] as choice (choice.actor_id)}
											{@const current = (teamOwnerId(entry.team) ?? '') === choice.actor_id}
											<button
												type="button"
												title="The person who owns a team directs it and decides what its lead passes up"
												aria-current={current ? 'true' : undefined}
												disabled={current || teamOwnerBusy === entry.team.id}
												onclick={() => void setTeamOwner(entry.team, choice.actor_id)}
												>{current ? 'Owned by' : 'Make owner:'}
												{choice.actor_id ? personName(choice.display) : 'You'}</button
											>
										{/each}
									</ActionMenu>
								</div>{/if}
						</header>
						{#if teamOwnerId(entry.team)}
							<p class="team-owner">Owned by {teamOwnerName(teamOwnerId(entry.team) ?? '')}</p>
						{:else if owner}
							<!-- A team nobody else owns is the owner's (S85-19). -->
							<p class="team-owner">Owned by you</p>
						{/if}
						{#if teamEdit && teamEdit.team === entry.team.id && teamEdit.mode !== 'new'}
							<TeamEditor
								{companyId}
								mode={teamEdit.mode}
								team={entry.team}
								{agents}
								ondone={(message) => void teamEdited(message)}
								oncancel={() => (teamEdit = null)}
							/>
						{/if}
						{#if entry.lead}{@render directoryPerson(entry.lead, 'lead')}{/if}
						{#each entry.members.filter((member) => entry.teamMatches || matchesDirectory(member, search
										.trim()
										.toLocaleLowerCase())) as member (member.actor_id)}
							{@render directoryPerson(member, entry.lead ? 'member teammate' : 'member')}
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
							<div class="person-row">
								<div class="directory-person colleague invited">
									<span class="avatar">{invite.email.slice(0, 1).toUpperCase()}</span>
									<span class="person-lines"
										><span class="name">{invite.email}</span><small class="doing"
											>Invited · link expires {inviteExpiry(invite.expires_at)}</small
										></span
									>
								</div>
								<span class="person-menu">
									<ActionMenu label={`Invitation to ${invite.email}`}>
										{#if coreMembers?.issuer?.invitation_delivery.includes('link')}<button
												type="button"
												onclick={() => void copyInvitation(invite)}>Copy invitation link</button
											>{/if}
										{#if mayChangeInvitation(invite)}
											<button
												type="button"
												disabled={memberBusy}
												title="Email the invitation again"
												onclick={() =>
													void memberChange(
														'invitations',
														{ email: invite.email, role: invite.role, resend: true },
														`Invitation sent to ${invite.email} again.`
													)}>Resend invitation</button
											>
											<button
												type="button"
												disabled={memberBusy}
												onclick={() => void cancelInvitation(invite)}>Cancel invitation…</button
											>
										{/if}
									</ActionMenu>
								</span>
							</div>
						{/each}
					</section>
				{/if}
				<section class="directory-section chats-section" aria-label="Group chats">
					<header class="team-directory-head"><span>Chats</span></header>
					{@render groupChats()}
				</section>
				{#if owner && agents.length && !search.trim()}
					{#if teamEdit?.mode === 'new'}
						<section class="directory-section" aria-label="New team">
							<header class="team-directory-head"><span>New team</span></header>
							<TeamEditor
								{companyId}
								mode="new"
								{agents}
								ondone={(message) => void teamEdited(message)}
								oncancel={() => (teamEdit = null)}
							/>
						</section>
					{:else}
						<button
							type="button"
							class="more"
							title="Form a team around an accountable lead"
							onclick={() => (teamEdit = { mode: 'new', team: '' })}>New team…</button
						>
					{/if}
				{/if}
				{#if !execShown && !directoryTeams.length && !directoryUnassigned.length && !directoryHumans.length && !shownInvitations.length}<p
						class="empty"
					>
						No people match.
					</p>{/if}
				{#if sparse}
					<!-- A new group chat is the + above; this only says why the list is short. -->
					<SidebarEmpty text="Leads and staff appear here as the Exec builds the team.">
						{#if owner}<a href={`/${encodeURIComponent(companyId)}/company/members`}
								><UserPlus size={15} strokeWidth={1.8} aria-hidden="true" />Invite a colleague</a
							>{/if}
					</SidebarEmpty>
				{/if}
			{:else if filter === 'archived'}
				<section class="directory-section" aria-label="Archived agents">
					{#each archivedAgents.filter((agent) => matchesDirectory(agent, search
								.trim()
								.toLocaleLowerCase())) as agent (agent.actor_id)}
						<div class="person-row">
							<a
								class="directory-person archived"
								href={href(agent.actor_id)}
								aria-current={personId === agent.actor_id ? 'page' : undefined}
							>
								<span class="avatar" aria-hidden="true">{initials(agent.display)}</span>
								<span class="person-lines"
									><span class="name-line"
										><span class="name">{personName(agent.display)}</span><span
											class="status-word idle">Archived</span
										></span
									><small class="doing" title={agent.retirement_reason || agent.role}
										>{agent.retirement_reason || agent.role}</small
									></span
								>
							</a>
						</div>
					{:else}
						<p class="empty">No archived agents.</p>
					{/each}
				</section>
			{:else}
				<section
					class="directory-section"
					aria-label={filter === 'working' ? 'Working' : 'Needs you'}
				>
					{#each filtered as person (person.actor_id)}
						{@render directoryPerson(person, person.kind === 'exec' ? 'executive' : 'member')}
					{:else}
						<p class="empty">
							{filter === 'working'
								? 'Nobody is working right now.'
								: 'Nobody needs you right now.'}
						</p>
					{/each}
				</section>
			{/if}
			{#if search.trim()}
				<!-- The server already limits hits to conversations this person can read, groups
				     included. -->
				<section class="directory-section message-hits" aria-label="Messages" aria-live="polite">
					<header class="team-directory-head">
						<span>Messages</span>{#if messageSearch.messages.length}<em
								>{messageSearch.messages.length}{messageSearch.hasMore ? '+' : ''}</em
							>{/if}
					</header>
					{#if search.trim() !== debouncedSearch || (messageSearch.status === 'unknown' && !messageSearch.failure)}
						<p class="empty">Searching messages…</p>
					{:else if !messageSearch.failure}
						{#each messageSearch.messages as message (message.id)}
							{@const place = searchPlace(message.room_id)}
							<a class="search-result" href={searchHref(message)}
								><strong
									>{speaker(message.from_actor) || 'Someone'}{#if place}<span class="search-place"
											>{` · ${place}`}</span
										>{/if}</strong
								><span>{message.snippet}</span></a
							>
						{:else}
							<p class="empty">No messages match.</p>
						{/each}
						{#if messageSearch.hasMore}<button
								class="more"
								disabled={messageSearch.loadingMore}
								onclick={() => messageSearch.loadMore()}
								>{messageSearch.loadingMore ? 'Loading…' : 'More messages'}</button
							>{/if}
					{/if}
				</section>
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
		{:else if personId === 'exec' && !roomId}
			<!-- Exec's chat is the rail itself, widened into this pane a moment after it mounts; a
			     room view's skeleton would only flash first. -->
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
	{#if workId}
		<aside class="work-sheet" aria-label="Work">
			{#key workId}<WorkDetail
					{companyId}
					{workId}
					onclose={closeWork}
					onowner={openOwnerChat}
				/>{/key}
		</aside>
	{/if}
</div>
{#if profileId}
	{@const profiled = people.find((person) => person.actor_id === profileId)}
	<PersonProfile {companyId} actorId={profileId} onclose={() => (profileId = '')}>
		{#snippet actions()}
			<a class="btn small primary" href={href(profileId)} onclick={() => (profileId = '')}
				>Message</a
			>
			{#if owner && profiled?.kind === 'staff'}
				<button type="button" class="btn small" onclick={() => askAbout('rebrief', profiled)}
					>Re-brief…</button
				>
				<button type="button" class="btn small" onclick={() => askAbout('pause', profiled)}
					>Pause…</button
				>
				<button type="button" class="btn small" onclick={() => askAbout('retire', profiled)}
					>Retire…</button
				>
				<button type="button" class="btn small" onclick={() => askAbout('hire-like', profiled)}
					>Add someone like this…</button
				>
			{/if}
		{/snippet}
	</PersonProfile>
{/if}

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
	/* Four filters in one row at the narrowest sidebar: each takes what its word needs. */
	.people-views {
		display: flex;
		flex: none;
		gap: 0;
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
		flex: 1 1 auto;
		gap: 4px;
		min-width: 0;
		min-height: 28px;
		padding: 0 4px;
		border: 0;
		border-radius: var(--radius-pane);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-label);
		white-space: nowrap;
		font-weight: 500;
		cursor: pointer;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.people-views button:hover {
		color: var(--ink);
	}
	.people-views button[aria-checked='true'] {
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
		min-width: 16px;
		height: 16px;
		padding: 0 4px;
		border-radius: 8px;
		background: var(--intent-conversation);
		color: var(--on-primary);
		font-size: var(--t-label);
		font-weight: 600;
		line-height: 16px;
	}
	.entries {
		flex: 1 1 auto;
		overflow-y: auto;
		overflow-x: hidden;
		min-height: 0;
		padding: 1px 2px 8px;
	}
	/* One column no wider than the sidebar: a long status ellipsizes instead of widening the row
	 * under its hover menu (S85-20). */
	.directory-section {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
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
	.more-chats {
		display: block;
		margin: var(--space-2) auto;
	}
	.archived-chats {
		margin-top: 8px;
	}
	.archived-chats summary {
		cursor: pointer;
		list-style: none;
	}
	.archived-chats summary::-webkit-details-marker {
		display: none;
	}
	.archived-chat {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 36px;
		padding: 0 4px 0 8px;
		color: var(--text-secondary);
	}
	.archived-chat a.name {
		color: inherit;
		text-decoration: none;
	}
	.archived-chat a.name:hover,
	.archived-chat a.name[aria-current='page'] {
		color: var(--ink);
	}
	.archived-chat .name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.team-owner {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0 0 4px;
		padding: 0 8px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	/* Who owns the team is changed from the header's menu, shown on hover or focus. */
	.team-menu {
		margin-left: auto;
		font-weight: 400;
		opacity: 0;
		transition: opacity var(--motion-state) var(--ease-standard);
	}
	.team-directory-head:hover .team-menu,
	.team-directory-head:focus-within .team-menu,
	.team-menu:has(:global(details[open])) {
		opacity: 1;
	}
	@media (hover: none) {
		.team-menu {
			opacity: 1;
		}
	}
	/* Each row keeps its menu beside the link, shown on hover or focus (always on touch). */
	.person-row {
		position: relative;
		min-width: 0;
	}
	.person-menu {
		position: absolute;
		top: 8px;
		right: 4px;
		opacity: 0;
		transition: opacity var(--motion-state) var(--ease-standard);
	}
	.person-row:hover .person-menu,
	.person-row:focus-within .person-menu,
	.person-menu:has(:global(details[open])) {
		opacity: 1;
	}
	@media (hover: none) {
		.person-menu {
			opacity: 1;
		}
	}
	.person-row:hover .directory-person,
	.person-row:focus-within .directory-person {
		padding-right: 34px;
	}
	.directory-person.teammate + .person-menu {
		top: 3px;
	}
	.people-invite {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		gap: 6px;
		padding: 4px 2px 6px 8px;
	}
	.people-invite input {
		min-width: 0;
		padding: 5px 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
	}
	.people-invite .empty {
		grid-column: 1 / -1;
		padding: 0;
	}
	.people-head :global(.invite-trigger) {
		margin-left: auto;
		margin-right: 6px;
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
	.name-line {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
	}
	.name-line .name {
		flex: 0 1 auto;
	}
	/* One word, coloured by meaning (S88-02). */
	.status-word {
		flex: none;
		margin-left: auto;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.status-word.working {
		color: var(--intent-feedback);
	}
	.status-word.waiting {
		color: var(--intent-authority);
	}
	.status-word.blocked {
		color: var(--state-danger);
	}
	/* Under a lead, the team reads as the lead's: indented, one line each, the role beside the
	 * name, so a team of nine is a short list rather than nine cards. */
	.directory-person.teammate {
		align-items: center;
		min-height: 30px;
		margin-left: 18px;
		padding: 2px 8px;
	}
	.directory-person.teammate .avatar {
		width: 20px;
		height: 20px;
		margin-top: 0;
		border-radius: 5px;
		font-size: var(--t-label);
	}
	.directory-person.teammate .doing {
		font-size: var(--t-label);
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
	.chat-meta :global(.chat-muted) {
		color: var(--text-tertiary);
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
	.avatar-working.here {
		background: var(--state-success);
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
	.message-hits {
		margin-top: 12px;
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
	.search-result .search-place {
		display: inline;
		color: var(--text-tertiary);
		font-weight: 400;
	}
	.search-result > span {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.back {
		display: none;
	}
	.conversation-workspace.with-work {
		grid-template-columns:
			var(--sidebar-width, 264px) minmax(0, 1fr)
			var(--work-sheet-width, 460px);
	}
	.work-sheet {
		position: relative;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
		display: flex;
	}
	.work-sheet > :global(*) {
		flex: 1;
		min-width: 0;
	}
	/* Narrower, the conversation list steps aside for the chat and the sheet. */
	@media (max-width: 1180px) {
		.conversation-workspace.with-work {
			grid-template-columns: minmax(0, 1fr) var(--work-sheet-width, 420px);
		}
		.with-work .conversation-index {
			display: none;
		}
	}
	@media (max-width: 760px) {
		.conversation-workspace,
		.conversation-workspace.with-work {
			grid-template-columns: minmax(0, 1fr);
		}
		/* On a phone the sheet covers the chat; closing it returns there. */
		.work-sheet {
			position: fixed;
			inset: 0;
			z-index: 30;
			background: var(--surface-raised);
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
