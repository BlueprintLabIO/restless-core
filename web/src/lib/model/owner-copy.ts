/* Owner-facing copy is written by the platform, in plain words, by state. An agent's or the
 * system's own reason (abandon notes, resolutions, run summaries) can carry ids, hashes, agent
 * handles and message numbers; those belong in Activity's details, never in a notice or a status
 * line. */

import type { DecisionContinuation } from './view';

/* The engine's half of this guard is crates/restless-engine/src/owner_copy.rs; keep the two
 * saying the same. */

/* 8+ hex characters with a digit and a letter among them (a commit, a short or full uuid), never
 * a word or a plain number such as a phone number. */
const HEX_ID = /(?<![\w/.-])[0-9a-f]+(?:-[0-9a-f]+)*(?![\w/-])/gi;
const MESSAGE_NUMBER = /\b(?:msg|message)\s*#?\d+/i;
/* An agent's handle: lowercase, hyphenated, ending in a role (`reno-supervisor`, `ops-lead`). */
const AGENT_HANDLE =
	/(?<![\w-])[a-z][a-z0-9]*(?:-[a-z0-9]+)*-(?:supervisor|lead|exec|staff|agent|worker|writer|reviewer|bot|ops)(?![\w-])/;
/* A code name such as `work_id` or `exec_startup_blocked`. */
const SNAKE_CASE = /(?<![\w/.-])[a-z][a-z0-9]*(?:_[a-z0-9]+)+(?![\w/-])/;
/* Engine nouns an owner never needs to read. */
const INTERNAL_NOUN =
	/\b(?:delivery node|work node|digests?|bound to|handoff|attempt[_ ]id|work[_ ]id|orgintel|authority record)\b/i;
/* A provider's or the runtime's own error rather than a sentence: a `[kind]` tag, a JSON body, an
 * `error:` label, a request id or an HTTP status code. */
const RAW_ERROR =
	/^\s*\[[a-z_-]*\]|[{}]|\brequest[ _]id\b|\berror\s*[:=]|\bexception\b|(?:^\s*|\b(?:http|status|code|error|returned)\s+)[45]\d\d\b/i;
const URL = /\S*:\/\/\S*/g;

function hexIds(text: string): string[] {
	return (text.match(HEX_ID) ?? []).filter((word) => {
		const bare = word.replace(/-/g, '');
		return bare.length >= 8 && /\d/.test(bare) && /[a-f]/i.test(bare);
	});
}

/** True when a line carries ids, hashes, message numbers, agent handles, code names, engine nouns
 * or a raw provider error. Addresses are read whole, so their paths are not words. */
export function leaksInternals(text: string, actorIds: readonly string[] = []): boolean {
	const plain = text.replace(URL, ' ');
	if (hexIds(plain).length || MESSAGE_NUMBER.test(plain) || AGENT_HANDLE.test(plain)) return true;
	if (SNAKE_CASE.test(plain) || INTERNAL_NOUN.test(plain) || RAW_ERROR.test(plain)) return true;
	return actorIds.some(
		(id) =>
			/[-_]/.test(id) &&
			new RegExp(`(?<![\\w-])${id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}(?![\\w-])`).test(plain)
	);
}

/**
 * An agent's own words with ids, hashes, message numbers and handles taken out: a known actor id
 * becomes their name, an unknown handle "a teammate", a code name plain words. Addresses stay
 * whole. What is left may still leak (a raw error); `ownerText` decides.
 */
export function scrubInternals(text: string, names: Readonly<Record<string, string>> = {}): string {
	const urls: string[] = [];
	let said = text.replace(URL, (url) => `\u0000${urls.push(url) - 1}\u0000`);
	/* "(msg 1513)", "[44b3016c]": an aside of internals goes with its space. */
	said = said.replace(/\s*[([]([^()[\]]*)[)\]]/g, (aside, inner: string) =>
		leaksInternals(inner) ? '' : aside
	);
	said = said.replace(new RegExp(MESSAGE_NUMBER.source, 'gi'), '');
	for (const id of hexIds(said)) said = said.replace(id, '');
	said = said.replace(new RegExp(AGENT_HANDLE.source, 'g'), (handle) => {
		const name = names[handle];
		return name && !AGENT_HANDLE.test(name) ? name : 'a teammate';
	});
	for (const [id, name] of Object.entries(names))
		if (/[-_]/.test(id) && name)
			said = said.replace(
				new RegExp(`(?<![\\w-])${id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}(?![\\w-])`, 'g'),
				name
			);
	said = said.replace(new RegExp(SNAKE_CASE.source, 'g'), (code) => code.replace(/_/g, ' '));
	said = said
		.replace(/\s+([,.;:])/g, '$1')
		.replace(/\(\s*\)|\[\s*\]/g, '')
		.replace(/,(?=[,.])/g, '')
		.replace(/:(?=\.)/g, '')
		.replace(/\s{2,}/g, ' ')
		.trim()
		.replace(/^[,.;:]+\s*/, '')
		.replace(/[,;:]+$/, '');
	return said.replace(/\u0000(\d+)\u0000/g, (_, at: string) => urls[Number(at)]);
}

/** A line an agent or a provider wrote, scrubbed for the owner, or the platform's own line when
 * nothing plain is left. */
export function ownerText(
	text: string | null | undefined,
	fallback: string,
	names: Readonly<Record<string, string>> = {}
): string {
	const said = scrubInternals(text ?? '', names);
	return said && !leaksInternals(said) ? said : fallback;
}

/** A recorded line the owner may read, or the platform's own line when it leaks internals. */
export function ownerLine(
	text: string,
	fallback: string,
	actorIds: readonly string[] = []
): string {
	const said = text.trim();
	return said && !leaksInternals(said, actorIds) ? said : fallback;
}

const TITLE_LIMIT = 120;
const LINK = /^(?:[("'[]*)(?:https?:\/\/|www\.)/i;

/**
 * A card title an owner reads: never a URL, never more than a short line (the engine's
 * `owner_copy::owner_title`). The link lives in the card's Open action. 3 November: a sign-in
 * step titled with its whole Cloudflare OAuth URL rendered as a ten-line heading.
 */
export function ownerTitle(text: string): string {
	let title = text
		.split(/\s+/)
		.filter((word) => word && !LINK.test(word))
		.join(' ')
		.replace(/[\s:,;([\-–—]+$/, '')
		.trim();
	if (title.length > TITLE_LIMIT) {
		const cut = title.slice(0, TITLE_LIMIT - 1);
		const space = cut.lastIndexOf(' ');
		title = `${(space > 40 ? cut.slice(0, space) : cut).replace(/[^\p{L}\p{N}]+$/u, '')}…`;
	}
	return title || text.trim().slice(0, TITLE_LIMIT);
}

/** A sentence with its raw links taken out, for a card whose action already carries the link. */
export function withoutLinks(text: string): string {
	const said = text
		.split(/(\s+)/)
		.filter((word) => !LINK.test(word))
		.join('')
		.replace(/[ \t]+\n/g, '\n')
		.replace(/[ \t]{2,}/g, ' ')
		.replace(/[\s:,;([\-–—]+$/, '')
		.trim();
	return said ? `${said}${/[.!?…]$/.test(said) ? '' : '.'}` : text;
}

export type DecidedOutcome = NonNullable<DecisionContinuation['outcome']>;

export interface DecidedNotice {
	/** The item's title, when it reads cleanly; the notice still makes sense without it. */
	title: string;
	text: string;
}

type Decided = Pick<
	DecisionContinuation,
	'title' | 'outcome' | 'decidedBy' | 'byOwner' | 'viaChat' | 'replacementItemId'
>;

/* "<who> approved this." in every voice the record allows. */
function decidedBy(
	decision: Decided,
	viewerIsOwner: boolean,
	verbs: { past: string; already: string; passive: string }
): string {
	if (decision.viaChat)
		return `Exec ${verbs.past} this from ${viewerIsOwner ? 'your' : 'the owner’s'} chat.`;
	if (decision.byOwner)
		return viewerIsOwner ? `You already ${verbs.already} this.` : `The owner ${verbs.past} this.`;
	if (decision.decidedBy) return `${decision.decidedBy} ${verbs.past} this.`;
	return `This was already ${verbs.passive}.`;
}

/**
 * What an old link to an Inbox item says once it is settled. Built only from the decision's
 * state; the record's own reason stays behind "See it in Activity".
 */
export function decidedNotice(
	decision: Decided,
	options: { viewerIsOwner: boolean; replacementInInbox?: boolean }
): DecidedNotice {
	const { viewerIsOwner } = options;
	const title = ownerLine(decision.title, '');
	let text: string;
	switch (decision.outcome) {
		case 'replaced':
			text = options.replacementInInbox
				? `This was replaced by an updated version. The new one is in ${viewerIsOwner ? 'your' : 'the'} Inbox.`
				: `This was replaced by an updated version. If it needs ${viewerIsOwner ? 'you' : 'the owner'}, it will come to ${viewerIsOwner ? 'your' : 'the'} Inbox.`;
			break;
		case 'withdrawn':
			text = 'Withdrawn: this is no longer needed.';
			break;
		case 'expired':
			text = 'This expired before it was decided.';
			break;
		case 'declined':
			text = decidedBy(decision, viewerIsOwner, {
				past: 'declined',
				already: 'declined',
				passive: 'declined'
			});
			break;
		case 'changes_requested':
			text = decidedBy(decision, viewerIsOwner, {
				past: 'asked for changes to',
				already: 'asked for changes to',
				passive: 'sent back for changes'
			});
			break;
		case 'completed':
			text = decidedBy(decision, viewerIsOwner, {
				past: 'did',
				already: 'did',
				passive: 'done'
			});
			break;
		case 'revoked':
			text = decidedBy(decision, viewerIsOwner, {
				past: 'took back approval for',
				already: 'took back approval for',
				passive: 'taken back'
			});
			break;
		case 'approved':
			text = decidedBy(decision, viewerIsOwner, {
				past: 'approved',
				already: 'approved',
				passive: 'approved'
			});
			break;
		default:
			text = 'This was already decided.';
	}
	return { title, text };
}

/** When an old link's item has no record yet (or the record could not be read). */
export const RESOLVED_ELSEWHERE = 'That item has been resolved or is no longer in the Inbox.';
