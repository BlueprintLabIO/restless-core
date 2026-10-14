import { browserControl } from './attention';
import { failureSentence } from './failure';

/** Quiet control is handed back after a minute so an absent owner never strands the computer. */
export const IDLE_RELEASE_MS = 60_000;
/** The owner is warned this long before, with a way to keep control. */
export const IDLE_WARNING_MS = 15_000;
const RENEW_MS = 8_000;

/**
 * One tab's hold on the shared company desktop, the same in the Inbox handover and on the Computer
 * page. The desktop connection itself never changes: taking or returning control only flips who
 * may send input, so the view stays connected across a hand-back.
 */
export class DesktopControl {
	controller = $state<'observer' | 'owner'>('observer');
	leaseId = $state('');
	working = $state<'' | 'control' | 'return'>('');
	/** Seconds left before an idle hand-back, while the warning shows; otherwise 0. */
	idleSeconds = $state(0);
	error = $state('');
	#lastActivity = 0;
	#lastRenewal = 0;
	#renewing = false;
	readonly #company: () => string;
	readonly #client: () => string;
	readonly #onchange: () => void;

	constructor(company: () => string, client: () => string, onchange: () => void = () => {}) {
		this.#company = company;
		this.#client = client;
		this.#onchange = onchange;
	}

	get owner(): boolean {
		return this.controller === 'owner';
	}

	/** Take keyboard and pointer for this tab. Resolves true when the lease is granted. */
	async take(silent = false): Promise<boolean> {
		const client = this.#client();
		if (!client) return false;
		if (this.owner) return true;
		if (this.working) return false;
		this.working = 'control';
		this.error = '';
		try {
			const control = await browserControl(this.#company(), 'take', client);
			this.leaseId = control.lease_id ?? '';
			this.controller = 'owner';
			this.#lastActivity = Date.now();
			this.#lastRenewal = Date.now();
			this.idleSeconds = 0;
			return true;
		} catch (cause) {
			if (!silent) this.error = failureSentence(cause, 'Control is held elsewhere.');
			return false;
		} finally {
			this.working = '';
			this.#onchange();
		}
	}

	/** Hand input back. An idle hand-back tells the agent nothing, so it keeps waiting. */
	async release(idle = false): Promise<void> {
		const client = this.#client();
		if (!client || !this.owner || this.working) return;
		this.working = 'return';
		this.error = '';
		try {
			await browserControl(this.#company(), 'return', client, this.leaseId, idle);
			this.lose();
		} catch (cause) {
			// The lease ends on its own a minute after the last input, so a failed idle
			// hand-back still leaves this tab an observer rather than retrying forever.
			if (idle) this.lose();
			else this.error = failureSentence(cause, 'Control could not be returned.');
		} finally {
			this.working = '';
			this.#onchange();
		}
	}

	/** Forget the lease locally, when the server says this tab no longer holds it. */
	lose(message = ''): void {
		this.controller = 'observer';
		this.leaseId = '';
		this.idleSeconds = 0;
		this.#lastActivity = 0;
		if (message) this.error = message;
	}

	/** A real input reached the desktop: keep the lease alive, at most every few seconds. */
	activity(): void {
		const now = Date.now();
		this.#lastActivity = now;
		this.idleSeconds = 0;
		if (!this.owner || this.#renewing || now - this.#lastRenewal < RENEW_MS) return;
		this.#renew(now);
	}

	/** The owner is still here, only not typing: renew now and clear the warning. */
	stillHere(): void {
		if (!this.owner) return;
		const now = Date.now();
		this.#lastActivity = now;
		this.idleSeconds = 0;
		if (!this.#renewing) this.#renew(now);
	}

	#renew(now: number) {
		this.#renewing = true;
		this.#lastRenewal = now;
		void browserControl(this.#company(), 'heartbeat', this.#client(), this.leaseId)
			.then(() => this.#onchange())
			.catch((cause) => {
				this.lose(failureSentence(cause, 'Desktop control expired.'));
				this.#onchange();
			})
			.finally(() => (this.#renewing = false));
	}

	/** Count down quiet control; returns the cleanup for onMount. */
	watch(): () => void {
		const timer = window.setInterval(() => {
			if (!this.owner || this.#lastActivity === 0 || this.working) {
				this.idleSeconds = 0;
				return;
			}
			const left = IDLE_RELEASE_MS - (Date.now() - this.#lastActivity);
			if (left <= 0) void this.release(true);
			else this.idleSeconds = left <= IDLE_WARNING_MS ? Math.ceil(left / 1000) : 0;
		}, 1_000);
		return () => window.clearInterval(timer);
	}
}
