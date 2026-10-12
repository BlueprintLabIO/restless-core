/* Reactions on a document's comments (S65): the same fixed reactions as chat, kept with the comment
 * in the company's records. Reacting is a stored signal: it never sends a message or wakes anyone.
 * Anyone who may read the document sees them; anyone who may comment adds or takes back their own. */
import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import { ownerJson } from './failure';
import { REACTIONS, type ReactionSummary } from './reactions.svelte';

interface CommentReactionRow {
	comment_id: string;
	actor_id: string;
	emoji: string;
}

export function documentCommentReactionsQuery(
	company: () => string,
	documentId: () => string,
	viewer: () => string
) {
	const client = useQueryClient();
	const key = () => ['document-comment-reactions', company(), documentId()];
	const url = () =>
		`/api/companies/${encodeURIComponent(company())}/documents/${encodeURIComponent(documentId())}/comment-reactions`;
	const query = createQuery(() => ({
		queryKey: key(),
		queryFn: async ({ signal }) => {
			const response = await fetch(url(), { credentials: 'same-origin', signal });
			return (await ownerJson<{ reactions: CommentReactionRow[] }>(response)).reactions;
		},
		enabled: !!company() && !!documentId(),
		staleTime: 10_000,
		refetchInterval: 30_000
	}));

	function summaryFor(commentId: string): ReactionSummary[] {
		const rows = (query.data ?? []).filter((row) => row.comment_id === commentId);
		return REACTIONS.flatMap((emoji) => {
			const matching = rows.filter((row) => row.emoji === emoji);
			return matching.length
				? [{ emoji, count: matching.length, mine: matching.some((row) => row.actor_id === viewer()) }]
				: [];
		});
	}

	async function react(commentId: string, emoji: string, on: boolean): Promise<void> {
		const response = await fetch(url(), {
			method: 'POST',
			credentials: 'same-origin',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ comment_id: commentId, emoji, on })
		});
		const { reactions } = await ownerJson<{ reactions: CommentReactionRow[] }>(response);
		client.setQueryData(key(), reactions);
	}

	return { summaryFor, react };
}
