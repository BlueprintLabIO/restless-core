/* Why a company cannot start, said as the one action that fixes it. The
 * portfolio and the company's own Attention surface state the same blocker in
 * the same words, so neither can call the company clear while the other says
 * it is stuck. */
function startAction(reason: string): string {
	if (reason.startsWith('Choose an intelligence provider')) {
		return 'Choose an intelligence provider and model';
	}
	if (reason.startsWith('no usable host credential for model native-codex-oauth/')) {
		return 'Check Codex sign-in and select Codex';
	}
	if (
		reason.startsWith('no usable host credential') ||
		reason.startsWith('Claude Agent requires')
	) {
		return 'Connect the selected intelligence provider';
	}
	return 'Check the intelligence setup';
}

/** The action and where to take it, for text that does not itself link there.
 * The Company page is called Intelligence; say its name the way the
 * navigation does. */
export function startGuidance(reason: string): string {
	return `${startAction(reason)} in Company → Intelligence.`;
}

/** The action alone, for a control that already opens the page that fixes it. */
export function startLinkLabel(reason: string): string {
	return `${startAction(reason)}.`;
}

export function startFixHref(companyId: string): string {
	return `/${encodeURIComponent(companyId)}/company/provider`;
}
