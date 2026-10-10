<script lang="ts">
	/* Who can open a document, for its owner: the company as a whole (people read it), and anyone
	 * named here, Exec and Staff included, at view, comment or edit. Exec and Staff never see a
	 * document through company visibility; this is how one reaches them. */
	import { untrack } from 'svelte';
	import X from '@lucide/svelte/icons/x';
	import Toggle from '$lib/ui/page/Toggle.svelte';
	import { failureSentence } from '$lib/model/failure';
	import type { DocumentTarget } from '$lib/model/document-cache';
	import {
		getDocument,
		getDocumentParticipants,
		isDocumentConflict,
		removeDocumentParticipant,
		setDocumentParticipant,
		updateDocumentMetadata,
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

	const ACCESS: ReadonlyArray<{ value: DocumentAccess; label: string }> = [
		{ value: 'read', label: 'Can view' },
		{ value: 'comment', label: 'Can comment' },
		{ value: 'edit', label: 'Can edit' }
	];

	let participants = $state<DocumentParticipantRow[]>([]);
	let loaded = $state(false);
	let busy = $state(false);
	let failure = $state('');
	let person = $state('');
	let grant = $state<DocumentAccess>('comment');

	const documentId = $derived(view.document.id);
	const ownerId = $derived(view.document.owner_actor_id);
	const nameOf = (actorId: string) =>
		people.find((candidate) => candidate.actor_id === actorId)?.display ?? actorId;
	const shared = $derived(participants.filter((row) => row.actor_id !== ownerId));
	const candidates = $derived(
		people.filter(
			(candidate) =>
				candidate.kind !== 'system' &&
				candidate.actor_id !== ownerId &&
				!participants.some((row) => row.actor_id === candidate.actor_id)
		)
	);

	async function load(target: DocumentTarget): Promise<void> {
		const rows: DocumentParticipantRow[] = [];
		let cursor: DocumentParticipantCursor | null = null;
		for (let page = 0; page < 10; page += 1) {
			const result = await getDocumentParticipants(target.companyId, target.documentId, cursor);
			rows.push(...result.items);
			cursor = result.next_cursor;
			if (!cursor) break;
		}
		if (target.companyId !== companyId || target.documentId !== documentId) return;
		participants = rows;
		loaded = true;
	}

	$effect(() => {
		const target = { companyId, documentId };
		untrack(() => {
			participants = [];
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
	): Promise<void> {
		if (busy) return;
		busy = true;
		failure = '';
		const target = { companyId, documentId };
		try {
			await run(view.document.version, crypto.randomUUID());
		} catch (cause) {
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
	}

	function share(event: SubmitEvent): void {
		event.preventDefault();
		const actorId = person;
		const access = grant;
		if (!actorId) return;
		person = '';
		void change(
			(version, commandId) =>
				setDocumentParticipant(companyId, documentId, actorId, version, access, commandId),
			'It was not shared.'
		);
	}

	function setAccess(row: DocumentParticipantRow, access: DocumentAccess): void {
		participants = participants.map((item) =>
			item.actor_id === row.actor_id ? { ...item, access } : item
		);
		void change(
			(version, commandId) =>
				setDocumentParticipant(companyId, documentId, row.actor_id, version, access, commandId),
			'Access was not changed.'
		);
	}

	function remove(row: DocumentParticipantRow): void {
		participants = participants.filter((item) => item.actor_id !== row.actor_id);
		void change(
			(version, commandId) =>
				removeDocumentParticipant(companyId, documentId, row.actor_id, version, commandId),
			'Access was not removed.'
		);
	}

	function setCompanyVisible(visible: boolean): void {
		const source = view.document;
		void change(
			(version, commandId) =>
				updateDocumentMetadata(
					companyId,
					documentId,
					{
						expected_version: version,
						title: source.title,
						kind: source.kind,
						visibility: visible ? 'company' : 'participants',
						linked_room_id: source.linked_room_id,
						inherit_room_visibility: visible ? false : source.inherit_room_visibility
					},
					commandId
				),
			'Visibility was not changed.'
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
		<div class="visibility">
			<span
				title="Everyone in the company can open and read it. Exec and Staff see a document only once it is shared with them below."
				>Company can read</span
			>
			<Toggle
				checked={view.document.visibility === 'company'}
				label="Everyone in the company can read it"
				disabled={busy}
				onchange={setCompanyVisible}
			/>
		</div>

		<form class="add" onsubmit={share}>
			<select bind:value={person} aria-label="Person" disabled={busy || !candidates.length}>
				<option value="">{candidates.length ? 'Add someone' : 'Everyone is added'}</option>
				{#each candidates as candidate (candidate.actor_id)}
					<option value={candidate.actor_id}>{candidate.display}</option>
				{/each}
			</select>
			<select bind:value={grant} aria-label="Access" disabled={busy}>
				{#each ACCESS as option (option.value)}
					<option value={option.value}>{option.label}</option>
				{/each}
			</select>
			<button type="submit" class="btn small primary" disabled={busy || !person}>Share</button>
		</form>

		{#if failure}<p class="share-error" role="alert">{failure}</p>{/if}

		<ul class="people" aria-label="People with access" aria-busy={!loaded}>
			<li>
				<span class="name">{nameOf(ownerId)}</span>
				<span class="fixed" title="The owner can always edit, share and rename it">Owner</span>
			</li>
			{#each shared as row (row.actor_id)}
				<li>
					<span class="name">{nameOf(row.actor_id)}</span>
					<select
						value={row.access}
						aria-label={`Access for ${nameOf(row.actor_id)}`}
						disabled={busy}
						onchange={(event) => setAccess(row, event.currentTarget.value as DocumentAccess)}
					>
						{#each ACCESS as option (option.value)}
							<option value={option.value}>{option.label}</option>
						{/each}
					</select>
					<button
						type="button"
						class="close"
						aria-label={`Remove ${nameOf(row.actor_id)}`}
						title="Remove access"
						disabled={busy}
						onclick={() => remove(row)}><X size={14} strokeWidth={2} /></button
					>
				</li>
			{/each}
		</ul>
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
	.close:hover:not(:disabled) {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.close:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}
	.share-body {
		display: grid;
		align-content: start;
		gap: 14px;
		min-height: 0;
		overflow: auto;
		padding: 14px 12px;
	}
	.visibility {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		color: var(--text-secondary);
	}
	.add {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		gap: 6px;
	}
	select {
		min-width: 0;
		height: 30px;
		padding: 0 6px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
	}
	.share-error {
		margin: 0;
		padding: 8px 10px;
		border-radius: var(--radius-control);
		background: var(--state-danger-soft);
		color: var(--state-danger);
	}
	.people {
		display: grid;
		margin: 0;
		padding: 0;
		list-style: none;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.people li {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		align-items: center;
		gap: 6px;
		min-height: 44px;
		padding: 6px 8px 6px 12px;
	}
	.people li + li {
		border-top: 1px solid var(--border);
	}
	.people li:first-child {
		grid-template-columns: minmax(0, 1fr) auto;
	}
	.name {
		min-width: 0;
		overflow: hidden;
		color: var(--ink);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.fixed {
		color: var(--text-tertiary);
	}
</style>
