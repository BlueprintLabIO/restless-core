/* Models by job (S102, S107): the everyday model for Staff chat and routine first tasks, and the
 * decision model that lets a routine result skip the Exec's wake-up. Saved through the same
 * route `restless company set` writes, owner only. */
import { ownerRequest } from './setting.svelte';
import { modelLabel } from './intelligence-labels';

export type DecisionMode = 'shadow' | 'active' | 'unavailable';

export type ModelJobs = {
	revision: string;
	primary_model: string | null;
	/** As stored: a provider/model, `off`, or null for the default. */
	everyday_model: string | null;
	/** In effect: null when every turn runs on the primary. */
	everyday: string | null;
	/** `included`, an OpenRouter provider/model, or null (the included model, watching only). */
	decision_model: string | null;
	decision_mode: DecisionMode;
	/** Whether this plane has the included decision model at all. */
	included_decisions: boolean;
};

const path = (company: string) => `/api/companies/${encodeURIComponent(company)}/model-jobs`;

export function readModelJobs(company: string, signal?: AbortSignal) {
	return ownerRequest<ModelJobs>(path(company), { signal });
}

export function saveModelJobs(
	company: string,
	body: { everyday_model?: string; decision_model?: string; revision: string }
) {
	return ownerRequest<ModelJobs>(path(company), { method: 'PUT', json: body });
}

export const EVERYDAY_OFF = 'off';
export const INCLUDED_DECISIONS = 'included';

/** "Claude Sonnet 5.5", or what running everything on the main model means. */
export function everydayLabel(model: string | null | undefined, primary: string | null): string {
	if (!model || model === EVERYDAY_OFF)
		return primary ? `Same as main (${modelLabel(primary)})` : 'Same as main model';
	return modelLabel(model);
}

export function decisionLabel(model: string | null | undefined): string {
	if (!model) return 'Included, watching only';
	if (model === INCLUDED_DECISIONS) return 'Included';
	return `${modelLabel(model)} on OpenRouter`;
}
