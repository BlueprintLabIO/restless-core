/**
 * A company's model connections: its provider status (company keys and sign-ins) and the account's
 * reusable connections it may use. Both are cached reads; every save answers with the new state,
 * which the screen writes straight into the cache.
 */
import { getCompanies, type CompanyCatalogEntry } from './cockpit';
import { ownerRequest } from './setting.svelte';

export type ProviderConnection = {
	provider: string;
	reference: string | null;
	credential_status: string;
	credential_detail: string | null;
	gateway_loaded: boolean;
	shareable_api_key?: boolean;
};
export type ProviderStatus = {
	revision: string;
	primary_provider: string;
	connections: ProviderConnection[];
	infisical_configured: boolean;
	infisical_status: string;
	infisical_detail: string | null;
	startup_issue: string | null;
	/** What the company runs on by default, as provider/model. */
	primary_model?: string | null;
	/** The owner's ordered fallbacks, tried in turn when the main model cannot answer. */
	model_failover?: string[];
};
export type ReusableConnection = {
	id: string;
	label: string;
	provider: string;
	kind?: 'api_key' | 'oauth';
	status?: 'present' | 'absent' | 'invalid' | 'checking';
	detail?: string | null;
	companies: { id: string; name: string; in_use?: boolean }[];
};
export type ReusableConnections = {
	connections: ReusableConnection[];
	/** Providers the plan's included AI serves; they take no key of the owner's own. */
	includedProviders: string[];
	scope: 'account' | 'company';
	manageUrl: string;
	/** Active companies, for importing a key from one of them. */
	companies: CompanyCatalogEntry[];
};

const company = (id: string) => `/api/companies/${encodeURIComponent(id)}`;

export const providerKey = (id: string) => ['provider', id] as const;
export const reusableConnectionsKey = (id: string) => ['reusable-connections', id] as const;

export function readProvider(id: string, signal?: AbortSignal) {
	return ownerRequest<ProviderStatus>(`${company(id)}/provider`, { signal });
}

export function saveProvider(
	id: string,
	body: {
		provider: string;
		model: string;
		reference: string;
		secret?: string;
		revision: string;
		disconnect: boolean;
	}
) {
	return ownerRequest<ProviderStatus>(`${company(id)}/provider`, { method: 'PUT', json: body });
}

/** Replace the ordered fallback models; the answer is the new provider status. */
export function saveModelFailover(id: string, models: string[], revision: string) {
	return ownerRequest<ProviderStatus>(`${company(id)}/provider/failover`, {
		method: 'PUT',
		json: { models, revision }
	});
}

export async function readReusableConnections(signal?: AbortSignal): Promise<ReusableConnections> {
	const [body, companies] = await Promise.all([
		ownerRequest<{
			connections?: ReusableConnection[];
			included_providers?: string[];
			scope?: string;
			manage_url?: string;
		}>('/api/connections', { signal }),
		getCompanies()
	]);
	const scope = body.scope === 'company' ? 'company' : 'account';
	return {
		connections: body.connections ?? [],
		includedProviders: body.included_providers ?? [],
		scope,
		manageUrl:
			scope === 'company' ? (body.manage_url ?? '/account/settings/accounts') : '/account/settings/accounts',
		companies: companies.filter((row) => row.lifecycle_status === 'active')
	};
}

export async function createConnection(body: { label: string; provider: string; secret: string }) {
	const result = await ownerRequest<{ connection?: { id?: string }; id?: string }>(
		'/api/connections',
		{ method: 'POST', json: body }
	);
	return result.connection ?? result;
}

export async function importConnection(body: {
	source_company: string;
	provider: string;
	label: string;
}) {
	const result = await ownerRequest<{ connection?: { id?: string }; id?: string }>(
		'/api/connections/import',
		{ method: 'POST', json: body }
	);
	return result.connection ?? result;
}

/** Give a company one of the account's connections. A conflict with an existing company key asks
 * the owner to confirm replacing it instead of failing. */
export async function grantCompanyConnection(
	id: string,
	connection: string,
	body: { model: string; make_default: boolean; revision: string; replace_existing?: boolean }
): Promise<{ replaceRequired: true } | { replaceRequired: false; provider: ProviderStatus }> {
	try {
		const result = await ownerRequest<{ provider: ProviderStatus }>(
			`${company(id)}/connections/${encodeURIComponent(connection)}`,
			{ method: 'POST', json: body }
		);
		return { replaceRequired: false, provider: result.provider };
	} catch (cause) {
		const failure = cause as { status?: number; code?: string };
		if (failure?.status === 409 && failure.code === 'provider_conflict' && !body.replace_existing)
			return { replaceRequired: true };
		throw cause;
	}
}

/** Make one of the account's connections this company's default model. A company key of the same
 * provider is replaced only when `replace` says the company has nothing working to keep; otherwise
 * the owner is sent to Intelligence, where the replacement is theirs to confirm. */
export async function useAccountConnection(
	id: string,
	connection: string,
	model: string,
	replace: boolean
): Promise<ProviderStatus> {
	const { revision } = await readProvider(id);
	const result = await grantCompanyConnection(id, connection, { model, make_default: true, revision });
	if (!result.replaceRequired) return result.provider;
	if (!replace)
		throw new Error('This company already has its own key for that provider. Use Intelligence instead.');
	const fresh = await readProvider(id);
	const replaced = await grantCompanyConnection(id, connection, {
		model,
		make_default: true,
		revision: fresh.revision,
		replace_existing: true
	});
	if (replaced.replaceRequired) throw new Error('That connection could not replace the company key.');
	return replaced.provider;
}

export async function revokeCompanyConnection(id: string, connection: string, revision: string) {
	const result = await ownerRequest<{ provider: ProviderStatus }>(
		`${company(id)}/connections/${encodeURIComponent(connection)}`,
		{ method: 'DELETE', json: { revision } }
	);
	return result.provider;
}
