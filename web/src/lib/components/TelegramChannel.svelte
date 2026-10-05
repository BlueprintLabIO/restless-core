<script lang="ts">
	/* The owner's Telegram channel: store a bot token reference, pair one chat
	 * with a one-time code, unpair. Only the Authority owner sees it; the API
	 * refuses everyone else, and this section then stays hidden. */
	import { Section, Row, Dot, Notice } from '$lib/ui/page';
	import { failureSentence } from '$lib/model/failure';
	import { onMount } from 'svelte';

	let { companyId }: { companyId: string } = $props();
	type Link = {
		token_reference: string;
		bot_username: string;
		pairing_code: string | null;
		code_expires_at: string | null;
		paired_at: string | null;
	};
	type Channel = { configured: boolean; paired: boolean; link?: Link; pair_url?: string };

	let view = $state<Channel | null>(null);
	let hidden = $state(false);
	let reference = $state('');
	let busy = $state(false);
	let error = $state('');
	const endpoint = $derived(`/api/companies/${encodeURIComponent(companyId)}/telegram`);
	const waiting = $derived(!!view?.pair_url && !view.paired);

	async function call(method: string, body?: unknown) {
		const response = await fetch(endpoint, {
			method,
			cache: 'no-store',
			credentials: 'same-origin',
			headers: body ? { 'content-type': 'application/json' } : undefined,
			body: body ? JSON.stringify(body) : undefined
		});
		if (response.status === 403) {
			hidden = true;
			return;
		}
		const result = await response.json().catch(() => null);
		if (!response.ok) throw new Error(result?.message ?? 'Telegram is unavailable.');
		view = result;
		if (view?.link && !reference) reference = view.link.token_reference;
	}

	async function act(method: string, body?: unknown) {
		busy = true;
		error = '';
		try {
			await call(method, body);
		} catch (e) {
			error = failureSentence(e, 'Telegram did not respond.');
		} finally {
			busy = false;
		}
	}

	function pair(event: SubmitEvent) {
		event.preventDefault();
		void act('POST', { token_reference: reference.trim(), cockpit_origin: location.origin });
	}

	onMount(() => void act('GET'));

	/* While a code is out, notice the pairing as soon as the bot sees it. */
	$effect(() => {
		if (!waiting) return;
		const timer = setInterval(() => void call('GET').catch(() => {}), 4000);
		return () => clearInterval(timer);
	});
</script>

{#if !hidden}
	<Section
		id="telegram"
		title="Telegram"
		info="Each new Attention item arrives in Telegram with a link back here, and approvals can be answered there. Only the paired chat can decide."
	>
		{#snippet actions()}
			{#if view?.paired}<Dot show tone="success" label="Paired" />
			{:else if waiting}<Dot show tone="progress" label="Waiting for the code" />{/if}
		{/snippet}
		{#if error}<Notice tone="danger" title="Telegram did not respond" details={error} />{/if}
		{#if view?.paired && view.link}
			<Row
				label="Bot"
				info="Approve and Decline in this chat act as you, the company's Authority owner."
			>
				<div class="line">
					<span>@{view.link.bot_username}</span>
					<button class="btn small ghost" disabled={busy} onclick={() => act('DELETE')}
						>Unpair</button
					>
				</div>
			</Row>
		{:else if waiting && view?.link && view.pair_url}
			<Row
				label="Pairing code"
				info="Send this code to the bot from your own Telegram account. It works once and expires in 30 minutes."
			>
				<div class="line">
					<code>{view.link.pairing_code}</code>
					<a class="btn primary small" href={view.pair_url} target="_blank" rel="noopener"
						>Open @{view.link.bot_username}</a
					>
					<button class="btn small ghost" disabled={busy} onclick={() => act('DELETE')}
						>Cancel</button
					>
				</div>
			</Row>
		{:else}
			<form onsubmit={pair}>
				<Row
					label="Bot token"
					info="Create a bot with BotFather, store its token in the Vault, then enter that secret's reference, such as infisical:/companies/{companyId}/telegram-bot or env:TELEGRAM_BOT_TOKEN."
				>
					<div class="line">
						<input
							class="wide"
							bind:value={reference}
							aria-label="Bot token reference"
							placeholder="infisical:/companies/{companyId}/telegram-bot"
							required
							disabled={busy}
						/>
						<button class="btn primary small" type="submit" disabled={busy || !reference.trim()}
							>{busy ? 'Checking…' : view?.configured ? 'New code' : 'Pair'}</button
						>
					</div>
				</Row>
			</form>
		{/if}
	</Section>
{/if}

<style>
	.line {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
	}
	.wide {
		width: min(320px, 100%);
	}
	code {
		font-size: var(--t-body);
		letter-spacing: 0.08em;
	}
</style>
