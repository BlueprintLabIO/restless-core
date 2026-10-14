/**
 * The account's model connections (API keys and sign-ins), shared across its companies, and the
 * requests that add, remove, sign in and grant them. The screen reads the list through the cache;
 * every write answers, then the list is read again.
 */
import { ownerRequest } from './setting.svelte';
import type { ProviderStatus } from './provider';

export type CompanyUse = {
	id: string;
	name: string;
	in_use?: boolean;
	/** The models this company runs through the connection. */
	models?: string[];
};
export type AccountConnection = {
	id: string;
	label: string;
	provider: string;
	kind?: 'api_key' | 'oauth';
	account_identity?: string;
	status?: 'present' | 'absent' | 'invalid' | 'checking';
	detail?: string | null;
	companies: CompanyUse[];
};
export type AccountConnectionsView = {
	connections: AccountConnection[];
	includedProviders: string[];
	scope: 'account' | 'company';
	manageUrl: string;
};
export type SignInJob = {
	state: string;
	url?: string;
	code?: string;
	message?: string;
};

export const accountConnectionsKey = ['account-connections-view'] as const;

export async function readAccountConnections(
	signal?: AbortSignal
): Promise<AccountConnectionsView> {
	const body = await ownerRequest<{
		connections?: AccountConnection[];
		included_providers?: string[];
		scope?: string;
		manage_url?: string;
	}>('/api/connections', { signal });
	return {
		connections: body.connections ?? [],
		includedProviders: body.included_providers ?? [],
		scope: body.scope === 'company' ? 'company' : 'account',
		manageUrl: body.manage_url ?? '/account/connections'
	};
}

export function addApiKeyConnection(body: { label: string; provider: string; secret: string }) {
	return ownerRequest<{ unverified?: boolean }>('/api/connections', { method: 'POST', json: body });
}

export function removeAccountConnection(id: string) {
	return ownerRequest<{ vault_deleted?: boolean }>(`/api/connections/${encodeURIComponent(id)}`, {
		method: 'DELETE'
	});
}

export function importCompanyCodexSignIn(company: string) {
	return ownerRequest<unknown>('/api/connections/import/company-codex', {
		method: 'POST',
		json: { company }
	});
}

export function startOAuthSignIn(provider: 'codex' | 'claude') {
	return ownerRequest<{ job: string }>(`/api/connections/oauth/${provider}`, { method: 'POST' });
}

export function readSignInJob(job: string) {
	return ownerRequest<SignInJob>(`/api/connections/oauth/jobs/${encodeURIComponent(job)}`);
}

export function completeSignInCallback(job: string, callbackUrl: string) {
	return ownerRequest<unknown>(`/api/connections/oauth/jobs/${encodeURIComponent(job)}/callback`, {
		method: 'POST',
		json: { callback_url: callbackUrl }
	});
}

const company = (id: string) => `/api/companies/${encodeURIComponent(id)}`;

export async function companyProviderRevision(id: string): Promise<string> {
	return (await ownerRequest<ProviderStatus>(`${company(id)}/provider`)).revision;
}

export function grantAccountConnection(
	companyId: string,
	connection: string,
	body: { model: string; make_default: boolean; revision: string; replace_existing?: boolean }
) {
	return ownerRequest<{ provider?: ProviderStatus }>(
		`${company(companyId)}/connections/${encodeURIComponent(connection)}`,
		{ method: 'POST', json: body }
	);
}

export function revokeAccountConnection(companyId: string, connection: string, revision: string) {
	return ownerRequest<{ provider?: ProviderStatus }>(
		`${company(companyId)}/connections/${encodeURIComponent(connection)}`,
		{ method: 'DELETE', json: { revision } }
	);
}

/** Whether a company already has a default model; a company with none takes its first grant. */
export async function companyHasDefaultModel(id: string): Promise<boolean | null> {
	try {
		const view = await ownerRequest<{ default?: unknown; base_default?: unknown }>(
			`${company(id)}/intelligence`
		);
		return Boolean(view.default ?? view.base_default);
	} catch {
		return null;
	}
}
