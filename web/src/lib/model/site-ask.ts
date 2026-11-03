/* Whether an agent's question asks the owner to do something on a site, so the card shows the
 * company computer at the page the agent left open (register a domain, sign in, pay).
 *
 * A decision never does. On 3 November "Approve sending the four prospect emails" embedded a live
 * desktop showing an unrelated physio website and a "Restore pages?" dialog: the old test matched
 * any link or bare domain, and a decision about emails names addresses and sites. Links and
 * addresses are the decision's evidence, shown with it, not a task on a screen. */

const SITE_TASK = /\b(?:sign|log) ?in\b|\bregister\b|\bcheck ?out\b|\bpay(?:ment)?\b|\bverify\b/i;
const ON_THE_COMPUTER = /company (?:browser|computer)|shared desktop/i;
const DECISION =
	/^\s*(?:approve|allow|decline|decide|choose|confirm|review|ok(?:ay)?)\b|\bapprov(?:e|al)\b|\bpermission\b/i;

export function isSiteTask(ask: string, context = ''): boolean {
	if (DECISION.test(ask)) return false;
	return SITE_TASK.test(ask) || ON_THE_COMPUTER.test(`${ask} ${context}`);
}
