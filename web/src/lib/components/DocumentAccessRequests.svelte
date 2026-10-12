<script lang="ts">
	/* Who has asked for this document, above its sharing, for the people who manage it: grant at
	 * the level they asked for, or set the ask aside. Nobody else sees asks, so nothing shows for
	 * them. */
	import { untrack } from 'svelte';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import {
		dismissDocumentAccessRequest,
		getDocument,
		getDocumentAccessRequests,
		setDocumentParticipant,
		type DocumentAccess,
		type DocumentAccessRequest,
		type DocumentReadView
	} from '$lib/model/documents';
	import type { DocumentTarget } from '$lib/model/document-cache';

	let {
		companyId,
		view,
		nameOf,
		ongranted
	}: {
		companyId: string;
		view: DocumentReadView;
		nameOf: (actorId: string) => string;
		/** Access changed: the page takes the document's new version and redraws sharing. */
		ongranted: (target: DocumentTarget, view: DocumentReadView) => void;
	} = $props();

	const WANTS: Record<DocumentAccess, string> = {
		read: 'asked to open it',
		comment: 'asked to comment',
		edit: 'asked to edit'
	};
	const GRANTS: Record<DocumentAccess, string> = {
		read: 'Let them view',
		comment: 'Let them comment',
		edit: 'Let them edit'
	};

	let asks = $state<DocumentAccessRequest[]>([]);
	let busy = $state('');
	const documentId = $derived(view.document.id);

	$effect(() => {
		const target = { companyId, documentId };
		untrack(() => {
			asks = [];
			/* Someone who does not manage sharing is refused; they simply see no asks. */
			void getDocumentAccessRequests(target.companyId, target.documentId)
				.then((items) => {
					if (target.companyId === companyId && target.documentId === documentId) asks = items;
				})
				.catch(() => undefined);
		});
	});

	async function grant(ask: DocumentAccessRequest): Promise<void> {
		if (busy) return;
		busy = ask.id;
		const target = { companyId, documentId };
		try {
			await setDocumentParticipant(
				target.companyId,
				target.documentId,
				ask.actor_id,
				view.document.version,
				ask.access,
				crypto.randomUUID()
			);
			asks = asks.filter((item) => item.id !== ask.id);
			toast(`${nameOf(ask.actor_id)} can ${ask.access === 'read' ? 'view' : ask.access} it now`);
			ongranted(target, await getDocument(target.companyId, target.documentId));
		} catch (cause) {
			toast(failureSentence(cause, 'Access was not given. Try again.'), { tone: 'failure' });
		} finally {
			busy = '';
		}
	}

	async function dismiss(ask: DocumentAccessRequest): Promise<void> {
		if (busy) return;
		busy = ask.id;
		try {
			await dismissDocumentAccessRequest(companyId, documentId, ask.id);
			asks = asks.filter((item) => item.id !== ask.id);
		} catch (cause) {
			toast(failureSentence(cause, 'That did not change. Try again.'), { tone: 'failure' });
		} finally {
			busy = '';
		}
	}
</script>

{#if asks.length}
	<section class="asks" aria-label="Asked for access">
		{#each asks as ask (ask.id)}
			<div class="ask">
				<p>
					<strong>{nameOf(ask.actor_id)}</strong>
					{WANTS[ask.access]}
					<RelativeTime value={ask.requested_at} />
				</p>
				<div class="ask-actions">
					<button
						type="button"
						class="btn small primary"
						disabled={Boolean(busy)}
						onclick={() => void grant(ask)}>{GRANTS[ask.access]}</button
					>
					<button
						type="button"
						class="btn small ghost"
						title="Set the ask aside. They can ask again tomorrow."
						disabled={Boolean(busy)}
						onclick={() => void dismiss(ask)}>Dismiss</button
					>
				</div>
			</div>
		{/each}
	</section>
{/if}

<style>
	.asks {
		display: grid;
		gap: 8px;
		padding: 12px 12px 0;
	}
	.ask {
		display: grid;
		gap: 8px;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.ask p {
		margin: 0;
		color: var(--text-secondary);
	}
	.ask strong {
		color: var(--ink);
		font-weight: 600;
	}
	.ask-actions {
		display: flex;
		gap: 6px;
	}
</style>
