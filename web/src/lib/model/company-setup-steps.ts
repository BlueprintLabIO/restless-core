/* Setting up a new company is a mode of the cockpit (S96): the top bar's tabs are the setup's
 * steps until the owner confirms the read-back, then they become the company's own tabs. The
 * step comes from saved state, so a reload, another device or a return from Intelligence lands
 * where the owner left off. */

export type SetupStep = 'name' | 'model' | 'idea' | 'readback';

export const SETUP_STEP_LABEL: Record<SetupStep, string> = {
	name: 'Name',
	model: 'Model',
	idea: 'Your idea',
	readback: 'Read back'
};

/** The steps in order. Model is a step only while Exec has no model to think with. */
export function setupSteps(needsModel: boolean): SetupStep[] {
	return needsModel ? ['name', 'model', 'idea', 'readback'] : ['name', 'idea', 'readback'];
}

/** Where the owner is: the read-back once Exec has written one, else the name until it is
 * settled, then a model if there is none, then Exec's questions. */
export function currentSetupStep(state: {
	nameSettled: boolean;
	needsModel: boolean;
	readback: boolean;
}): SetupStep {
	if (state.readback) return 'readback';
	if (!state.nameSettled) return 'name';
	if (state.needsModel) return 'model';
	return 'idea';
}

/** The top bar's tabs for a step: earlier ones done, this one current. While Exec asks its
 * questions, the step counts them, so the interview is seen moving. */
export function setupTabs(
	step: SetupStep,
	needsModel: boolean,
	asked = 0
): { key: string; label: string; done: boolean; on: boolean }[] {
	const steps = setupSteps(needsModel || step === 'model');
	const at = steps.indexOf(step);
	return steps.map((key, index) => ({
		key: `setup-${key}`,
		label:
			key === 'idea' && index === at && asked > 0
				? `${SETUP_STEP_LABEL[key]} · ${asked}`
				: SETUP_STEP_LABEL[key],
		done: index < at,
		on: index === at
	}));
}

/** The setup conversation as this step reads it: who spoke, and what Exec's message carries. */
export type SetupMessage = {
	from: string;
	text: string;
	intent?: { ownerNeed?: string | null; readback?: unknown[] | null } | null;
};

/** Exec stopped before finishing a thought: its newest reply to the owner neither asks nor reads
 * back, so "Carry on" is offered. Exec's opening lines are written before the owner has said
 * anything; they are a greeting, not a reply, and a first setup never offers to carry on from
 * them (23 October). */
export function setupStoppedShort(messages: SetupMessage[]): boolean {
	const lastOwner = messages.findLastIndex((message) => message.from === 'you');
	if (lastOwner < 0) return false;
	const reply = messages.findLast((message, index) => message.from !== 'you' && index > lastOwner);
	if (!reply) return false;
	return !reply.intent?.readback?.length && !reply.intent?.ownerNeed?.trim();
}
