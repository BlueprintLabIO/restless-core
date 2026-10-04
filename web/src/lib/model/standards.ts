/* The quality bar belongs to the outcome: a Goal sets it, and each piece of
 * Work inherits its Goal's bar unless it states its own. It is separate from
 * how hard a model thinks, which belongs to the agent. */
import type { OutcomeStandard } from './company';
import { ownerJson } from './failure';

export const STANDARDS: { value: OutcomeStandard; label: string; title: string }[] = [
	{ value: 'fast', label: 'Fast', title: 'Good enough to move on; speed matters most.' },
	{ value: 'thorough', label: 'Thorough', title: 'Checked carefully before it is called done.' },
	{ value: 'exceptional', label: 'Exceptional', title: 'Clearly better than the usual result.' },
	{ value: 'frontier', label: 'Frontier', title: 'The best credible result, whatever it takes.' }
];

export function standardLabel(value: OutcomeStandard | null | undefined): string {
	return STANDARDS.find((standard) => standard.value === value)?.label ?? 'Exceptional';
}

function post(url: string, standard: OutcomeStandard | null) {
	return fetch(url, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ standard })
	}).then((response) => ownerJson<unknown>(response));
}

export function setGoalStandard(company: string, goal: string, standard: OutcomeStandard) {
	return post(
		`/api/companies/${encodeURIComponent(company)}/goals/${encodeURIComponent(goal)}/standard`,
		standard
	);
}

/** `null` returns the Work to its Goal's bar. */
export function setWorkStandard(company: string, work: string, standard: OutcomeStandard | null) {
	return post(
		`/api/companies/${encodeURIComponent(company)}/work/${encodeURIComponent(work)}/standard`,
		standard
	);
}
