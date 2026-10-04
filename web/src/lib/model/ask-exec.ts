/* Any page can hand the owner to the Exec: open the rail on the general
 * conversation with the composer ready. The layout listens. */
export const ASK_EXEC_EVENT = 'restless:ask-exec';

export function askExec() {
	window.dispatchEvent(new CustomEvent(ASK_EXEC_EVENT));
}
