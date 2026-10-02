/* Why a company cannot start, said as the one action that fixes it. The
 * portfolio and the company's own Attention surface state the same blocker in
 * the same words, so neither can call the company clear while the other says
 * it is stuck. */
export function providerName(model: string): string {
	const provider = model.split('/')[0];
	if (provider.includes('codex')) return 'ChatGPT / Codex';
	if (provider.includes('claude')) return 'Claude Code';
	return (
		(
			{
				openai: 'OpenAI',
				anthropic: 'Anthropic',
				google: 'Google Gemini',
				litellm: 'model gateway'
			} as Record<string, string>
		)[provider] ?? 'intelligence provider'
	);
}
function startAction(reason: string): string {
	const normalized = reason.toLowerCase();
	if (reason.startsWith('Choose an intelligence provider'))
		return 'Choose an intelligence provider and model';
	if (normalized.includes('codex')) return 'Reconnect ChatGPT / Codex';
	if (normalized.includes('claude')) return 'Reconnect Claude Code';
	if (reason.startsWith('no usable host credential')) {
		const model = reason.match(/for model ([^ ]+)/)?.[1];
		return model
			? `Reconnect ${providerName(model)}`
			: 'Reconnect the selected intelligence provider';
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
	return startAction(reason);
}

export function startFixHref(companyId: string): string {
	return `/${encodeURIComponent(companyId)}/company/provider`;
}
