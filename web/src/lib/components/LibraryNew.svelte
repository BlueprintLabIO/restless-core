<script lang="ts">
	/* Starting a document or sheet from the Library: name it, then land in its
	 * editor. A retried document create reuses its command, and a retried sheet
	 * create reuses its id, so a lost response never makes a duplicate. */
	import { goto } from '$app/navigation';
	import { useQueryClient } from '@tanstack/svelte-query';
	import X from '@lucide/svelte/icons/x';
	import { Segmented } from '$lib/ui/page';
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

	let dialog: HTMLDialogElement;
	let kind = $state<Kind>('doc');
	let title = $state('');
	let busy = $state(false);
	let failure = $state('');
	let documentAttempt: PendingDocumentCreation | null = null;
	let sheetId = '';

	export function open(next: Kind) {
		kind = next;
		title = '';
		failure = '';
		documentAttempt = null;
		sheetId = '';
		dialog.showModal();
	}

	function editorHref(company: string, next: Kind, id: string) {
		const root = `/${encodeURIComponent(company)}/library`;
		return next === 'doc'
			? `${root}/documents?document=${encodeURIComponent(id)}`
			: `${root}/sheets?sheet=${encodeURIComponent(id)}`;
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		const name = title.trim();
		if (!name || busy) return;
		const company = companyId;
		const target = kind;
		busy = true;
		failure = '';
		let createdId: string | null = null;
		try {
			if (target === 'doc') {
				const attempt = pendingDocumentCreation(documentAttempt, {
					title: name,
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
				createdId = (await createSheet(company, sheetId, name)).id;
			}
			if (company !== companyId) return;
			dialog.close();
			await goto(editorHref(company, target, createdId));
		} catch (cause) {
			if (company !== companyId) return;
			if (target === 'doc') {
				failClosedDocumentRead(client, cause, company, createdId);
				if (!isRetryableDocumentFailure(cause)) documentAttempt = null;
			}
			failure = failureSentence(
				cause,
				`The ${target === 'doc' ? 'document' : 'sheet'} was not created.`
			);
		} finally {
			busy = false;
		}
	}
</script>

<dialog
	bind:this={dialog}
	aria-labelledby="library-new-title"
	oncancel={(event) => {
		if (busy) event.preventDefault();
	}}
>
	<form onsubmit={submit}>
		<header>
			<h2 id="library-new-title">New {kind === 'doc' ? 'document' : 'sheet'}</h2>
			<button
				type="button"
				class="close"
				aria-label="Close"
				title="Close"
				disabled={busy}
				onclick={() => dialog.close()}><X size={16} strokeWidth={2} /></button
			>
		</header>
		<Segmented
			label="Kind"
			value={kind}
			disabled={busy}
			options={[
				{ value: 'doc', label: 'Document' },
				{ value: 'sheet', label: 'Sheet' }
			]}
			onchange={(value) => {
				kind = value;
				failure = '';
			}}
		/>
		<input
			aria-label="Title"
			bind:value={title}
			oninput={() => (documentAttempt = null)}
			placeholder={kind === 'doc' ? 'Untitled document' : 'Untitled sheet'}
			maxlength="200"
			disabled={busy}
			{@attach (input) => input.focus()}
		/>
		{#if failure}<p class="failure" role="alert">{failure}</p>{/if}
		<footer>
			<button type="button" class="btn small ghost" disabled={busy} onclick={() => dialog.close()}
				>Cancel</button
			>
			<button type="submit" class="btn small primary" disabled={busy || !title.trim()}
				>{busy ? 'Creating…' : 'Create'}</button
			>
		</footer>
	</form>
</dialog>

<style>
	dialog {
		width: min(420px, calc(100vw - 32px));
		padding: 0;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		color: var(--ink);
		box-shadow: var(--shadow-soft);
	}
	dialog[open] {
		animation: rise var(--motion-disclosure) var(--ease-out);
	}
	dialog::backdrop {
		background: color-mix(in srgb, var(--ink) 24%, transparent);
	}
	form {
		display: grid;
		gap: 14px;
		padding: 16px;
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	h2 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.close {
		display: grid;
		place-items: center;
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
	.failure {
		margin: 0;
		color: var(--state-danger);
	}
	footer {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
	@keyframes rise {
		from {
			opacity: 0;
			transform: translateY(4px) scale(0.99);
		}
	}
</style>
