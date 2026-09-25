import type { WorkStatus } from '$lib/model/generated/orgintel';

/* One name per Work state, used by the map, the board, the detail page and
 * every relation link. Before this the same proposed Work read "Next" on the
 * map, "Not started" on its detail page and "proposed" on the board. */
export const WORK_STATUS_LABEL: Record<WorkStatus, string> = {
	proposed: 'Next',
	active: 'In motion',
	blocked: 'Waiting',
	completed: 'Done',
	abandoned: 'Stopped'
};

export function workStatusLabel(status: string): string {
	return WORK_STATUS_LABEL[status as WorkStatus] ?? status.replaceAll('_', ' ');
}

/* The latest run's state, or nothing when no run has started: an absent run
 * is already implied by the Work state and must not be a second status. */
export function runStateLabel(state: string | null | undefined): string {
	return state ? state.replaceAll('_', ' ') : '';
}
