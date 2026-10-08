/* Connected tools (Sprint 57). The owner adds an MCP server by URL, command or
 * plugin bundle; Restless probes it and the owner grants each observed tool a
 * class. Authority owns every rule here; this module only reads and asks. */

import { responseFailure } from './failure.ts';

export type ToolClass = 'reads' | 'acts' | 'reserved';

export type ObservedTool = {
	name: string;
	title?: string | null;
	description?: string | null;
	annotations?: Record<string, unknown> | null;
	digest: string;
};

export type GrantedTool = {
	tool: string;
	class: ToolClass;
	party_args: string[];
	digest: string;
};

export type ConnectionGrant = {
	connection: string;
	grantee: string;
	tools: GrantedTool[];
	granted_by: string;
	granted_at: string;
	expires_at?: string | null;
};

export type ConnectionStatus =
	'awaiting_sign_in' | 'awaiting_probe' | 'working' | 'failed' | 'disconnected';

export type ToolConnection = {
	name: string;
	kind: 'remote' | 'local';
	endpoint?: string | null;
	command?: string | null;
	args: string[];
	auth_type: 'none' | 'bearer' | 'oauth';
	status: ConnectionStatus;
	frozen: boolean;
	/** A local tool allowed to drive the company computer's browser and its sign-ins. */
	browser?: boolean;
	account?: string | null;
	server_name?: string | null;
	server_version?: string | null;
	tools: ObservedTool[];
	failure?: string | null;
	source?: string | null;
	last_probe_at?: string | null;
	grants: ConnectionGrant[];
	proposed: GrantedTool[];
	changed: string[];
};

export type ToolDecision = { tool: string; class: ToolClass; party_args?: string[] | null };

export type NewConnection =
	| {
			kind: 'remote';
			name: string;
			endpoint: string;
			auth?:
				| { type: 'none' }
				| { type: 'bearer'; credential: string }
				| {
						type: 'oauth';
						credential: string;
						client_id?: string;
						client_secret?: string;
						scopes?: string[];
				  };
			source?: string;
	  }
	| {
			kind: 'local';
			name: string;
			command: string;
			args?: string[];
			env?: Record<string, string>;
			source?: string;
	  };

export type ToolReceipts = {
	reads: { id: string; actor: string; tool: string; status: string; observed_at: string }[];
	effects: {
		id: string;
		tool: string;
		purpose: string;
		success: boolean;
		actor: string;
		parties?: string[];
		created_at: string;
		outcome?: { status?: string };
	}[];
};

export type PluginImport = {
	import: {
		plugin: string;
		source: string;
		commit: string;
		connections: ToolConnection[];
		skipped: string[];
		skills: string[];
	};
	skills_requested: boolean;
};

/* Owner language for each class; the tooltip carries the consequence. */
export const CLASS_LABEL: Record<ToolClass, { label: string; title: string }> = {
	reads: {
		label: 'Reads freely',
		title: 'Agents call it as ordinary work. Each call leaves a read receipt.'
	},
	acts: {
		label: 'Acts with a receipt',
		title:
			'Each call is a governed effect with a receipt. The first contact with a new person waits for you; an unclear outcome is never retried blindly.'
	},
	reserved: {
		label: 'Asks you first',
		title: 'Every call waits in your Inbox with the exact request until you allow it.'
	}
};

function path(company: string, rest = ''): string {
	return `/api/companies/${encodeURIComponent(company)}/tool-connections${rest}`;
}

async function call<T>(url: string, init?: RequestInit): Promise<T> {
	const response = await fetch(url, {
		credentials: 'same-origin',
		...init,
		headers: init?.body ? { 'content-type': 'application/json' } : undefined
	});
	if (!response.ok) throw await responseFailure(response);
	return (await response.json()) as T;
}

const post = <T>(url: string, body: unknown = {}) =>
	call<T>(url, { method: 'POST', body: JSON.stringify(body) });

export async function fetchConnections(company: string): Promise<ToolConnection[]> {
	return (await call<{ connections: ToolConnection[] }>(path(company))).connections;
}

export const addConnection = (company: string, input: NewConnection) =>
	post<{ connection: ToolConnection }>(path(company), input);

export const importPlugin = (company: string, url: string) =>
	post<PluginImport>(path(company, '/plugins'), { url });

const named = (company: string, name: string, action: string) =>
	path(company, `/${encodeURIComponent(name)}/${action}`);

export const probeConnection = (company: string, name: string) =>
	post<{ connection: ToolConnection }>(named(company, name, 'probe'));

export async function signIn(company: string, name: string): Promise<string> {
	return (await post<{ authorization_url: string }>(named(company, name, 'sign-in')))
		.authorization_url;
}

export const grantTools = (company: string, name: string, tools: ToolDecision[]) =>
	post(named(company, name, 'grant'), { grantee: '*', tools });

export const revokeGrant = (company: string, name: string) =>
	post(named(company, name, 'revoke'), { grantee: '*' });

export const freezeConnection = (company: string, name: string, frozen: boolean) =>
	post(named(company, name, 'freeze'), { frozen });
export const setConnectionBrowser = (company: string, name: string, browser: boolean) =>
	post(named(company, name, 'browser'), { browser });

export const disconnectConnection = (company: string, name: string) =>
	post(named(company, name, 'disconnect'));

export const fetchReceipts = (company: string, name: string) =>
	call<ToolReceipts>(named(company, name, 'receipts'));

/* The class each tool has now: the live grant, else Exec's proposal. */
export function currentClasses(connection: ToolConnection): Record<string, GrantedTool> {
	const granted = connection.grants.find((grant) => grant.grantee === '*');
	const classes: Record<string, GrantedTool> = {};
	for (const proposal of connection.proposed) classes[proposal.tool] = proposal;
	for (const tool of granted?.tools ?? []) classes[tool.tool] = tool;
	return classes;
}

export function statusLabel(connection: ToolConnection): string {
	if (connection.frozen) return 'Frozen';
	return (
		{
			awaiting_sign_in: 'Needs sign-in',
			awaiting_probe: 'Not checked yet',
			working: 'Working',
			failed: 'Not working',
			disconnected: 'Disconnected'
		}[connection.status] ?? connection.status
	);
}

/* Adding an app is one step: add it, let the gateway check it, and allow it at once with Exec's
 * proposal (reads run; anything that acts asks the owner the first time). An app that needs a
 * sign-in comes back awaiting it, for the caller to start. */
export async function addAndAllow(company: string, input: NewConnection): Promise<ToolConnection> {
	const { connection } = await addConnection(company, input);
	const view = (await fetchConnections(company)).find((row) => row.name === connection.name);
	if (!view || view.status !== 'working' || !view.proposed.length) return view ?? connection;
	await grantTools(
		company,
		view.name,
		view.proposed.map(({ tool, class: kind, party_args }) => ({ tool, class: kind, party_args }))
	);
	return view;
}
