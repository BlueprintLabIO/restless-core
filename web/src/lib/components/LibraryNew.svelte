<script lang="ts">
	/* Starting a document or sheet from the Library, as Notion does: it is made untitled at once and
	 * opens in its editor with the title ready to type. There is no naming step. A retried document
	 * create reuses its command, and a retried sheet create reuses its id, so a lost response never
	 * makes a duplicate. */
	import { goto } from '$app/navigation';
	import { useQueryClient } from '@tanstack/svelte-query';
	import X from '@lucide/svelte/icons/x';
	import { failureSentence } from '$lib/model/failure';
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

	type Kind = 'doc' | 'sheet';
	let { companyId }: { companyId: string } = $props();
	const client = useQueryClient();

	let busy = $state(false);
	let failure = $state<{ kind: Kind; message: string } | null>(null);
	let documentAttempt: PendingDocumentCreation | null = null;
	let sheetId = '';

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
				message: failureSentence(
					cause,
					`The ${target === 'doc' ? 'document' : 'sheet'} was not created.`
				)
			};
		} finally {
			busy = false;
		}
	}
</script>

{#if failure}
	<div class="library-new-failure" role="alert">
		<span>{failure.message}</span>
		<button
			type="button"
			class="btn small"
			disabled={busy}
			onclick={() => open(failure!.kind, true)}>Try again</button
		>
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
