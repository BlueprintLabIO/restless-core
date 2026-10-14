/* Reactions on the messages in view. A reaction is a stored signal: it
 * never sends a message or starts a turn. */
import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import type { MessageReactionRow } from './generated/orgintel';
import type { ConversationMessageView } from './generated/conversation';
import { ownerJson } from './failure';

export const REACTIONS = ['👍', '👀', '✅', '❤️', '🎉', '❓'] as const;
/** Pinning is a reaction too: stored, shared, and never a message. */
export const PIN = '📌';

export type ReactionSummary = { emoji: string; count: number; mine: boolean };

export function reactionsQuery(company: () => string, ids: () => number[], viewer: () => string) {
	const client = useQueryClient();
	const key = () => ['message-reactions', company(), ids().join(',')];
	const query = createQuery(() => ({
		queryKey: key(),
		queryFn: async ({ signal }) => {
			const list = ids();
			if (!list.length) return [] as MessageReactionRow[];
			const response = await fetch(
				`/api/companies/${encodeURIComponent(company())}/message-reactions?ids=${list.join(',')}`,
				{ signal }
			);
			return (await ownerJson<{ reactions: MessageReactionRow[] }>(response)).reactions;
		},
		enabled: !!company() && ids().length > 0,
		staleTime: 10_000,
		refetchInterval: 30_000
	}));
	function summaryFor(messageId: string | number): ReactionSummary[] {
		const id = Number(messageId);
		const rows = (query.data ?? []).filter((row) => row.message_id === id);
		return REACTIONS.flatMap((emoji) => {
			const matching = rows.filter((row) => row.emoji === emoji);
			return matching.length
				? [
						{
							emoji,
							count: matching.length,
							mine: matching.some((row) => row.actor_id === viewer())
						}
					]
				: [];
		});
	}
	/** Messages in view someone has pinned. */
	function pinned(): Set<number> {
		return new Set(
			(query.data ?? []).filter((row) => row.emoji === PIN).map((row) => row.message_id)
		);
	}
	async function react(messageId: string | number, emoji: string, on: boolean) {
		const response = await fetch(
			`/api/companies/${encodeURIComponent(company())}/messages/${encodeURIComponent(String(messageId))}/reactions`,
			{
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ emoji, on })
			}
		);
		await ownerJson(response);
		await client.invalidateQueries({ queryKey: ['message-reactions', company()] });
	}
	return { summaryFor, react, pinned };
}

/** Every message pinned in one Room, however old: pins are listed by Room, not found among the
 * messages a page happens to hold. Shares the reactions key, so pinning refreshes it. */
export function roomPinsQuery(company: () => string, room: () => string, enabled: () => boolean) {
	const query = createQuery(() => ({
		queryKey: ['message-reactions', company(), 'pins', room()],
		queryFn: async ({ signal }) => {
			const response = await fetch(
				`/api/companies/${encodeURIComponent(company())}/rooms/${encodeURIComponent(room())}/pins`,
				{ signal }
			);
			return (await ownerJson<{ pins: ConversationMessageView[] }>(response)).pins;
		},
		enabled: !!company() && !!room() && enabled(),
		staleTime: 10_000,
		refetchInterval: 60_000
	}));
	return {
		/** Null until the list has loaded (or when it cannot be). */
		get pins(): ConversationMessageView[] | null {
			return query.data ?? null;
		}
	};
}
