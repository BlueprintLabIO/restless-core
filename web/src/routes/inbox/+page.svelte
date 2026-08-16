<script lang="ts">
	/**
	 * The Inbox page owns the calls; the surface owns the rendering.
	 *
	 * Writes live here rather than in the surface for the reason every write in
	 * this product does: a component that fetched for itself would be a second
	 * write path, and authority belongs to the caller.
	 */
	import AppShell from '$lib/components/AppShell.svelte';
	import InboxSurface from '$lib/surfaces/InboxSurface.svelte';
	import { approvalAction, issueDesktopTicket, signIn } from '$lib/api/attention';
	import {
		attention,
		refreshAttention,
		reportAttentionFailure
	} from '$lib/model/attention.svelte';
	import { company, getInbox, type ApiMessage, type Outcome } from '$lib/api/client';
	import type { AttentionItem } from '$lib/model/view';

	let messages = $state<ApiMessage[]>([]);
	let mail = $state<Outcome<unknown>>({ state: 'ok', data: null });
	let busy = $state<string | null>(null);

	/** Stable per tab. The browser lease is held by a client, not by a person. */
	const clientId = crypto.randomUUID();

	// The queue itself is read by the shell, so the nav count and this stack are
	// the same answer. Mail is this page's own, and separate.
	$effect(() => {
		let cancelled = false;
		getInbox().then((inbox) => {
			if (cancelled) return;
			// An empty inbox comes back as ok-with-[]; the client only calls it a
			// stub when data is genuinely null.
			if (inbox.state === 'ok') {
				messages = inbox.data;
				mail = { state: 'ok', data: null };
			} else if (inbox.state === 'stub') {
				messages = [];
				mail = { state: 'ok', data: null };
			} else {
				mail = inbox;
			}
		});
		return () => {
			cancelled = true;
		};
	});

	/**
	 * Resolve an approval at its source. The party comes from the item id, which
	 * `attention.rs` builds as `authority:approval:<capability>:<party>` — the
	 * projection deliberately holds no separate party field, because that would
	 * be a second copy of something Authority owns.
	 */
	async function act(item: AttentionItem, actionId: string) {
		if (actionId !== 'grant' && actionId !== 'decline') return;
		const party = item.id.split(':').slice(3).join(':');
		if (!party) return;
		busy = item.id;
		const failed = await approvalAction(company(), actionId, party);
		busy = null;
		if (failed) {
			reportAttentionFailure(failed);
			return;
		}
		// Re-read rather than removing the card locally. Granting a party can
		// resolve more than the item you clicked, and a queue that guessed which
		// ones would be wrong in the owner's favour.
		await refreshAttention();
	}

	/** The prepared last mile: hand the owner the company's own browser session. */
	async function openBrowser(item: AttentionItem) {
		busy = item.id;
		const ticket = await issueDesktopTicket(company(), item.id, clientId);
		busy = null;
		if ('error' in ticket) {
			reportAttentionFailure(ticket.error);
			return;
		}
		window.open(ticket.url, '_blank', 'noopener');
	}

	async function authenticate(token: string) {
		const failed = await signIn(token);
		if (failed) {
			reportAttentionFailure(failed);
			return;
		}
		await refreshAttention();
	}
</script>

<svelte:head><title>Inbox</title></svelte:head>

<AppShell surface="inbox">
	<InboxSurface
		attention={attention()}
		{messages}
		{mail}
		{busy}
		onAct={act}
		onOpenBrowser={openBrowser}
		onSignIn={authenticate}
		onReload={refreshAttention}
	/>
</AppShell>
