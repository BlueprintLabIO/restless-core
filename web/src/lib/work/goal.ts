import type { CockpitGoal } from '$lib/model/generated/cockpit';
import type { CollaborationGoal } from '$lib/model/collaboration';

/* An owner reads a Goal's finish line and due date; a member's Goal carries only its description. */
export type Goal = CockpitGoal | CollaborationGoal;

export function finishLine(goal: Goal): string {
	return 'done_when' in goal ? goal.done_when.trim() : '';
}

export function dueLabel(goal: Goal): string {
	return 'due_on' in goal && goal.due_on
		? new Date(`${goal.due_on}T00:00:00`).toLocaleDateString(undefined, {
				day: 'numeric',
				month: 'short'
			})
		: '';
}

/* What a Goal is for, when it is done and when it is due, for the info mark beside it. A hover
 * title alone never reaches a touch screen. */
export function goalAbout(goal: Goal): string {
	return [
		goal.body.trim(),
		'done_when' in goal
			? finishLine(goal)
				? `Done when ${finishLine(goal)}`
				: 'No finish line yet: Exec will agree one with you'
			: '',
		dueLabel(goal) ? `Due ${dueLabel(goal)}` : ''
	]
		.filter(Boolean)
		.join(' · ');
}
