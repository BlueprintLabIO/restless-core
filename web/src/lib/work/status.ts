import type { WorkAttemptState, WorkStatus } from '$lib/model/generated/orgintel';

/* One name per Work state, used by the map, the board, the detail page and
 * every relation link. Before this the same proposed Work read "Next" on the
 * map, "Not started" on its detail page and "proposed" on the board. Blocked
 * Work is "Blocked" everywhere; "Needs you" is only Work waiting on the viewer. */
export const WORK_STATUS_LABEL: Record<WorkStatus, string> = {
	proposed: 'Next',
	active: 'In motion',
	blocked: 'Blocked',
	completed: 'Done',
	abandoned: 'Stopped'
};

export function workStatusLabel(status: string): string {
	return WORK_STATUS_LABEL[status as WorkStatus] ?? status.replaceAll('_', ' ');
}

/* A run's state in owner words, or nothing when no run has started: an absent
 * run is already implied by the Work state and must not be a second status. */
const RUN_STATE_LABEL: Record<WorkAttemptState, string> = {
	running: 'Running',
	produced: 'Finished',
	changes_requested: 'Sent back for changes',
	blocked: 'Blocked',
	failed: 'Failed',
	abandoned: 'Stopped',
	superseded: 'Replaced by a later run'
};
export function runStateLabel(state: string | null | undefined): string {
	return state ? (RUN_STATE_LABEL[state as WorkAttemptState] ?? state.replaceAll('_', ' ')) : '';
}
