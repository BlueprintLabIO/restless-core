<script lang="ts">
	/* Starting a document or sheet from the Library, as Notion does: it is made untitled at once and
	 * opens in its editor with the title ready to type. There is no naming step. A retried document
	 * create reuses its command, and a retried sheet create reuses its id, so a lost response never
	 * makes a duplicate.
	 *
	 * Adding files lives here too: picked or dropped files go to the company as they are, with no
	 * message, and land in the Library under From you. */
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
		type PendingDocumentCreation
	} from '$lib/model/documents';
	import { createSheet } from '$lib/model/sheets';
	import {
		MAX_UPLOAD_BYTES,
		uploadAttachments,
		type UploadedAttachment
	} from '$lib/model/attachments';
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
	let sheetId = '';
	let picker: HTMLInputElement | undefined = $state();
	let uploading = $state(0);
	let unsent: File[] = [];
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

	/** Make one untitled document or sheet and open it. */
	export async function open(target: Kind, retry = false) {
		if (busy) return;
		if (!retry) {
			documentAttempt = null;
			sheetId = '';
		}
		const company = companyId;
		busy = true;
		failure = null;
		let createdId: string | null = null;
		try {
			if (target === 'doc') {
				const attempt = pendingDocumentCreation(documentAttempt, {
					title: UNTITLED.doc,
					kind: 'freeform',
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

	const names = (files: File[]) =>
		files.length === 1 ? `“${files[0].name}” is` : `${files.length} files are`;

	/** Add files to the company as they are. A file over the per-file limit is named and left out;
	 * the rest go ahead. */
	export async function upload(files: File[], retry = false) {
		if (uploading || !files.length) return;
		const company = companyId;
		const fits = files.filter((file) => file.size <= MAX_UPLOAD_BYTES);
		const tooLarge = files.filter((file) => file.size > MAX_UPLOAD_BYTES);
		const tooLargeSentence = tooLarge.length
			? `${names(tooLarge)} larger than ${MAX_UPLOAD_BYTES / (1024 * 1024)} MB, the most one file can be.`
			: '';
		failure = null;
		if (!fits.length) {
			failure = { kind: 'upload', retry: false, message: tooLargeSentence };
			return;
		}
		uploading = fits.length;
		let added: UploadedAttachment[] = [];
		try {
			added = await uploadAttachments(company, fits);
			unsent = [];
			if (tooLargeSentence && company === companyId)
				failure = {
					kind: 'upload',
					retry: false,
					message: `Added ${added.length === 1 ? 'one file' : `${added.length} files`}. ${tooLargeSentence}`
				};
		} catch (cause) {
			added = (cause as { added?: UploadedAttachment[] }).added ?? [];
			unsent = fits.slice(added.length);
			if (company !== companyId) return;
			const said = cause as OwnerFailure;
			/* The computer being stopped, or other files still being stored, is already a sentence
			 * the owner can act on; other server failures keep the generic reading. */
			const sentence =
				said.status === 503 &&
				(said.code === 'runtime' || said.code === 'attachment_staging') &&
				said.serverMessage
					? said.serverMessage
					: failureSentence(
							cause,
							retry ? 'The files were still not added.' : 'The files were not added.'
						);
			failure = {
				kind: 'upload',
				retry: said.status !== 409 && said.status !== 413,
				message: [
					added.length ? `Added ${added.length} of ${fits.length}.` : '',
					sentence,
					tooLargeSentence
				]
					.filter(Boolean)
					.join(' ')
			};
		} finally {
			uploading = 0;
			if (added.length && company === companyId) onuploaded?.(added);
		}
	}

	function retry() {
		if (!failure) return;
		if (failure.kind === 'upload') void upload(unsent, true);
		else if (failure.kind === 'import') {
			if (importFailed) void importFile(importFailed);
		} else void open(failure.kind, true);
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

{#if importing}
	<div class="library-new-failure uploading">
		<Pending label={`Importing ${importing}`} />
	</div>
{:else if uploading}
	<div class="library-new-failure uploading">
		<Pending label={uploading === 1 ? 'Adding the file' : `Adding ${uploading} files`} />
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

<style>
	.library-new-picker {
		display: none;
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
