/* Exec manages the owner's Inbox from their chat (owner decision, 3 November 2026): it accepts,
 * declines, answers, snoozes, skips, moves and adds items on the owner's word. Each change names
 * the owner's message, shows beside its item, and can be undone for the same five seconds as the
 * owner's own clicks. */
import { ownerRequest } from './setting.svelte';
import { formatMoment } from '$lib/ui/time';

export type ExecVerb = 'accept' | 'decline' | 'answer' | 'snooze' | 'dismiss' | 'move' | 'add';

/** What Exec last did to one item, as the Inbox shows it beside the item. */
export interface ExecInboxNote {
	id: string;
	verb: ExecVerb;
	state: 'pending' | 'applied' | 'failed' | 'canceled';
	note: string;
	owner_words: string;
	source_message_id: number;
	undo_until: string;
	problem: string | null;
	snooze_until: string | null;
	position: number | null;
	at: string;
}

/** One of Exec's changes in the last few minutes: the notice that says it happened reads it, and
 * so does an open card before the full Inbox read has caught up. */
export interface ExecInboxActivity extends ExecInboxNote {
	item_id: string;
	title: string;
}

export interface ExecInboxActivityRead {
	items: ExecInboxActivity[];
	server_now: string;
	/** When the read left this browser, to turn server deadlines into local ones. */
	startedAt: number;
}

export const execActionsKey = (company: string) => ['exec-inbox-actions', company] as const;

/** Exec's changes in the last few minutes (the owner's only). */
export async function readExecActions(
	company: string,
	signal?: AbortSignal
): Promise<ExecInboxActivityRead> {
	const startedAt = Date.now();
	const reply = await ownerRequest<Omit<ExecInboxActivityRead, 'startedAt'>>(
		`/api/companies/${encodeURIComponent(company)}/exec-inbox-actions`,
		{ signal }
	);
	return { ...reply, startedAt };
}

/** A server deadline on this browser's clock, never later than the server meant. */
export function localDeadline(
	at: string,
	read: Pick<ExecInboxActivityRead, 'server_now' | 'startedAt'>
) {
	return Date.parse(at) - (Date.parse(read.server_now) - read.startedAt);
}

const done: Record<ExecVerb, string> = {
	accept: 'accepted',
	decline: 'declined',
	answer: 'answered',
	snooze: 'snoozed',
	dismiss: 'skipped',
	move: 'moved',
	add: 'added'
};
const doing: Record<ExecVerb, string> = {
	accept: 'accept',
	decline: 'decline',
	answer: 'answer',
	snooze: 'snooze',
	dismiss: 'skip',
	move: 'move',
	add: 'add'
};

function place(position: number | null): string {
	if (position === 1) return ' to the top';
	if (position === 0) return ' to the bottom';
	return position ? ` to place ${position}` : '';
}

/** The words before "from your chat": what Exec did to this item. */
export function execNoteLead(
	note: Pick<ExecInboxNote, 'verb' | 'state' | 'snooze_until' | 'position' | 'problem'>
): string {
	if (note.state === 'failed') return `Exec could not ${doing[note.verb]} this`;
	if (note.verb === 'snooze' && note.snooze_until)
		return `Exec snoozed this until ${formatMoment(new Date(note.snooze_until))}`;
	if (note.verb === 'move') return `Exec moved this${place(note.position)}`;
	if (note.verb === 'add') return 'Exec added this';
	return `Exec ${done[note.verb]} this`;
}

/** One sentence for the notice: "Exec accepted “Launch page” from your chat." */
export function execActivityText(
	activity: Pick<ExecInboxActivity, 'verb' | 'state' | 'title' | 'problem'>
): string {
	const title = `“${activity.title}”`;
	if (activity.state === 'failed')
		return `Exec could not ${doing[activity.verb]} ${title}. ${activity.problem ?? ''}`.trim();
	if (activity.verb === 'add') return `Exec added ${title} to your Inbox from your chat.`;
	return `Exec ${done[activity.verb]} ${title} from your chat.`;
}

export const ownerSaid = (words: string) => `You said: “${words}”`;

/** Undo one of Exec's changes while its window is open. */
export function undoExecAction(company: string, id: string) {
	return ownerRequest<{ id: string; state: string }>(
		`/api/companies/${encodeURIComponent(company)}/exec-inbox-actions/${encodeURIComponent(id)}`,
		{ method: 'DELETE' }
	);
}

/** Open the Exec rail at one message: the owner's words a change came from. */
export const SHOW_MESSAGE_EVENT = 'restless:show-message';
export function showChatMessage(messageId: number) {
	window.dispatchEvent(new CustomEvent(SHOW_MESSAGE_EVENT, { detail: { messageId } }));
}
