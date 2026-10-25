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
		)[provider] ?? 'model provider'
	);
}
/** The connection works but does not serve the chosen model, so the fix is a
 * model choice in Company → Intelligence, not the connection. */
export function needsAnotherModel(reason: string): boolean {
	return reason.toLowerCase().includes('not available on this connection');
}
function startAction(reason: string): string {
	const normalized = reason.toLowerCase();
	if (reason.startsWith('Connect a model') || reason.startsWith('Choose an intelligence provider'))
		return 'Connect a model';
	if (normalized.includes('rejected the key')) return 'Replace the rejected key';
	if (needsAnotherModel(reason)) return 'Choose another model';
	if (normalized.includes('codex')) return 'Reconnect ChatGPT / Codex';
	if (normalized.includes('claude')) return 'Reconnect Claude Code';
	const unavailable = reason.match(/^The connection for (\S+) is unavailable\./)?.[1];
	if (unavailable) return `Reconnect ${providerName(unavailable)}`;
	return 'Check the model setup';
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

/** An account connection as the account Connections API lists it. */
export interface AccountConnectionSummary {
	id: string;
	label: string;
	provider: string;
	kind?: 'api_key' | 'oauth';
	status?: 'present' | 'absent' | 'invalid' | 'checking';
	companies: { id: string }[];
}

const ACCOUNT_PROVIDER: Record<string, string> = {
	codex: 'openai-codex',
	claude: 'anthropic'
};

/* "Reconnect ChatGPT / Codex" is the wrong instruction when the account
 * already holds a working sign-in for that provider and this company simply
 * has not been given it: nothing needs reconnecting. Then the fix is one
 * grant, made on the account Connections page with the company preselected. */
export function accountGrantFix(
	reason: string,
	companyId: string,
	connections: AccountConnectionSummary[]
): { label: string; href: string; connection: AccountConnectionSummary } | null {
	if (needsAnotherModel(reason)) return null;
	const normalized = reason.toLowerCase();
	const harness = normalized.includes('codex')
		? 'codex'
		: normalized.includes('claude')
			? 'claude'
			: null;
	if (!harness) return null;
	const connection = connections.find(
		(item) =>
			item.provider === ACCOUNT_PROVIDER[harness] &&
			item.status === 'present' &&
			!item.companies.some((use) => use.id === companyId)
	);
	if (!connection) return null;
	const name = harness === 'codex' ? 'ChatGPT / Codex' : 'Claude';
	return {
		label: `Use your ${name} sign-in`,
		href: `/account/settings/accounts?grant=${encodeURIComponent(companyId)}&connection=${encodeURIComponent(connection.id)}`,
		connection
	};
}

/** What the owner does next with an account connection that works but no company may use yet.
 * Signing in and giving a company access are separate steps, so a working sign-in with no
 * company says so and offers the grant, never another sign-in. The intended company is the one
 * waiting on exactly this connection, else the only company; with several, the owner chooses.
 * The step only opens the grant form: nothing is granted until the owner confirms it there. */
export function emptyAccessStep(
	connection: AccountConnectionSummary,
	companies: { id: string; name: string; unstartable_reason?: string | null }[],
	connections: AccountConnectionSummary[]
): { sentence: string; action: string | null; companyId: string | null } | null {
	if (connection.status !== 'present' || connection.companies.length) return null;
	const works = connection.kind === 'api_key' ? 'Key works.' : 'Signed in.';
	if (!companies.length)
		return { sentence: `${works} No company to give it to yet.`, action: null, companyId: null };
	const waiting = companies.find(
		(company) =>
			!!company.unstartable_reason &&
			accountGrantFix(company.unstartable_reason, company.id, connections)?.connection.id ===
				connection.id
	);
	const intended = waiting ?? (companies.length === 1 ? companies[0] : null);
	return {
		sentence: `${works} No company uses it yet.`,
		action: intended ? `Give ${intended.name} access` : 'Choose a company',
		companyId: intended?.id ?? null
	};
}

export async function getAccountConnections(
	signal?: AbortSignal
): Promise<AccountConnectionSummary[]> {
	const response = await fetch('/api/connections', { cache: 'no-store', signal });
	if (!response.ok) return [];
	const body = (await response.json()) as { connections?: AccountConnectionSummary[] };
	return body.connections ?? [];
}
