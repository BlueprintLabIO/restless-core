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

export type McpClient = 'claude-code' | 'claude-desktop' | 'codex';

export const MCP_CLIENTS: { value: McpClient; label: string; title: string }[] = [
	{ value: 'claude-code', label: 'Claude Code', title: 'Run it in a terminal.' },
	{
		value: 'claude-desktop',
		label: 'Claude Desktop',
		title: 'Add it to claude_desktop_config.json (Settings, Developer, Edit config), then restart.'
	},
	{ value: 'codex', label: 'Codex', title: 'Add it to ~/.codex/config.toml.' }
];

/* What adds Restless to each client: the same URL and bearer header, in each one's own form. */
export function clientSetup(client: McpClient, origin: string, secret: string): string {
	const url = `${origin}/mcp`;
	if (client === 'claude-desktop')
		return JSON.stringify(
			{
				mcpServers: {
					restless: {
						command: 'npx',
						args: ['-y', 'mcp-remote', url, '--header', `Authorization: Bearer ${secret}`]
					}
				}
			},
			null,
			2
		);
	if (client === 'codex')
		return `[mcp_servers.restless]\nurl = "${url}"\nhttp_headers = { Authorization = "Bearer ${secret}" }`;
	return `claude mcp add --transport http restless ${url} --header "Authorization: Bearer ${secret}"`;
}
