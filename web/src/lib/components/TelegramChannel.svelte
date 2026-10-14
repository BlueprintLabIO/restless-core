<script lang="ts">
	/* The owner's Telegram channel: paste the bot's token (kept in the Vault) or a
	 * reference to one, pair one chat with a one-time code, unpair. Only the Authority owner sees it; the API
	 * refuses everyone else, and this section then stays hidden. */
	import { Section, Row, Dot, Notice, Toggle } from '$lib/ui/page';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { saveSetting, settingQuery } from '$lib/model/setting.svelte';
	import { telegramKey, telegramRequest, type TelegramView } from '$lib/model/telegram';

	let { companyId }: { companyId: string } = $props();
	const client = useQueryClient();
	/* While a code is out, notice the pairing as soon as the bot sees it. */
	const source = settingQuery<TelegramView>({
		key: () => telegramKey(companyId),
		read: () => telegramRequest(companyId, 'GET'),
		poll: (current) => (current?.pair_url && !current.paired ? 4_000 : false)
	});
	const view = $derived(source.view);
	const hidden = $derived(!source.loading && !source.failure && source.view === null);
	let reference = $state('');
	let busy = $state(false);
	let actionError = $state('');
	const error = $derived(
		actionError ||
			(source.failure ? failureSentence(source.failure, 'Telegram did not respond.') : '')
	);
	$effect(() => {
		if (view?.link && !reference) reference = view.link.token_reference;
	});
	const waiting = $derived(!!view?.pair_url && !view.paired);
	/* Pairing stores this page's address for the links Telegram sends; once a
	 * code is out or a chat is paired, the stored address decides. */
	const hostname = location.hostname;
	const pageLocal =
		hostname === 'localhost' ||
		hostname.endsWith('.localhost') ||
		hostname === '[::1]' ||
		/^127\.\d+\.\d+\.\d+$/.test(hostname);
	const stored = $derived(!!view?.paired || waiting);
	const localLinks = $derived(stored ? !!view?.local_links : pageLocal);

	/* Unpairing forgets the chat; pairing again needs a new code from the owner's phone. */
	async function unpair() {
		if (
			await confirmAction({
				title: `Unpair @${view?.link?.bot_username ?? 'the bot'}?`,
				body: 'Telegram stops receiving Attention items, and its buttons stop answering. Pairing again needs a new code.',
				action: 'Unpair',
				danger: true
			})
		)
			void act('DELETE');
	}

	async function act(method: 'POST' | 'PATCH' | 'DELETE', body?: unknown) {
		busy = true;
		actionError = '';
		try {
			await saveSetting(client, telegramKey(companyId), () =>
				telegramRequest(companyId, method, body)
			);
		} catch (e) {
			actionError = failureSentence(e, 'Telegram did not respond.');
		} finally {
			busy = false;
		}
	}

	/* A stored reference (after the first pairing, or typed by someone who keeps their own
	 * secrets) is sent as one; anything else is the token BotFather gave, kept in the Vault. */
	function pair(event: SubmitEvent) {
		event.preventDefault();
		const value = reference.trim();
		const isReference = /^(infisical|env):/.test(value);
		void act('POST', {
			...(isReference ? { token_reference: value } : { token: value }),
			cockpit_origin: location.origin
		});
	}
</script>

{#if !hidden}
	<Section
		id="telegram"
		title="Telegram"
		info={localLinks
			? `Each new Attention item arrives in Telegram, and approvals can be answered there. Each item's link opens only on ${stored ? 'the computer you paired from' : 'this computer'}; pair from an address your phone can reach to open it there. Only the paired chat can decide.`
			: 'Each new Attention item arrives in Telegram with a link back here, and approvals can be answered there. Only the paired chat can decide.'}
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
					<button class="btn small ghost" disabled={busy} onclick={unpair}>Unpair</button>
				</div>
			</Row>
			<Row
				label="Only decisions"
				info="Send only approvals and requests you can answer in the chat. Everything else still waits in the Inbox."
			>
				<Toggle
					checked={!!view.link.decisions_only}
					label="Send only decisions to Telegram"
					disabled={busy}
					onchange={(next) => act('PATCH', { decisions_only: next })}
				/>
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
					info="In Telegram, open BotFather, send /newbot and follow its two questions. Paste the token it replies with here; it is kept in the company Vault. A Vault reference such as env:TELEGRAM_BOT_TOKEN works too."
				>
					<div class="line">
						<a class="btn small ghost" href="https://t.me/BotFather" target="_blank" rel="noopener"
							>Open BotFather</a
						>
						<input
							class="wide"
							bind:value={reference}
							aria-label="Bot token"
							placeholder="123456789:AA…"
							autocomplete="off"
							spellcheck="false"
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
