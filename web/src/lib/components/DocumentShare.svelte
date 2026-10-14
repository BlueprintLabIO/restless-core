<script lang="ts">
	/* Who can open a document: its general access for the company's people, and anyone added by
	 * name, Exec and Staff included, at view, comment or edit. Exec and Staff never see a document
	 * through general access; adding them is how one reaches them. Whoever may not manage sharing
	 * sees the same, read-only. */
	import { untrack } from 'svelte';
	import X from '@lucide/svelte/icons/x';
	import SharePanel from '$lib/components/SharePanel.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { GENERAL_ACCESS_OPTIONS, type GeneralAccess } from '$lib/model/general-access';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { mayManageCompanySetup } from '$lib/model/company-access';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { AlreadyMember, emailInvites, inviteToCompany } from '$lib/model/members';
	import {
		FORMER_MEMBER,
		invitedFromShare,
		sharedWithGuest,
		shareRows
	} from '$lib/model/share-people';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import {
		cancelPendingShare,
		getPendingShares,
		shareWithInvitee,
		type PendingShare
	} from '$lib/model/pending-shares';
	import type { DocumentTarget } from '$lib/model/document-cache';
	import {
		getDocument,
		getDocumentParticipants,
		isDocumentConflict,
		removeDocumentParticipant,
		setDocumentGeneralAccess,
		setDocumentParticipant,
		type DocumentAccess,
		type DocumentParticipantCursor,
		type DocumentParticipantRow,
		type DocumentReadView
	} from '$lib/model/documents';

	interface Person {
		actor_id: string;
		display: string;
		kind?: string;
	}

	let {
		companyId,
		view,
		people = [],
		onchanged = () => {},
		onclose
	}: {
		companyId: string;
		view: DocumentReadView;
		people?: Person[];
		/** The document after a sharing change, so the page holds its new version. */
		onchanged?: (target: DocumentTarget, view: DocumentReadView) => void;
		onclose: () => void;
	} = $props();

	const LEVELS: ReadonlyArray<{ value: DocumentAccess; label: string }> = [
		{ value: 'read', label: 'Can view' },
		{ value: 'comment', label: 'Can comment' },
		{ value: 'edit', label: 'Can edit' }
	];

	let participants = $state<DocumentParticipantRow[]>([]);
	let mayManage = $state(false);
	let pending = $state<PendingShare[]>([]);
	let loaded = $state(false);
	let busy = $state(false);
	let failure = $state('');

	const documentId = $derived(view.document.id);
	const ownerId = $derived(view.document.owner_actor_id);
	const principalSource = $derived(companyPrincipalQuery(companyId));
	holdQueries(() => [principalSource]);
	const viewerId = $derived(principalSource.view?.actor_id ?? '');
	/* Whoever may share it and may invite to the company can invite by email from here. */
	const canInvite = $derived(mayManage && mayManageCompanySetup(principalSource.view));
	/* Email invitations go out only once sharing is set up; until then the panel says where. */
	let invitesReady = $state<{ company: string; ready: boolean; guests: boolean } | null>(null);
	$effect(() => {
		const target = companyId;
		if (!canInvite || invitesReady?.company === target) return;
		void emailInvites(target)
			.then(({ ready, guests }) => {
				if (target === companyId) invitesReady = { company: target, ready, guests };
			})
			.catch(() => undefined);
	});
	const invitesFor = $derived(invitesReady?.company === companyId ? invitesReady.ready : null);
	/* Someone outside the company is shared only this, as a guest, unless invited as a member. */
	const guestInvites = $derived(invitesReady?.company === companyId && invitesReady.guests);
	const nameOf = (actorId: string) =>
		people.find((candidate) => candidate.actor_id === actorId)?.display ?? FORMER_MEMBER;
	const shared = $derived(
		shareRows(participants, {
			owner: ownerId,
			viewer: viewerId,
			nameOf: (actorId) => people.find((candidate) => candidate.actor_id === actorId)?.display
		})
	);
	const directory = $derived(people.filter((person) => person.actor_id !== ownerId));
	const link = $derived(
		typeof location === 'undefined'
			? ''
			: new URL(
					`/${encodeURIComponent(companyId)}/library/documents?document=${encodeURIComponent(documentId)}`,
					location.origin
				).href
	);

	async function load(target: DocumentTarget): Promise<void> {
		const rows: DocumentParticipantRow[] = [];
		let cursor: DocumentParticipantCursor | null = null;
		let manages = false;
		for (let page = 0; page < 10; page += 1) {
			const result = await getDocumentParticipants(target.companyId, target.documentId, cursor);
			rows.push(...result.items);
			manages = result.may_manage;
			cursor = result.next_cursor;
			if (!cursor) break;
		}
		const waiting = manages
			? await getPendingShares(target.companyId, { document: target.documentId })
			: [];
		if (target.companyId !== companyId || target.documentId !== documentId) return;
		participants = rows;
		pending = waiting;
		mayManage = manages;
		loaded = true;
	}

	$effect(() => {
		const target = { companyId, documentId };
		untrack(() => {
			participants = [];
			pending = [];
			mayManage = false;
			loaded = false;
			failure = '';
			void load(target).catch((cause) => {
				failure = failureSentence(cause, 'Sharing could not be loaded.');
			});
		});
	});

	/* Every change names the document version it was decided against, then reads the result back,
	 * so two people changing sharing at once cannot silently overwrite each other. */
	async function change(
		run: (version: number, commandId: string) => Promise<unknown>,
		fallback: string
	): Promise<boolean> {
		if (busy) return false;
		busy = true;
		failure = '';
		const target = { companyId, documentId };
		let done = true;
		try {
			await run(view.document.version, crypto.randomUUID());
		} catch (cause) {
			done = false;
			failure = isDocumentConflict(cause)
				? 'Sharing changed while you were looking. Check it and try again.'
				: failureSentence(cause, fallback);
		}
		try {
			const latest = await getDocument(target.companyId, target.documentId);
			onchanged(target, latest);
			await load(target);
		} catch (cause) {
			failure ||= failureSentence(cause, 'Sharing could not be refreshed.');
		} finally {
			busy = false;
		}
		return done;
	}

	function add(actorId: string, access: string): Promise<boolean> {
		return change(
			(version, commandId) =>
				setDocumentParticipant(
					companyId,
					documentId,
					actorId,
					version,
					access as DocumentAccess,
					commandId
				),
			`${nameOf(actorId)} was not added.`
		);
	}

	function setAccess(actorId: string, access: string): void {
		participants = participants.map((item) =>
			item.actor_id === actorId ? { ...item, access: access as DocumentAccess } : item
		);
		void change(
			(version, commandId) =>
				setDocumentParticipant(
					companyId,
					documentId,
					actorId,
					version,
					access as DocumentAccess,
					commandId
				),
			'Access was not changed.'
		);
	}

	/* Removing someone says so, with Undo: it adds them back at the level they had. */
	function remove(actorId: string): void {
		if (busy) return;
		const before = participants.find((item) => item.actor_id === actorId);
		participants = participants.filter((item) => item.actor_id !== actorId);
		void change(
			(version, commandId) =>
				removeDocumentParticipant(companyId, documentId, actorId, version, commandId),
			'Access was not removed.'
		).then((done) => {
			if (!done || !before) return;
			const who = actorId === viewerId ? 'You no longer have' : `${nameOf(actorId)} no longer has`;
			toast(`${who} access.`, {
				undo: async () => {
					if (!(await add(actorId, before.access)))
						toast(failure || `${nameOf(actorId)} was not added back.`, { tone: 'failure' });
				}
			});
		});
	}

	async function invite(email: string, access: string, role: 'guest' | 'member'): Promise<boolean> {
		failure = '';
		try {
			await inviteToCompany(companyId, email, role);
			await shareWithInvitee(companyId, { document: documentId }, email, access);
			toast(role === 'guest' ? sharedWithGuest(email) : invitedFromShare(email));
			void load({ companyId, documentId }).catch(() => undefined);
			return true;
		} catch (cause) {
			if (cause instanceof AlreadyMember) return add(cause.member.actor_id, access);
			failure = failureSentence(cause, `${email} was not invited.`);
			return false;
		}
	}

	/* Invitees: changing the level they will get, or stopping, works as for colleagues. */
	function setPendingAccess(email: string, access: string): void {
		void change(
			() => shareWithInvitee(companyId, { document: documentId }, email, access),
			`What ${email} gets was not changed.`
		);
	}
	function removePending(email: string): void {
		const before = pending.find((row) => row.email === email);
		void change(
			() => cancelPendingShare(companyId, { document: documentId }, email),
			'It is still shared.'
		).then((done) => {
			if (!done || !before) return;
			toast(`No longer shared with ${email}.`, {
				undo: () =>
					change(
						() => shareWithInvitee(companyId, { document: documentId }, email, before.access),
						`${email} was not added back.`
					)
			});
		});
	}

	function setGeneral(value: GeneralAccess): void {
		void change(
			(version, commandId) =>
				setDocumentGeneralAccess(companyId, documentId, version, value, commandId),
			'General access was not changed.'
		);
	}
</script>

<section class="document-share" aria-label="Share">
	<header class="share-head">
		<h2>Share</h2>
		<button type="button" class="close" aria-label="Close" title="Close" onclick={onclose}
			><X size={16} strokeWidth={2} /></button
		>
	</header>
	<div class="share-body">
		<SharePanel
			general={view.document.general_access}
			generalOptions={GENERAL_ACCESS_OPTIONS}
			levels={LEVELS}
			ownerName={nameOf(ownerId)}
			ownerIsYou={!!viewerId && ownerId === viewerId}
			people={shared}
			{directory}
			canManage={mayManage}
			{busy}
			{loaded}
			{failure}
			{link}
			ongeneral={setGeneral}
			onadd={add}
			onaccess={setAccess}
			onremove={remove}
			oninvite={canInvite && invitesFor ? invite : undefined}
			{guestInvites}
			inviteSetupHref={canInvite && invitesFor === false
				? `/${encodeURIComponent(companyId)}/company/members`
				: undefined}
			{pending}
			onpendingaccess={setPendingAccess}
			onpendingremove={removePending}
		/>
	</div>
</section>

<style>
	.document-share {
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--surface-rail);
	}
	.share-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
		min-height: var(--pane-head-h, 52px);
		padding: 8px 12px 8px 16px;
		border-bottom: 1px solid var(--border);
	}
	h2 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.close {
		display: grid;
		place-items: center;
		flex: none;
		width: 28px;
		height: 28px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.close:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.share-body {
		min-height: 0;
		overflow: auto;
		padding: 14px 12px;
	}
</style>
