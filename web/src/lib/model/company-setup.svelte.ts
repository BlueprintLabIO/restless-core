/* What the setup page tells the cockpit around it (S96): which step the owner is on, for the top
 * bar's tabs, and where the setup chat sat when the owner confirmed, so Exec's rail can grow
 * out of it instead of appearing somewhere else. */
import type { SetupStep } from './company-setup-steps';

export const companySetup = $state<{ step: SetupStep; needsModel: boolean }>({
	step: 'name',
	needsModel: false
});

let handoff: DOMRect | null = null;

/** The setup chat's place on screen as it hands over to the cockpit. */
export function rememberSetupHandoff(rect: DOMRect) {
	handoff = rect;
}

/** Read once: the rail grows from it the first time it appears after setup. */
export function takeSetupHandoff(): DOMRect | null {
	const rect = handoff;
	handoff = null;
	return rect;
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
