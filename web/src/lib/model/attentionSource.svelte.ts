/* One poller for the Attention projection, shared by every consumer.
 *
 * Two defects made this necessary.
 *
 * The first was a lie on first paint. `view` started null, `items` derived to
 * `[]`, and an empty array is indistinguishable from "we have not asked yet" —
 * so the surface confidently rendered "Queue clear" and "Nothing needs your
 * judgement" for one fetch round trip before contradicting itself. The fix is
 * not a spinner, it is a third state: `unknown` is not `[]`.
 *
 * The second was two independent timers. The shell polled this endpoint for the
 * tab badge and the surface polled it again for the queue, 8s apart on separate
 * clocks, so the badge and the pane could disagree for a beat and every owner
 * paid double the requests. Consumers now attach to one source; the poll starts
 * with the first consumer and stops with the last.
 *
 * A background refresh never blanks what is already on screen. If the source
 * drops, the last observed truth stays visible and is marked stale — an owner
 * deciding on a £2,400 approval should not have the queue vanish under them
 * because one poll timed out. */

import { getAttention, type AttentionView } from './attention';

/** `unknown` is the state that did not exist before, and the whole point. */
export type SourceStatus = 'unknown' | 'live' | 'stale';

const POLL_MS = 8_000;

class AttentionSource {
	readonly companyId: string;
	view = $state<AttentionView | null>(null);
	status = $state<SourceStatus>('unknown');
	/** The last failure, kept whole so callers can act on `.status === 401`. */
	failure = $state<(Error & { status?: number }) | null>(null);

	#consumers = 0;
	#timer: ReturnType<typeof setInterval> | undefined;
	#inFlight: Promise<void> | null = null;

	constructor(companyId: string) {
		this.companyId = companyId;
	}

	/** Coalesced: an action's post-write refresh joins a poll already running. */
	refresh(): Promise<void> {
		this.#inFlight ??= this.#load().finally(() => {
			this.#inFlight = null;
		});
		return this.#inFlight;
	}

	async #load(): Promise<void> {
		try {
			this.view = await getAttention(this.companyId);
			this.status = 'live';
			this.failure = null;
		} catch (cause) {
			this.failure = cause as Error & { status?: number };
			/* Keep the last good view. Only a source that has never answered
			 * stays `unknown` — everything else is stale, not absent. */
			this.status = this.view ? 'stale' : 'unknown';
		}
	}

	attach(): () => void {
		this.#consumers += 1;
		if (this.#consumers === 1) {
			void this.refresh();
			this.#timer = setInterval(() => void this.refresh(), POLL_MS);
		}
		let released = false;
		return () => {
			if (released) return;
			released = true;
			this.#consumers -= 1;
			if (this.#consumers === 0) {
				clearInterval(this.#timer);
				this.#timer = undefined;
			}
		};
	}
}

const sources = new Map<string, AttentionSource>();

/** The shared source for a company. Call `attach()` in an effect and release it. */
export function attentionSource(companyId: string): AttentionSource {
	let source = sources.get(companyId);
	if (!source) {
		source = new AttentionSource(companyId);
		sources.set(companyId, source);
	}
	return source;
}

export type { AttentionSource };
