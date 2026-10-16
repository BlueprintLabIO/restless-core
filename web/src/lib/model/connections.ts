import { bootFetch } from './boot-fetch.ts';
/* Connected tools (Sprint 57): the services an app connects, such as Stripe. Not the account's
 * AI model accounts (account-connections.ts). The owner adds an MCP server by URL, command or
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
	/** What a local tool last printed to its error output, secrets removed. */
	last_output?: string | null;
	/** With no live grant: the last grant that ended (turned off or expired), and when. */
	ended_grant?: ConnectionGrant | null;
	ended_at?: string | null;
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
	reads: {
		id: string;
		actor: string;
		tool: string;
		status: string;
		observed_at: string;
		work_id?: string | null;
	}[];
	effects: {
		id: string;
		tool: string;
		purpose: string;
		success: boolean;
		actor: string;
		parties?: string[];
		created_at: string;
		work_id?: string | null;
		outcome?: { status?: string };
	}[];
	/** Actor id → display name, for who used it. */
	actors?: Record<string, string>;
	/** How far back this page reaches; a full page means there may be more. */
	limit?: number;
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
	const response = await (init?.method && init.method.toUpperCase() !== 'GET' ? fetch : bootFetch)(
		url,
		{
			credentials: 'same-origin',
			...init,
			headers: init?.body ? { 'content-type': 'application/json' } : undefined
		}
	);
	if (!response.ok) throw await responseFailure(response);
	return (await response.json()) as T;
}

const post = <T>(url: string, body: unknown = {}) =>
	call<T>(url, { method: 'POST', body: JSON.stringify(body) });

export async function fetchConnections(
	company: string,
	signal?: AbortSignal
): Promise<ToolConnection[]> {
	return (await call<{ connections: ToolConnection[] }>(path(company), { signal })).connections;
}

export const addConnection = (company: string, input: NewConnection) =>
	post<{ connection: ToolConnection }>(path(company), input);

export const importPlugin = (company: string, url: string) =>
	post<PluginImport>(path(company, '/plugins'), { url });

const named = (company: string, name: string, action: string) =>
	path(company, `/${encodeURIComponent(name)}/${action}`);

export const probeConnection = (company: string, name: string) =>
	post<{ connection: ToolConnection }>(named(company, name, 'probe'));

/** The app's sign-in page. Its server chose the address, so only a web page is returned. */
export async function signIn(company: string, name: string): Promise<string> {
	const url = (await post<{ authorization_url: string }>(named(company, name, 'sign-in')))
		.authorization_url;
	if (!/^https?:$/.test(new URL(url).protocol))
		throw new Error('This app’s sign-in page is not a web address, so it was not opened.');
	return url;
}

/* Where the owner was when a sign-in began (the Inbox, a conversation), kept across the
 * provider's redirect so the app page can lead back there. Per tab, and only a cockpit path. */
const returnKey = (company: string, name: string) => `sign-in-return:${company}:${name}`;
export function rememberSignInReturn(company: string, name: string, path: string): void {
	if (!path.startsWith(`/${encodeURIComponent(company)}`) || path.startsWith('//')) return;
	try {
		sessionStorage.setItem(returnKey(company, name), path);
	} catch {
		/* Without storage the app page simply has no way back to offer. */
	}
}
export function takeSignInReturn(company: string, name: string): string {
	try {
		const path = sessionStorage.getItem(returnKey(company, name)) ?? '';
		sessionStorage.removeItem(returnKey(company, name));
		return path;
	} catch {
		return '';
	}
}

/** Allow tools for everyone, until `expiresAt` when given. */
export const grantTools = (
	company: string,
	name: string,
	tools: ToolDecision[],
	expiresAt?: string | null
) =>
	post(named(company, name, 'grant'), {
		grantee: '*',
		tools,
		...(expiresAt ? { expires_at: expiresAt } : {})
	});

/** Change an online app's address or token in place; what it may do is kept. */
export const changeConnection = (
	company: string,
	name: string,
	change: { endpoint?: string; credential?: string }
) =>
	call<{ connection: ToolConnection }>(path(company, `/${encodeURIComponent(name)}`), {
		method: 'PATCH',
		body: JSON.stringify(change)
	});

export const revokeGrant = (company: string, name: string) =>
	post(named(company, name, 'revoke'), { grantee: '*' });

export const freezeConnection = (company: string, name: string, frozen: boolean) =>
	post(named(company, name, 'freeze'), { frozen });
export const setConnectionBrowser = (company: string, name: string, browser: boolean) =>
	post(named(company, name, 'browser'), { browser });

export const disconnectConnection = (company: string, name: string) =>
	post(named(company, name, 'disconnect'));

export const fetchReceipts = (company: string, name: string, limit = 20, signal?: AbortSignal) =>
	call<ToolReceipts>(`${named(company, name, 'receipts')}?limit=${limit}`, { signal });

/** Turned off on purpose: working, signed in, nothing allowed now, and a grant that ended. */
export function isOff(connection: ToolConnection): boolean {
	return connection.status === 'working' && !connection.grants.length && !!connection.ended_grant;
}

/** The tools to allow when an app is turned back on: what it allowed before, for each tool it
 * still offers, and Exec's proposal for any tool that is new since. */
export function restoredDecisions(connection: ToolConnection): ToolDecision[] {
	const before = new Map((connection.ended_grant?.tools ?? []).map((tool) => [tool.tool, tool]));
	const proposed = new Map(connection.proposed.map((tool) => [tool.tool, tool]));
	return connection.tools.map((tool) => {
		const earlier = before.get(tool.name) ?? proposed.get(tool.name);
		return {
			tool: tool.name,
			class: earlier?.class ?? 'reserved',
			party_args: earlier?.party_args ?? []
		};
	});
}

/** "4 read freely, 2 act with a receipt, 1 asks you first", for a summary of what was allowed. */
export function classSummary(tools: { class: ToolClass }[]): string {
	const counts = (['reads', 'acts', 'reserved'] as ToolClass[])
		.map((klass) => ({ klass, count: tools.filter((tool) => tool.class === klass).length }))
		.filter((row) => row.count > 0);
	const words: Record<ToolClass, [string, string]> = {
		reads: ['reads freely', 'read freely'],
		acts: ['acts with a receipt', 'act with a receipt'],
		reserved: ['asks you first', 'ask you first']
	};
	return counts
		.map(({ klass, count }) => `${count} ${words[klass][count === 1 ? 0 : 1]}`)
		.join(', ');
}

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

/* Answering an agent's app request is one step: add it, let the gateway check it, and allow it at
 * once with Exec's proposal (reads run; anything that acts asks the owner the first time), so the
 * waiting Work resumes. The caller says what was allowed. An app added any other way lands on its
 * permission list instead, where the owner sees each tool before allowing it. An app that needs a
 * sign-in comes back awaiting it, for the caller to start. */
export async function addAndAllow(
	company: string,
	input: NewConnection
): Promise<{ connection: ToolConnection; allowed: GrantedTool[] }> {
	const { connection } = await addConnection(company, input);
	const view = (await fetchConnections(company)).find((row) => row.name === connection.name);
	if (!view || view.status !== 'working' || !view.proposed.length)
		return { connection: view ?? connection, allowed: [] };
	await grantTools(
		company,
		view.name,
		view.proposed.map(({ tool, class: kind, party_args }) => ({ tool, class: kind, party_args }))
	);
	return { connection: view, allowed: view.proposed };
}
