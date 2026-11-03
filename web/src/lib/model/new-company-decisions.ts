/* The decision default's choices in the owner's words (S107 B1). */
import { modelLabel } from './intelligence-labels.ts';

export const INCLUDED_DECISIONS = 'included';

export type DecisionChoice = 'included' | 'watching' | 'openrouter';

export function decisionChoice(model: string | null | undefined): DecisionChoice {
	if (!model) return 'watching';
	return model === INCLUDED_DECISIONS ? 'included' : 'openrouter';
}

/** The row's value, in the words the choices use. */
export function decisionSentence(model: string | null | undefined): string {
	switch (decisionChoice(model)) {
		case 'included':
			return 'Included';
		case 'watching':
			return 'Included, watching only';
		case 'openrouter':
			return `${modelLabel(model)} on OpenRouter`;
	}
}
