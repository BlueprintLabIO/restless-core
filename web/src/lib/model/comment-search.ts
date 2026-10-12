/* Comments on documents the viewer can read, found by their text (S65). The server applies the
 * document access rule; the link opens the document with its comments beside it. */
import { responseFailure } from './failure';

export interface CommentSearchHit {
	document_id: string;
	document_title: string;
	thread_id: string;
	comment_id: string;
	author_actor_id: string;
	snippet: string;
	created_at: string;
	href: string;
}

export async function searchComments(
	company: string,
	text: string,
	limit = 6,
	signal?: AbortSignal
): Promise<CommentSearchHit[]> {
	const query = new URLSearchParams({ q: text, limit: String(limit) });
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/comment-search?${query}`,
		{ credentials: 'same-origin', cache: 'no-store', signal }
	);
	if (!response.ok) throw await responseFailure(response);
	const body = (await response.json()) as { comments?: CommentSearchHit[] };
	return Array.isArray(body.comments) ? body.comments : [];
}
