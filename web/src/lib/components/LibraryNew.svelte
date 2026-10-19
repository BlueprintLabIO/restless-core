<script lang="ts">
	/* Starting a document or sheet from the Library, as Notion does: it is made untitled at once and
	 * opens in its editor with the title ready to type. There is no naming step. A retried document
	 * create reuses its command, and a retried sheet create reuses its id, so a lost response never
	 * makes a duplicate.
	 *
	 * Adding files lives here too: picked or dropped files, whole folders and the files inside a
	 * zip go to the company as they are, with no message, and land in the Library under From you.
	 * A folder or zip becomes a collection. Files go three at a time, each with its own progress,
	 * in a panel that stays until the owner closes it. */
	import { goto } from '$app/navigation';
	import { useQueryClient } from '@tanstack/svelte-query';
	import X from '@lucide/svelte/icons/x';
	import { failureSentence, type OwnerFailure } from '$lib/model/failure';
	import { documentQueryKeys } from '$lib/model/document-queries.svelte';
	import { failClosedDocumentRead } from '$lib/model/document-cache';
	import {
		createDocument,
		getDocument,
		isRetryableDocumentFailure,
		pendingDocumentCreation,
		DOCUMENT_KINDS,
		type DocumentKind,
		type PendingDocumentCreation
	} from '$lib/model/documents';
	import { createSheet } from '$lib/model/sheets';
	import {
		MAX_UPLOAD_BYTES,
		uploadAttachments,
		type UploadedAttachment
	} from '$lib/model/attachments';
	import {
		droppedEntries,
		expandZips,
		itemsFromEntries,
		itemsFromPicked,
		type UploadItem
	} from '$lib/model/library-upload';
	import { setLibraryCollection } from '$lib/model/library-items';
	import Pending from '$lib/ui/feedback/Pending.svelte';
	import { importOfficeFile, importedHref } from '$lib/model/office-import';

	type Kind = 'doc' | 'sheet';
	let {
		companyId,
		onuploaded
	}: {
		companyId: string;
		/** Files the company now keeps, after any upload that added at least one. */
		onuploaded?: (added: UploadedAttachment[]) => void;
	} = $props();
	const client = useQueryClient();

	let busy = $state(false);
	let failure = $state<{
		kind: Kind | 'upload' | 'import';
		message: string;
		retry: boolean;
	} | null>(null);
	let documentAttempt: PendingDocumentCreation | null = null;
	let documentKind: DocumentKind = 'freeform';
	let templates: HTMLDialogElement | undefined = $state();
	let sheetId = '';
	let picker: HTMLInputElement | undefined = $state();
	let folderPicker: HTMLInputElement | undefined = $state();

	type Job = {
		id: number;
		file: File;
		collection: string | null;
		state: 'waiting' | 'sending' | 'done' | 'failed' | 'cancelled';
		sent: number;
		message?: string;
		added?: UploadedAttachment;
	};
	let jobs = $state<Job[]>([]);
	let uploadNotes = $state<string[]>([]);
	let showJobs = $state(false);
	let preparing = $state(false);
	let nextJob = 0;
	let running = false;
	const SIMULTANEOUS = 3;
	const totals = $derived.by(() => {
		const size = jobs.reduce((sum, job) => sum + job.file.size, 0);
		const sent = jobs.reduce(
			(sum, job) => sum + (job.state === 'done' ? job.file.size : job.sent),
			0
		);
		const count = (state: Job['state']) => jobs.filter((job) => job.state === state).length;
		return {
			size,
			sent,
			done: count('done'),
			failed: count('failed'),
			active: count('waiting') + count('sending'),
			cancelled: count('cancelled')
		};
	});
	const megabytes = (bytes: number) =>
		bytes >= 1024 * 1024
			? `${(bytes / (1024 * 1024)).toFixed(1)} MB`
			: `${Math.max(1, Math.round(bytes / 1024))} KB`;
	let importPicker: HTMLInputElement | undefined = $state();
	let importing = $state('');
	let importFailed: File | null = null;

	const UNTITLED: Record<Kind, string> = { doc: 'Untitled document', sheet: 'Untitled sheet' };

	function editorHref(company: string, next: Kind, id: string) {
		const root = `/${encodeURIComponent(company)}/library`;
		return next === 'doc'
			? `${root}/documents?document=${encodeURIComponent(id)}&new=1`
			: `${root}/sheets?sheet=${encodeURIComponent(id)}&new=1`;
	}

	/** Offer the kinds of document, each with headings to start from. */
	export function chooseTemplate() {
		templates?.showModal();
	}

	/** Make one untitled document (blank, or a kind's headings) or sheet and open it. */
	export async function open(target: Kind, retry = false, kind: DocumentKind = 'freeform') {
		if (busy) return;
		if (!retry) {
			documentAttempt = null;
			sheetId = '';
			documentKind = kind;
		}
		const company = companyId;
		busy = true;
		failure = null;
		let createdId: string | null = null;
		try {
			if (target === 'doc') {
				const attempt = pendingDocumentCreation(documentAttempt, {
					title: UNTITLED.doc,
					kind: documentKind,
					visibility: 'company',
					linked_room_id: null,
					inherit_room_visibility: false,
					reason: 'Created'
				});
				documentAttempt = attempt;
				const receipt = await createDocument(company, attempt.input, attempt.command.id);
				createdId = receipt.document_id;
				const created = await getDocument(company, receipt.document_id);
				client.setQueryData(documentQueryKeys.detail(company, receipt.document_id), created);
				await client.invalidateQueries({ queryKey: documentQueryKeys.list(company) });
			} else {
				sheetId ||= crypto.randomUUID();
				createdId = (await createSheet(company, sheetId, UNTITLED.sheet)).id;
			}
			if (company !== companyId) return;
			documentAttempt = null;
			sheetId = '';
			await goto(editorHref(company, target, createdId));
		} catch (cause) {
			if (company !== companyId) return;
			if (target === 'doc') {
				failClosedDocumentRead(client, cause, company, createdId);
				if (!isRetryableDocumentFailure(cause)) documentAttempt = null;
			}
			failure = {
				kind: target,
				retry: true,
				message: failureSentence(
					cause,
					`The ${target === 'doc' ? 'document' : 'sheet'} was not created.`
				)
			};
		} finally {
			busy = false;
		}
	}

	/** Choose a Word, Excel or CSV file to make into a document or sheet. */
	export function pickImport() {
		importPicker?.click();
	}

	/** Make a Word, Excel or CSV file an editable document or sheet, and open it. */
	export async function importFile(file: File) {
		if (importing) return;
		const company = companyId;
		importing = file.name;
		failure = null;
		try {
			const imported = await importOfficeFile(company, file);
			importFailed = null;
			if (imported.kind === 'doc')
				await client.invalidateQueries({ queryKey: documentQueryKeys.list(company) });
			if (company === companyId) await goto(importedHref(company, imported));
		} catch (cause) {
			importFailed = file;
			if (company === companyId)
				failure = {
					kind: 'import',
					retry: true,
					message: failureSentence(cause, `${file.name} was not imported.`)
				};
		} finally {
			importing = '';
		}
	}

	/** Choose files to add from this device. */
	export function pick() {
		picker?.click();
	}

	/** Choose a whole folder; it becomes a collection. */
	export function pickFolder() {
		folderPicker?.click();
	}

	/** Files dropped on the Library, folders included. Read the entries before the event ends. */
	export function drop(transfer: DataTransfer | null) {
		const entries = droppedEntries(transfer);
		if (entries.length) void itemsFromEntries(entries).then((items) => queue(items));
		else void upload([...(transfer?.files ?? [])]);
	}

	const names = (files: File[]) =>
		files.length === 1 ? `“${files[0].name}” is` : `${files.length} files are`;

	/** Add files, folders' files and zips' contents to the queue and start sending. */
	export async function queue(items: UploadItem[]) {
		if (!items.length) return;
		preparing = true;
		const { items: expanded, notes } = await expandZips(items).finally(() => (preparing = false));
		const tooLarge = expanded.filter((item) => item.file.size > MAX_UPLOAD_BYTES);
		uploadNotes = [
			...notes,
			...(tooLarge.length
				? [
						`${names(tooLarge.map((item) => item.file))} larger than ${MAX_UPLOAD_BYTES / (1024 * 1024)} MB, the most one file can be, and ${tooLarge.length === 1 ? 'was' : 'were'} left out.`
					]
				: [])
		];
		const fits = expanded.filter((item) => item.file.size <= MAX_UPLOAD_BYTES);
		// Finished files from an earlier batch make way; a run in progress keeps its rows.
		if (!running) jobs = jobs.filter((job) => job.state !== 'done' && job.state !== 'cancelled');
		jobs.push(
			...fits.map((item) => ({
				id: nextJob++,
				file: item.file,
				collection: item.collection,
				state: 'waiting' as const,
				sent: 0
			}))
		);
		if (fits.length > SIMULTANEOUS) showJobs = true;
		void run();
	}

	async function send(job: Job, company: string) {
		job.state = 'sending';
		try {
			const [added] = await uploadAttachments(company, [job.file], (fraction) => {
				job.sent = Math.round(fraction * job.file.size);
			});
			job.added = added;
			job.state = 'done';
		} catch (cause) {
			const said = cause as OwnerFailure;
			job.state = 'failed';
			job.message =
				said.status === 503 && said.serverMessage
					? said.serverMessage
					: failureSentence(cause, 'Not added.');
			// A full Library refuses every file after this one too: stop rather than fail each.
			if (said.status === 409 || said.status === 413)
				for (const other of jobs)
					if (other.state === 'waiting') {
						other.state = 'failed';
						other.message = job.message;
					}
		}
	}

	async function run() {
		if (running) return;
		running = true;
		const company = companyId;
		const before = new Set(jobs.filter((job) => job.state === 'done').map((job) => job.id));
		const worker = async () => {
			for (;;) {
				const job = jobs.find((candidate) => candidate.state === 'waiting');
				if (!job || company !== companyId) return;
				await send(job, company);
			}
		};
		await Promise.all(Array.from({ length: SIMULTANEOUS }, worker));
		running = false;
		const finished = jobs.filter((job) => job.state === 'done' && !before.has(job.id) && job.added);
		const added = finished.map((job) => job.added!);
		// A folder's or zip's files share its collection.
		const byCollection = new Map<string, string[]>();
		for (const job of finished)
			if (job.collection)
				byCollection.set(job.collection, [
					...(byCollection.get(job.collection) ?? []),
					`upload:${job.added!.id}`
				]);
		for (const [collection, keys] of byCollection)
			await setLibraryCollection(company, keys, collection).catch(() => {
				uploadNotes = [...uploadNotes, `The files were added, but not put in “${collection}”.`];
			});
		if (added.length && company === companyId) onuploaded?.(added);
		if (!totals.failed && !uploadNotes.length && !showJobs) jobs = [];
	}

	function cancelWaiting() {
		for (const job of jobs) if (job.state === 'waiting') job.state = 'cancelled';
	}

	function retryFailed() {
		for (const job of jobs)
			if (job.state === 'failed') {
				job.state = 'waiting';
				job.sent = 0;
				job.message = undefined;
			}
		void run();
	}

	function closeUploads() {
		cancelWaiting();
		jobs = jobs.filter((job) => job.state === 'sending');
		uploadNotes = [];
		showJobs = false;
	}

	/** Add files to the company as they are; zips open into their files. */
	export async function upload(files: File[]) {
		await queue(files.map((file) => ({ file, collection: null })));
	}

	function retry() {
		if (!failure) return;
		if (failure.kind === 'upload') retryFailed();
		else if (failure.kind === 'import') {
			if (importFailed) void importFile(importFailed);
		} else void open(failure.kind, true, documentKind);
	}
</script>

<input
	bind:this={picker}
	class="library-new-picker"
	type="file"
	multiple
	tabindex="-1"
	aria-hidden="true"
	onchange={(event) => {
		const files = [...(event.currentTarget.files ?? [])];
		event.currentTarget.value = '';
		void upload(files);
	}}
/>

<input
	bind:this={folderPicker}
	class="library-new-picker"
	type="file"
	webkitdirectory
	multiple
	tabindex="-1"
	aria-hidden="true"
	onchange={(event) => {
		const files = [...(event.currentTarget.files ?? [])];
		event.currentTarget.value = '';
		void queue(itemsFromPicked(files));
	}}
/>

<input
	bind:this={importPicker}
	class="library-new-picker"
	type="file"
	accept=".docx,.xlsx,.csv,application/vnd.openxmlformats-officedocument.wordprocessingml.document,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,text/csv"
	tabindex="-1"
	aria-hidden="true"
	onchange={(event) => {
		const file = event.currentTarget.files?.[0];
		event.currentTarget.value = '';
		if (file) void importFile(file);
	}}
/>

<dialog bind:this={templates} class="templates" aria-labelledby="templates-title">
	<header>
		<h2 id="templates-title">New document</h2>
		<button
			type="button"
			class="close"
			aria-label="Close"
			title="Close"
			onclick={() => templates?.close()}><X size={14} strokeWidth={2} /></button
		>
	</header>
	<div class="template-list">
		{#each DOCUMENT_KINDS as entry (entry.kind)}
			<button
				type="button"
				title={entry.headings.length
					? `Starts with: ${entry.headings.join(', ')}`
					: 'An empty page'}
				disabled={busy}
				onclick={() => {
					templates?.close();
					void open('doc', false, entry.kind);
				}}
				><strong>{entry.label}</strong>{#if entry.headings.length}<span
						>{entry.headings.join(' · ')}</span
					>{/if}</button
			>
		{/each}
	</div>
</dialog>

{#if importing}
	<div class="library-new-failure uploading">
		<Pending label={`Importing ${importing}`} />
	</div>
{:else if preparing}
	<div class="library-new-failure uploading">
		<Pending label="Opening the files" />
	</div>
{:else if failure}
	<div class="library-new-failure" role="alert">
		<span>{failure.message}</span>
		{#if failure.retry}<button type="button" class="btn small" disabled={busy} onclick={retry}
				>Try again</button
			>{/if}
		<button
			type="button"
			class="close"
			aria-label="Dismiss"
			title="Dismiss"
			onclick={() => (failure = null)}><X size={14} strokeWidth={2} /></button
		>
	</div>
{/if}

{#if jobs.length || uploadNotes.length}
	<section class="upload-panel" aria-label="Uploads">
		<header>
			<span class="upload-summary" role="status">
				{#if totals.active}
					{totals.done} of {jobs.length - totals.cancelled}
					{jobs.length === 1 ? 'file' : 'files'} added · {megabytes(totals.sent)} of {megabytes(
						totals.size
					)}
				{:else if jobs.length}
					Added {totals.done} of {jobs.length - totals.cancelled}
					{jobs.length === 1 ? 'file' : 'files'}{totals.failed
						? ` · ${totals.failed} not added`
						: ''}
				{:else}
					Upload notes
				{/if}
			</span>
			{#if jobs.length > 1}<button
					type="button"
					class="btn small"
					aria-expanded={showJobs}
					onclick={() => (showJobs = !showJobs)}>Files</button
				>{/if}
			{#if totals.failed && !totals.active}<button
					type="button"
					class="btn small"
					onclick={retryFailed}>Try again</button
				>{/if}
			{#if totals.active}<button
					type="button"
					class="btn small"
					title="Stop the files not yet sent; files on their way still arrive"
					onclick={cancelWaiting}>Cancel</button
				>{:else}<button
					type="button"
					class="close"
					aria-label="Close"
					title="Close"
					onclick={closeUploads}><X size={14} strokeWidth={2} /></button
				>{/if}
		</header>
		{#if totals.size}
			<progress max={totals.size} value={totals.sent} aria-label="Upload progress"></progress>
		{/if}
		{#each uploadNotes as note (note)}<p class="upload-note">{note}</p>{/each}
		{#if showJobs}
			<ul class="upload-jobs">
				{#each jobs as job (job.id)}
					<li class={job.state}>
						<span
							class="upload-name"
							title={job.collection ? `${job.collection} › ${job.file.name}` : job.file.name}
							>{job.file.name}</span
						>
						<span class="upload-state"
							>{job.state === 'sending'
								? `${Math.round((100 * job.sent) / Math.max(1, job.file.size))}%`
								: job.state === 'done'
									? 'Added'
									: job.state === 'failed'
										? (job.message ?? 'Not added')
										: job.state === 'cancelled'
											? 'Cancelled'
											: megabytes(job.file.size)}</span
						>
					</li>
				{/each}
			</ul>
		{/if}
	</section>
{/if}

<style>
	/* Floats like the other Library notices, so uploads stay in view while the list changes. */
	.upload-panel {
		position: fixed;
		bottom: 20px;
		left: 50%;
		z-index: 40;
		display: grid;
		gap: 8px;
		width: min(440px, calc(100vw - 32px));
		padding: 10px 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-float, var(--shadow-soft));
		font-size: var(--t-label);
		transform: translateX(-50%);
	}
	.upload-panel header {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.upload-summary {
		flex: 1;
		min-width: 0;
		color: var(--ink);
		font-weight: 500;
	}
	.upload-panel progress {
		width: 100%;
		height: 4px;
		accent-color: var(--intent-conversation);
	}
	.upload-note {
		margin: 0;
		color: var(--text-secondary);
	}
	.upload-jobs {
		display: grid;
		gap: 2px;
		max-height: 220px;
		margin: 0;
		padding: 0;
		overflow-y: auto;
		list-style: none;
	}
	.upload-jobs li {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		padding: 3px 0;
	}
	.upload-name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.upload-state {
		flex: none;
		max-width: 60%;
		overflow: hidden;
		color: var(--text-tertiary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.upload-jobs li.done .upload-state {
		color: var(--state-success);
	}
	.upload-jobs li.failed .upload-state {
		color: var(--state-danger, var(--text-secondary));
	}
	.library-new-picker {
		display: none;
	}
	.templates {
		width: min(420px, calc(100vw - 32px));
		padding: 0;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		color: var(--ink);
		box-shadow: var(--shadow-float, var(--shadow-soft));
	}
	.templates::backdrop {
		background: #0f172a40;
	}
	.templates header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 12px 4px 16px;
	}
	.templates h2 {
		margin: 0;
		font-size: var(--t-body);
		font-weight: 600;
	}
	.template-list {
		display: grid;
		padding: 4px 8px 10px;
	}
	.template-list button {
		display: grid;
		gap: 2px;
		padding: 9px 10px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: inherit;
		text-align: left;
		font: 500 var(--t-body) var(--font-ui);
		cursor: pointer;
	}
	.template-list button:hover:not(:disabled),
	.template-list button:focus-visible {
		background: var(--wash-hover);
	}
	.template-list span {
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.library-new-failure {
		position: fixed;
		bottom: 20px;
		left: 50%;
		z-index: 40;
		display: flex;
		align-items: center;
		gap: 10px;
		max-width: min(520px, calc(100vw - 32px));
		padding: 8px 8px 8px 14px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-float, var(--shadow-soft));
		color: var(--ink);
		font-size: var(--t-body);
		transform: translateX(-50%);
	}
	.library-new-failure.uploading {
		padding: 10px 14px;
	}
	.library-new-failure span {
		flex: 1;
	}
	.close {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
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
</style>
