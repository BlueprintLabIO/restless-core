/* Why a company cannot start, said as the one action that fixes it. The
 * portfolio and the company's own Attention surface state the same blocker in
 * the same words, so neither can call the company clear while the other says
 * it is stuck. */
export function startGuidance(reason: string): string {
	/* The Company page is called Intelligence; say its name the way the
	 * navigation does. */
	if (reason.startsWith('Choose an intelligence provider')) {
		return 'Choose an intelligence provider and model in Company → Intelligence.';
	}
	if (reason.startsWith('no usable host credential for model native-codex-oauth/')) {
		return 'Check Codex sign-in and select Codex in Company → Intelligence.';
	}
	if (
		reason.startsWith('no usable host credential') ||
		reason.startsWith('Claude Agent requires')
	) {
		return 'Connect the selected intelligence provider in Company → Intelligence.';
	}
	return 'Check the intelligence setup in Company → Intelligence.';
}

export function startFixHref(companyId: string): string {
	return `/${encodeURIComponent(companyId)}/company/provider`;
}
