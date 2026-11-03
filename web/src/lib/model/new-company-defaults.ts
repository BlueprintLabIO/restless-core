/* What a new company starts on (S107 B1): the owner's default model, as one of their account
 * connections and a model it serves, and their default decision model. New companies inherit
 * them when they are created; changing a default never changes a company that already exists. */
import { ownerRequest } from './setting.svelte';

export type NewCompanyDefaults = {
	model?: { connection: string; model: string } | null;
	/** `included`, an OpenRouter provider/model, or none (the included model, watching only). */
	decision_model?: string | null;
};

export const newCompanyDefaultsKey = ['new-company-defaults'] as const;
const PATH = '/api/account/new-company-defaults';

export function readNewCompanyDefaults(signal?: AbortSignal) {
	return ownerRequest<NewCompanyDefaults>(PATH, { signal });
}

export function saveNewCompanyDefaults(body: NewCompanyDefaults) {
	const clean: NewCompanyDefaults = {};
	if (body.model) clean.model = body.model;
	if (body.decision_model) clean.decision_model = body.decision_model;
	return ownerRequest<NewCompanyDefaults>(PATH, { method: 'PUT', json: clean });
}

export {
	INCLUDED_DECISIONS,
	decisionChoice,
	decisionSentence,
	type DecisionChoice
} from './new-company-decisions.ts';
