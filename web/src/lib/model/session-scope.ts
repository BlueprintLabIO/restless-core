/* What this browser's session holds on this plane: the owner's account, one company, or nothing. */
export type SessionScope = {
	scope: 'account' | 'company' | 'none';
	company?: string | null;
	role?: string | null;
};

export async function sessionScope(): Promise<SessionScope | null> {
	try {
		const response = await fetch('/api/session/scope', { headers: { accept: 'application/json' } });
		return response.ok ? ((await response.json()) as SessionScope) : null;
	} catch {
		return null;
	}
}
