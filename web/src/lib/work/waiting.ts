import type { AttentionItem } from '$lib/model/view';
import { attentionAsker } from '../model/attention-presentation.ts';
import { routeActionLabel, type AgentRouteState } from '../model/intelligence-route.ts';
import { ownerText } from '../model/owner-copy.ts';
import { blockerSentence } from '../model/work-actions.ts';
import { newestAttempt } from './status.ts';

/* Why something waits, and on whom, in one plain line (S109 A1): "Waiting on you: approve the
 * brand rules", "Waiting on Baloo: running", "Waiting on a model: connect a model for Baloo".
 * Work rows, board cards, the Work page and Inbox rows all read it here, so they never disagree
 * about who holds the next move. Finished or stopped Work waits on nobody and has no line. */

/** An authored action as the tail of the line: "Approve transfer." reads "approve transfer". A
 * first word that is not an ordinary capitalised verb (a name, an acronym) keeps its case. */
export function actionPhrase(text: string): string {
	/* A lead's raw ask can carry ids and handles; they never reach the line (owner-copy.ts), and
	 * an ask that is only internals gives way to the caller's fallback. */
	const said = ownerText(text, '')
		.replace(/^needs you:\s*/i, '')
		.replace(/[.!?…]+$/, '')
		.trim();
	const first = said.split(/\s/)[0] ?? '';
	return /^[A-Z][a-z]+$/.test(first) && first !== 'Exec'
		? said[0].toLowerCase() + said.slice(1)
		: said;
}

/** What an Inbox item asks, in a few words, when its author wrote no action. */
function askedFallback(item: Pick<AttentionItem, 'category'>): string {
	if (item.category === 'approval') return 'your approval';
	if (item.category === 'review') return 'your review';
	if (item.category === 'human_step') return 'your step';
	if (item.category === 'conversation') return 'your answer';
	if (item.category === 'blocker') return 'your setup';
	return 'your decision';
}

/** The Inbox row's line: everything there waits on the viewer unless it is still being prepared. */
export function inboxWaitingLine(item: AttentionItem): string {
	const asker = attentionAsker(item);
	if (item.preparing) return `Waiting on ${asker}: preparing it`;
	const asked = actionPhrase(item.requestedAction) || askedFallback(item);
	return `Waiting on you: ${asked} · from ${asker}`;
}

type WaitingAttempt = {
	work_id: string;
	actor_id: string;
	state: string;
	revision: number;
	attempt_no: number;
	started_at: string;
	summary?: string;
};

export type WaitingSources = {
	/** The viewer's actor; "you" when the line names them. */
	viewer: string | null;
	viewerIsOwner: boolean;
	name: (actorId: string) => string;
	/** The owner's queue: items on this Work wait on the owner, or are still being prepared. */
	items?: readonly AttentionItem[];
	/** A non-owner's queue: the handoffs passed to them. */
	viewerHandoffs?: readonly { work_id: string; requested_action: string }[];
	/** Every handoff in the graph, where the viewer may read them (the owner's). */
	handoffs?: readonly {
		work_id: string;
		state?: string;
		assigned_to?: string | null;
		requested_action: string;
	}[];
	edges?: readonly { from_work_id: string; to_work_id: string; kind: string }[];
	work?: readonly { id: string; title: string; status: string }[];
	attempts?: readonly WaitingAttempt[];
	/** Whether an actor's model can reply now; omitted when the routes are not read. */
	route?: (actorId: string) => AgentRouteState;
};

/** The one line for an open Work, or null when nothing waits. `reason: false` leaves a blocker's
 * reason out where the page already shows it in full. */
export function workWaitingLine(
	work: { id: string; owner_id: string; status: string; resolution: string },
	sources: WaitingSources,
	{ reason = true }: { reason?: boolean } = {}
): string | null {
	if (work.status === 'completed' || work.status === 'abandoned') return null;
	const who = (actor: string) => (actor && actor === sources.viewer ? 'you' : sources.name(actor));

	const items = (sources.items ?? []).filter((item) => item.workId === work.id);
	const ready = items.find((item) => !item.preparing);
	if (ready)
		return `Waiting on you: ${actionPhrase(ready.requestedAction) || askedFallback(ready)}`;
	const passed = sources.viewerHandoffs?.find((handoff) => handoff.work_id === work.id);
	if (passed) return `Waiting on you: ${actionPhrase(passed.requested_action) || 'your decision'}`;
	const preparing = items.find((item) => item.preparing);
	if (preparing) return `Waiting on ${attentionAsker(preparing)}: preparing your step`;

	const held = sources.handoffs?.find(
		(handoff) => handoff.work_id === work.id && handoff.state === 'pending'
	);
	if (held) {
		const asked = actionPhrase(held.requested_action) || 'a decision';
		if (held.assigned_to) return `Waiting on ${who(held.assigned_to)}: ${asked}`;
		return `Waiting on ${sources.viewerIsOwner ? 'you' : 'the owner'}: ${asked}`;
	}

	if (work.status === 'proposed' || work.status === 'blocked') {
		const open = new Map((sources.work ?? []).map((row) => [row.id, row] as const));
		const before = (sources.edges ?? [])
			.filter((edge) => edge.kind === 'requires' && edge.to_work_id === work.id)
			.flatMap((edge) => {
				const row = open.get(edge.from_work_id);
				return row && row.status !== 'completed' && row.status !== 'abandoned' ? [row] : [];
			});
		if (before.length)
			return `Waiting on other Work: ${before[0].title}${before.length > 1 ? ` and ${before.length - 1} more` : ''}`;
	}

	const attempt = newestAttempt((sources.attempts ?? []).filter((row) => row.work_id === work.id));
	if (work.status === 'blocked') {
		const why = reason ? blockerSentence(work.resolution, attempt?.summary ?? '') : '';
		const resumer = sources.viewerIsOwner ? 'you' : 'the owner';
		return `Waiting on ${resumer}: resume it${why ? `. ${why.replace(/[.!?…]+$/, '')}` : ''}`;
	}

	const doer = attempt?.state === 'running' ? attempt.actor_id : work.owner_id;
	const action = sources.route ? routeActionLabel(sources.route(doer)) : null;
	if (action) return `Waiting on a model: ${actionPhrase(action)} for ${who(doer)}`;

	if (work.status === 'proposed') return `Waiting on ${who(work.owner_id)}: not started yet`;
	if (attempt?.state === 'running') return `Waiting on ${who(attempt.actor_id)}: running`;
	if (attempt?.state === 'changes_requested')
		return `Waiting on ${who(work.owner_id)}: the changes asked for`;
	if (attempt?.state === 'failed' || attempt?.state === 'blocked')
		return `Waiting on ${who(work.owner_id)}: a new run after one failed`;
	return `Waiting on ${who(work.owner_id)}: the next run`;
}
