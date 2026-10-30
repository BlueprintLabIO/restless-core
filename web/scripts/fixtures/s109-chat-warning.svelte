<script lang="ts">
	import ExecutiveRail from '$lib/components/ExecutiveRail.svelte';
	import { onMount } from 'svelte';
	let hydrated = $state(false);
	onMount(() => {
		hydrated = true;
	});
	let broken = $state(false);
	let fails = $state(true);
	let retries = $state(0);
	const messages = [
		{
			id: '1',
			from: 'agent' as const,
			author: 'Exec',
			text: 'A stable message.',
			createdAt: '2026-10-30T00:00:00Z',
			replyToMessageId: null,
			assetId: null,
			runId: null,
			attachments: []
		}
	];
	async function send() {
		return fails
			? {
					error: 'Could not confirm the send. Your draft is still here.',
					retryCommandId: 'fixture'
				}
			: {};
	}
</script>

<div class="fixture-controls" data-ready={hydrated} data-broken={broken}>
	<button onclick={() => (broken = true)}>Break conversation</button><button
		onclick={() => (broken = false)}>Restore conversation</button
	><button onclick={() => (fails = false)}>Allow send</button><span data-retries>{retries}</span>
</div>
<div class="fixture-rail">
	<ExecutiveRail
		companyId="fixture_test"
		membershipRole="owner"
		connected={true}
		connectionStatus={broken ? 'error' : 'available'}
		conversationStatus="live"
		{messages}
		onrefreshConversation={() => retries++}
		onask={send}
	/>
</div>

<style>
	.fixture-rail {
		height: 700px;
		width: min(420px, 100%);
	}
</style>
