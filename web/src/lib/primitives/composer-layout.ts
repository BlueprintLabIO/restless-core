/* The composer's send control, as a decision rather than a pile of inline conditions.
 *
 * The button has to say two things at once: whether pressing it will do anything, and what
 * pressing it actually does. The second half is not decoration — with provider disclosure off
 * (UIR-009) a send records the message and nothing more, and a control that still said "Send"
 * would be promising an answer that is not coming. */

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
export function sendButtonLabel(input: {
	state: SendState;
	providerDisclosureEnabled: boolean;
}): string {
	if (input.state === 'disabled') return 'Sending is not available to you here';
	const verb = input.providerDisclosureEnabled ? 'Send' : 'Record only — nothing is sent';
	return input.state === 'empty' ? `${verb} (write something first)` : verb;
}
