/* The composer's send control, as a decision rather than inline conditions. */

export type SendState = 'ready' | 'empty' | 'disabled';

/**
 * Whether the send button can be pressed, and why not when it cannot.
 *
 * `empty` and `disabled` both render inert, but they are different facts: `empty` is the
 * ordinary resting state of a composer nobody has typed in, and `disabled` means this reader
 * may not post here at all. Keeping them apart lets the button explain itself.
 */
export function sendState(input: {
	disabled: boolean;
	value: string;
	minlength: number;
}): SendState {
	if (input.disabled) return 'disabled';
	return input.value.trim().length >= input.minlength ? 'ready' : 'empty';
}

/**
 * The send button's accessible name.
 *
 * Deliberately not "Send message" in every case. When nothing will be transmitted onward the
 * button performs a different act, and a screen reader should hear the act, not the icon.
 */
export function sendButtonLabel(state: SendState): string {
	if (state === 'disabled') return 'Sending is not available to you here';
	return state === 'empty' ? 'Send (write something first)' : 'Send';
}
