/* Personal tokens for an outside MCP client (Claude Code, Claude Desktop,
 * Codex). Each token acts as the person who made it, with that person's
 * membership checked on every call. */

import { responseFailure } from './failure.ts';

export type AccessToken = {
	id: string;
	label: string;
	created_at: string;
	last_used_at?: string | null;
};

async function call<T>(url: string, init?: RequestInit): Promise<T> {
	const response = await fetch(url, {
		credentials: 'same-origin',
		...init,
		headers: init?.body ? { 'content-type': 'application/json' } : undefined
	});
	if (!response.ok) throw await responseFailure(response);
	return (await response.json()) as T;
}

export async function fetchTokens(): Promise<AccessToken[]> {
	return (await call<{ tokens: AccessToken[] }>('/api/mcp-access')).tokens;
}

export const issueToken = (label: string) =>
	call<{ token: AccessToken; secret: string }>('/api/mcp-access', {
		method: 'POST',
		body: JSON.stringify({ label })
	});

export const revokeToken = (id: string) =>
	call(`/api/mcp-access/${encodeURIComponent(id)}/revoke`, { method: 'POST' });

/* The one command that adds Restless to Claude Code. */
export function claudeCodeCommand(origin: string, secret: string): string {
	return `claude mcp add --transport http restless ${origin}/mcp --header "Authorization: Bearer ${secret}"`;
}
