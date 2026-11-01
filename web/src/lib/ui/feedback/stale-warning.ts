/* What a surface still showing its last good data says when a refresh fails, as a toast message
 * for toastWhile: the same words FailureNotice's `stale` form used, with Retry when retrying can
 * help. Silent while the app's connection banner already speaks for the outage. */
import { describeFailure } from '$lib/model/failure';
import { connection } from '$lib/model/connection.svelte';
import type { ToastMessage } from './toast.svelte';

export function staleRefreshMessage(
	error: unknown,
	subject: string,
	dedupeKey: string,
	retry?: () => unknown
): ToastMessage | null {
	if (!error) return null;
	const view = describeFailure(error, subject);
	if (
		(view.kind === 'unreachable' && connection.lost) ||
		(view.kind === 'offline' && typeof navigator !== 'undefined' && !navigator.onLine)
	)
		return null;
	const transient = ['offline', 'unreachable', 'busy', 'unavailable'].includes(view.kind);
	return {
		text: transient
			? `Couldn’t refresh ${subject}. Showing the last update.`
			: `${view.title}. ${view.detail}`,
		tone: 'failure',
		details: view.technical ?? null,
		dedupeKey,
		...(retry && view.retryable ? { action: { label: 'Retry', run: retry } } : {})
	};
}
