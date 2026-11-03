/**
 * How soon a change hint may read the same thing again.
 *
 * Every read a hosted cockpit makes goes through the edge router, which is billed per request.
 * A company at work sends a hint every few seconds (a progress estimate, a file, a message), and
 * each hint used to read everything on screen again: three idle tabs made about 45 requests a
 * minute each and the daily cap took restless.run down (3 Nov 2026).
 *
 * So a query a hint touches reads at once the first time, then waits longer each time the hints
 * keep coming, up to a ceiling. The ceiling is short while the person is using the page and long
 * while nobody has touched it; the moment they touch it again, everything held back reads at once
 * (`catchUp`). A quiet spell resets the wait. Pure, so its bounds are tested without a browser.
 */

/** The first wait after an immediate read; each further read in the same run doubles it. */
export const HINT_STEP_MS = 3_000;
/** The longest a hinted query waits while the person is using the page. */
export const HINT_ACTIVE_CEILING_MS = 10_000;
/** The longest a hinted query waits while nobody has touched the page. */
export const HINT_IDLE_CEILING_MS = 60_000;
/** No input for this long and the page counts as untouched. */
export const IDLE_AFTER_MS = 60_000;
/** A query not read for a hint in this long starts its run again: the next hint reads at once. */
export const HINT_QUIET_MS = 2 * HINT_IDLE_CEILING_MS;

type Run = { at: number; streak: number };

export class HintPace {
	private runs = new Map<string, Run>();

	/** How long a query hinted now must wait before it reads; 0 means read now. */
	wait(key: string, now: number, idle: boolean): number {
		const run = this.runs.get(key);
		if (!run || now - run.at >= HINT_QUIET_MS) return 0;
		const ceiling = idle ? HINT_IDLE_CEILING_MS : HINT_ACTIVE_CEILING_MS;
		const gap = Math.min(HINT_STEP_MS * 2 ** run.streak, ceiling);
		return Math.max(0, run.at + gap - now);
	}

	/** Record that the query read for a hint now. */
	read(key: string, now: number): void {
		const run = this.runs.get(key);
		const continuing = run && now - run.at < HINT_QUIET_MS;
		this.runs.set(key, { at: now, streak: continuing ? run.streak + 1 : 0 });
	}

	forget(key: string): void {
		this.runs.delete(key);
	}
}
