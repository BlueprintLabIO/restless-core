<script lang="ts">
	/* Who can open a sheet, as for documents: its general access for the company's people, and
	 * anyone added by name, Exec and Staff included, at view or edit. Whoever may not manage
	 * sharing sees the same, read-only. */
	import { untrack } from 'svelte';
	import SharePanel from '$lib/components/SharePanel.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { generalAccessOptions, type GeneralAccess } from '$lib/model/general-access';
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

	const nameOf = (actorId: string) =>
		people.find((candidate) => candidate.actor_id === actorId)?.display ?? actorId;
	const owner = $derived(sharing?.owner_actor_id ?? '');
	const shared = $derived(
		(sharing?.people ?? [])
			.filter((row) => row.actor_id !== owner)
			.map((row) => ({ actor_id: row.actor_id, name: nameOf(row.actor_id), access: row.access }))
	);
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

	async function change(run: () => Promise<unknown>, fallback: string): Promise<void> {
		if (busy) return;
		busy = true;
		failure = '';
		const target = { company, sheet };
		try {
			await run();
		} catch (cause) {
			failure = failureSentence(cause, fallback);
		}
		try {
			await load(target);
		} catch (cause) {
			failure ||= failureSentence(cause, 'Sharing could not be refreshed.');
		} finally {
			busy = false;
		}
	}

	function setGeneral(value: GeneralAccess): void {
		void change(async () => onchanged(await setSheetGeneralAccess(company, sheet, value)),
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
	people={shared}
	{directory}
	canManage={sharing.may_manage}
	{busy}
	{failure}
	{link}
	ongeneral={setGeneral}
	onadd={(actorId, access) =>
		void change(
			() => setSheetParticipant(company, sheet, actorId, access as 'read' | 'edit'),
			`${nameOf(actorId)} was not added.`
		)}
	onaccess={(actorId, access) =>
		void change(
			() => setSheetParticipant(company, sheet, actorId, access as 'read' | 'edit'),
			'Access was not changed.'
		)}
	onremove={(actorId) =>
		void change(
			() => setSheetParticipant(company, sheet, actorId, null),
			'Access was not removed.'
		)}
/>{/if}

<style>
	.quiet {
		margin: 4px;
		color: var(--text-tertiary);
	}
</style>
