<script lang="ts">
	import { tick } from 'svelte';
	import Link from '@lucide/svelte/icons/link';
	import ListPlus from '@lucide/svelte/icons/list-plus';
	import Pin from '@lucide/svelte/icons/pin';
	import Reply from '@lucide/svelte/icons/reply';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import { followChat } from '$lib/actions/follow-chat';
	import type { ReactionSummary } from '$lib/model/reactions.svelte';

	/* The Exec rail's transcript on example messages: owner bubbles, a long Exec reply that folds, a
	 * continued reply, a day divider, the hover toolbar with the rail's own actions, and sending —
	 * the sent message rises to the top and a streamed reply grows beneath it. Width follows the
	 * window, so the rail and a phone can both be judged. */
	const at = (minutes: number) =>
		new Date(Date.parse('2026-10-07T01:00:00Z') + minutes * 60_000).toISOString();
	const long = `Both recorded: the company is **Blueprint Lab**, and we're going after small local service businesses with dated websites.

Only two things are left before the page can take enquiries — confirm A$4,800 or give a different number, and send the email address or booking link the "Book a free website review" button should use. Until then it stays honest that booking isn't open.

No customer has been contacted and nothing is published. The page is still live for you in the company browser, and the critic's review is attached to the Work item if you want the detail before deciding.`;
	const reply = `On it. I'll have Robin shortlist premium service businesses whose landing pages undersell them — slow, dated, unclear offers or no booking path — and score each on how much a renovation would lift enquiries.

You'll get a ranked list of 50 with a one-line diagnosis and the live link for each, so you can pick who to approach first. Nothing is sent to any of them until you say so.

First results should land within the hour.`;

	type Line = {
		id: number;
		sender: 'owner' | 'agent';
		author: string;
		text: string;
		createdAt: string;
	};
	let lines = $state<Line[]>([
		{
			id: 1,
			sender: 'owner',
			author: 'You',
			text: 'a business that helps other businesses renovate their websites!',
			createdAt: at(-1380)
		},
		{
			id: 2,
			sender: 'agent',
			author: 'The Exec',
			text: "Owner direction recorded: build a business that renovates other businesses' websites; first storefront landing page is in production.",
			createdAt: at(-1170)
		},
		{ id: 3, sender: 'owner', author: 'You', text: 'Both are right', createdAt: at(17) },
		{ id: 4, sender: 'agent', author: 'The Exec', text: long, createdAt: at(17) }
	]);
	let reactions = $state<Record<number, ReactionSummary[]>>({
		4: [{ emoji: '👍', count: 1, mine: true }]
	});
	let pinned = $state<Set<number>>(new Set());
	let draft = $state('can you find me 50 premium services/sites with terrible landing pages?');
	let streaming = $state(false);
	let scrollEl = $state<HTMLDivElement>();
	let away = $state(false);

	function react(id: number, emoji: string, on: boolean) {
		const current = reactions[id] ?? [];
		const rest = current.filter((reaction) => reaction.emoji !== emoji);
		reactions[id] = on ? [...rest, { emoji, count: 1, mine: true }] : rest;
	}

	async function send() {
		const text = draft.trim();
		if (!text || streaming) return;
		draft = '';
		const now = new Date().toISOString();
		lines.push({ id: lines.length + 1, sender: 'owner', author: 'You', text, createdAt: now });
		await tick();
		scrollEl?.dispatchEvent(new Event('chat-scroll-anchor'));
		streaming = true;
		const id = lines.length + 1;
		lines.push({ id, sender: 'agent', author: 'The Exec', text: '', createdAt: now });
		const words = reply.split(/(\s+)/);
		for (let index = 0; index < words.length; index += 1) {
			await new Promise((resolve) => setTimeout(resolve, 18));
			const line = lines.find((entry) => entry.id === id);
			if (line) line.text += words[index];
		}
		streaming = false;
	}
</script>

<main class="bridge-root transcript-gallery">
	<section class="rail" aria-label="Exec transcript example">
		<div
			class="exr-msgs"
			bind:this={scrollEl}
			use:followChat={{ key: 'gallery', onfollow: (following) => (away = !following) }}
		>
			{#each lines as line, index (line.id)}
				{#if index === 0}<div class="day-sep" aria-hidden="true"><span>Yesterday</span></div>{/if}
				{#if index === 2}<div class="day-sep" aria-hidden="true"><span>Today</span></div>{/if}
				<ConversationMessage
					sender={line.sender}
					author={line.author}
					text={line.text || '…'}
					createdAt={line.createdAt}
					messageId={String(line.id)}
					reactions={reactions[line.id] ?? []}
					onreact={(emoji, on) => react(line.id, emoji, on)}
				>
					{#snippet actions()}<button
							type="button"
							class:confirmed={pinned.has(line.id)}
							aria-label={pinned.has(line.id) ? 'Unpin this message' : 'Pin this message'}
							title={pinned.has(line.id) ? 'Unpin' : 'Pin above the conversation'}
							onclick={() => {
								const next = new Set(pinned);
								if (!next.delete(line.id)) next.add(line.id);
								pinned = next;
							}}><Pin size={12} aria-hidden="true" /></button
						><button type="button" aria-label="Copy link to this message" title="Copy link"
							><Link size={12} aria-hidden="true" /></button
						><button
							type="button"
							aria-label="Turn this into Work"
							title="Ask the Exec to turn this into Work"
							><ListPlus size={12} aria-hidden="true" /></button
						>{#if line.sender !== 'owner'}<button
								type="button"
								aria-label="Reply to this message"
								title="Reply to this message"><Reply size={12} aria-hidden="true" /></button
							>{/if}{/snippet}
				</ConversationMessage>
			{/each}
		</div>
		{#if away}<button
				type="button"
				class="jump"
				onclick={() => scrollEl?.dispatchEvent(new Event('chat-scroll-end'))}
				>Jump to latest ↓</button
			>{/if}
		<form
			class="composer"
			onsubmit={(event) => {
				event.preventDefault();
				void send();
			}}
		>
			<input bind:value={draft} aria-label="Message" placeholder="Answer Exec, or ask anything…" />
			<button type="submit" disabled={streaming || !draft.trim()}>Send</button>
		</form>
	</section>
</main>

<style>
	.transcript-gallery {
		display: grid;
		place-items: start center;
		min-height: 100vh;
		padding: 24px 12px;
		background: var(--bg-app);
	}
	.rail {
		position: relative;
		display: flex;
		flex-direction: column;
		width: min(560px, 100%);
		height: min(640px, calc(100vh - 48px));
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
		overflow: hidden;
	}
	.exr-msgs {
		padding: 14px 0 8px;
	}
	.jump {
		position: absolute;
		right: 50%;
		bottom: 64px;
		transform: translateX(50%);
		padding: 5px 12px;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: var(--surface-raised);
		color: var(--ink);
		font: 500 var(--t-label) var(--font-ui);
		box-shadow: 0 4px 12px rgba(43, 51, 66, 0.12);
	}
	.composer {
		display: flex;
		gap: 8px;
		padding: 10px;
		border-top: 1px solid var(--border);
	}
	.composer input {
		flex: 1;
		min-width: 0;
		padding: 8px 10px;
		border: 1px solid var(--border);
		border-radius: 8px;
		font: var(--t-body) var(--font-ui);
	}
</style>
