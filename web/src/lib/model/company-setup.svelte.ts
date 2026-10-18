/* What the setup page tells the cockpit around it (S96): which step the owner is on, for the top
 * bar's tabs, and where the setup chat sat when the owner confirmed, so Exec's rail can grow
 * out of it instead of appearing somewhere else. */
import type { SetupStep } from './company-setup-steps';

export const companySetup = $state<{ step: SetupStep; needsModel: boolean; asked: number }>({
	step: 'name',
	needsModel: false,
	asked: 0
});

export type SetupHandoff = { rect: DOMRect; chat: HTMLElement };
let handoff: SetupHandoff | null = null;

/** The setup chat as it hands over to the cockpit: where it sat, and a still copy of it. */
export function rememberSetupHandoff(next: SetupHandoff) {
	handoff = next;
}

/** Read once: the copy morphs into Exec's rail the first time the rail appears after setup. */
export function takeSetupHandoff(): SetupHandoff | null {
	const taken = handoff;
	handoff = null;
	return taken;
}

/* The name step is settled on this device once the owner continues past it. Exec's later
 * messages settle it everywhere: an owner who has answered a question is past the name. */
const settledKey = (companyId: string) => `restless:${companyId}:setup-name`;

export function nameSettledHere(companyId: string): boolean {
	try {
		return localStorage.getItem(settledKey(companyId)) === '1';
	} catch {
		return false;
	}
}

export function settleNameHere(companyId: string) {
	try {
		localStorage.setItem(settledKey(companyId), '1');
	} catch {
		/* Only this visit remembers it; the page keeps its own state. */
	}
}
