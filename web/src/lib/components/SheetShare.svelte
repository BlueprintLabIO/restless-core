<script lang="ts">
	/* Who can open a sheet, as for documents: its general access for the company's people, and
	 * anyone added by name, Exec and Staff included, at view or edit. Whoever may not manage
	 * sharing sees the same, read-only. */
	import { untrack } from 'svelte';
	import SharePanel from '$lib/components/SharePanel.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { generalAccessOptions, type GeneralAccess } from '$lib/model/general-access';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { mayManageCompanySetup } from '$lib/model/company-access';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { AlreadyMember, inviteToCompany } from '$lib/model/members';
	import { FORMER_MEMBER, invitedFromShare, shareRows } from '$lib/model/share-people';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import {
		getSheetSharing,
		setSheetGeneralAccess,
		setSheetParticipant,
		sheetHref,
		type SheetRow,
		type SheetSharing
	} from '$lib/model/sheets';

	interface Person {
		actor_id: string;
		display: string;
		kind?: string;
	}

	let {
		company,
		sheet,
		people = [],
		onchanged = () => {}
	}: {
		company: string;
		sheet: string;
		people?: Person[];
		/** The sheet after a general access change, so the header's lock follows. */
		onchanged?: (row: SheetRow) => void;
	} = $props();

	const LEVELS = [
		{ value: 'read', label: 'Can view' },
		{ value: 'edit', label: 'Can edit' }
	] as const;

	let sharing = $state<SheetSharing | null>(null);
	let busy = $state(false);
	let failure = $state('');

	const principalSource = $derived(companyPrincipalQuery(company));
	holdQueries(() => [principalSource]);
	const viewerId = $derived(principalSource.view?.actor_id ?? '');
	const display = (actorId: string) =>
		people.find((candidate) => candidate.actor_id === actorId)?.display;
	const nameOf = (actorId: string) => display(actorId) ?? FORMER_MEMBER;
	const owner = $derived(sharing?.owner_actor_id ?? '');
	const shared = $derived(
		shareRows(sharing?.people ?? [], { owner, viewer: viewerId, nameOf: display })
	);
	const canInvite = $derived(!!sharing?.may_manage && mayManageCompanySetup(principalSource.view));
	const directory = $derived(people.filter((person) => person.actor_id !== owner));
	const link = $derived(
		typeof location === 'undefined' ? '' : new URL(sheetHref(company, sheet), location.origin).href
	);

	async function load(target: { company: string; sheet: string }): Promise<void> {
		const value = await getSheetSharing(target.company, target.sheet);
		if (target.company === company && target.sheet === sheet) sharing = value;
	}

	$effect(() => {
		const target = { company, sheet };
		untrack(() => {
			sharing = null;
			failure = '';
			void load(target).catch((cause) => {
				failure = failureSentence(cause, 'Sharing could not be loaded.');
			});
		});
	});

	async function change(run: () => Promise<unknown>, fallback: string): Promise<boolean> {
		if (busy) return false;
		busy = true;
		failure = '';
		const target = { company, sheet };
		let done = true;
		try {
			await run();
		} catch (cause) {
			done = false;
			failure = failureSentence(cause, fallback);
		}
		try {
			await load(target);
		} catch (cause) {
			failure ||= failureSentence(cause, 'Sharing could not be refreshed.');
		} finally {
			busy = false;
		}
		return done;
	}

	function add(actorId: string, access: 'read' | 'edit'): Promise<boolean> {
		return change(
			() => setSheetParticipant(company, sheet, actorId, access),
			`${nameOf(actorId)} was not added.`
		);
	}

	/* Removing someone says so, with Undo: it adds them back at the level they had. */
	function remove(actorId: string): void {
		if (busy) return;
		const before = sharing?.people.find((row) => row.actor_id === actorId);
		optimistic(actorId, null);
		void change(
			() => setSheetParticipant(company, sheet, actorId, null),
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

	async function invite(email: string, access: string): Promise<boolean> {
		failure = '';
		try {
			await inviteToCompany(company, email);
			toast(invitedFromShare(email, sharing?.general_access ?? 'private'));
			return true;
		} catch (cause) {
			if (cause instanceof AlreadyMember)
				return add(cause.member.actor_id, access as 'read' | 'edit');
			failure = failureSentence(cause, `${email} was not invited.`);
			return false;
		}
	}

	/* Show a level change or removal at once; the read-back after it restores the truth. */
	function optimistic(actorId: string, access: 'read' | 'edit' | null): void {
		if (!sharing) return;
		const people = access
			? sharing.people.map((row) => (row.actor_id === actorId ? { ...row, access } : row))
			: sharing.people.filter((row) => row.actor_id !== actorId);
		sharing = { ...sharing, people };
	}

	function setGeneral(value: GeneralAccess): void {
		void change(
			async () => onchanged(await setSheetGeneralAccess(company, sheet, value)),
			'General access was not changed.'
		);
	}
</script>

{#if !sharing}
	<p class="quiet" role={failure ? 'alert' : undefined}>{failure || 'Loading…'}</p>
{:else}<SharePanel
		general={sharing.general_access}
		generalOptions={generalAccessOptions('sheet')}
		levels={LEVELS}
		ownerName={nameOf(owner)}
		ownerIsYou={!!viewerId && owner === viewerId}
		people={shared}
		{directory}
		canManage={sharing.may_manage}
		{busy}
		{failure}
		{link}
		ongeneral={setGeneral}
		onadd={(actorId, access) => void add(actorId, access as 'read' | 'edit')}
		oninvite={canInvite ? invite : undefined}
		onaccess={(actorId, access) => {
			if (busy) return;
			optimistic(actorId, access as 'read' | 'edit');
			void change(
				() => setSheetParticipant(company, sheet, actorId, access as 'read' | 'edit'),
				'Access was not changed.'
			);
		}}
		onremove={remove}
	/>{/if}

<style>
	.quiet {
		margin: 4px;
		color: var(--text-tertiary);
	}
</style>
