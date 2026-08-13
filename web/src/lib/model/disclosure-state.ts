/* What the composer says about provider disclosure — and, mostly, that it says nothing.
 *
 * Provider disclosure used to be a control in the composer: a two-option radiogroup on chats
 * ("Ask" / "Record only") and a checkbox on the rail, both writing `providerDisclosureAccepted`
 * on every single message. That put a standing data-custody consent in the position of a send
 * mode, and made the safe answer one you had to re-choose forever. It is company settings now
 * (UIR-009), so the composer carries no control at all.
 *
 * What it must still do is show the **non-default state**. If disclosure is off and the composer
 * looks ordinary, you type a question, press send, and silently get no answer — today's status
 * line lands only after the message is recorded, which is too late to be a warning. So: nothing
 * when disclosure is on, and something unmissable when it is off. */

export type ComposerDisclosure =
	{ kind: 'silent' } | { kind: 'record-only'; message: string; settingsHref: string };

/**
 * Whether the composer shows a disclosure state, and what it says.
 *
 * `hasResponder` matters because the notice is about an answer that will not arrive. With no
 * connected runtime there is no answer to lose — the composer already says so in its own empty
 * state — and a second notice about a provider that is not there would be noise.
 */
export function composerDisclosure(input: {
	providerDisclosureEnabled: boolean;
	hasResponder: boolean;
	companyId: string;
}): ComposerDisclosure {
	if (input.providerDisclosureEnabled) return { kind: 'silent' };
	if (!input.hasResponder) return { kind: 'silent' };
	return {
		kind: 'record-only',
		message: "Record only — this company doesn't send to a provider.",
		settingsHref: `/${input.companyId}/settings/ai-and-data`
	};
}

/**
 * The send button's accessible name.
 *
 * When nothing will be sent onward, the button should not promise that it will. "Send" and
 * "Record only" are different acts and the control has to name the one it performs.
 */
export function sendLabel(providerDisclosureEnabled: boolean): string {
	return providerDisclosureEnabled ? 'Send' : 'Record only';
}
