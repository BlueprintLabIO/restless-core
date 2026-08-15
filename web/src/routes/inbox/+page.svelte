<script lang="ts">
	import AppShell from '$lib/components/AppShell.svelte';
	import InboxSurface from '$lib/surfaces/InboxSurface.svelte';
	import { getAttention, getInbox, type ApiMessage, type Outcome } from '$lib/api/client';

	let messages = $state<ApiMessage[]>([]);
	let outcome = $state<Outcome<unknown>>({ state: 'ok', data: null });
	let attention = $state<Outcome<unknown>>({ state: 'ok', data: null });

	let reloads = $state(0);

	$effect(() => {
		reloads;
		let cancelled = false;
		(async () => {
			const [mail, merged] = await Promise.all([getInbox(), getAttention()]);
			if (cancelled) return;
			attention = merged;
			// An empty inbox comes back as ok-with-[] from the daemon; the client
			// only calls it a stub when data is genuinely null.
			if (mail.state === 'ok') {
				messages = mail.data;
				outcome = { state: 'ok', data: null };
			} else if (mail.state === 'stub') {
				messages = [];
				outcome = { state: 'ok', data: null };
			} else {
				outcome = mail;
			}
		})();
		return () => {
			cancelled = true;
		};
	});
</script>

<svelte:head><title>Inbox</title></svelte:head>

<AppShell surface="inbox">
	<InboxSurface {messages} {outcome} {attention} onSeen={() => (reloads += 1)} />
</AppShell>
