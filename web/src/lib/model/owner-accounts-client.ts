/* The owner plane's accounts API (S98-01), as one client the Accounts page is given, so a
 * gallery or a test can hand it another. */
import { ownerRequest } from './setting.svelte';
import type { Account, AccountKind, AccountUse, Grant, GrantInput } from './owner-accounts';

export type NewAccount = {
	kind: AccountKind;
	label: string;
	provider?: string | null;
	personal?: boolean;
	/** Kind-specific; secret fields go straight to the vault and are never read back. */
	credential: Record<string, unknown>;
};

export type AccountsClient = {
	list(): Promise<Account[]>;
	create(input: NewAccount): Promise<Account>;
	update(id: string, patch: { label?: string; frozen?: boolean }): Promise<Account>;
	remove(id: string): Promise<void>;
	grant(id: string, input: GrantInput & { company: string }): Promise<Grant>;
	updateGrant(id: string, grant: string, patch: GrantInput): Promise<Grant>;
	removeGrant(id: string, grant: string): Promise<void>;
	uses(
		id: string,
		query?: { company?: string; before?: string; limit?: number }
	): Promise<{
		uses: AccountUse[];
		next_before: string | null;
	}>;
};

const base = '/api/accounts';
const one = (id: string) => `${base}/${encodeURIComponent(id)}`;

export const ownerAccounts: AccountsClient = {
	list: async () => (await ownerRequest<{ accounts: Account[] }>(base)).accounts,
	create: (input) => ownerRequest<Account>(base, { method: 'POST', json: input }),
	update: (id, patch) => ownerRequest<Account>(one(id), { method: 'PATCH', json: patch }),
	remove: async (id) => {
		await ownerRequest<unknown>(one(id), { method: 'DELETE' });
	},
	grant: (id, input) => ownerRequest<Grant>(`${one(id)}/grants`, { method: 'POST', json: input }),
	updateGrant: (id, grant, patch) =>
		ownerRequest<Grant>(`${one(id)}/grants/${encodeURIComponent(grant)}`, {
			method: 'PATCH',
			json: patch
		}),
	removeGrant: async (id, grant) => {
		await ownerRequest<unknown>(`${one(id)}/grants/${encodeURIComponent(grant)}`, {
			method: 'DELETE'
		});
	},
	uses: (id, query = {}) => {
		const params = new URLSearchParams();
		if (query.company) params.set('company', query.company);
		if (query.before) params.set('before', query.before);
		if (query.limit) params.set('limit', String(query.limit));
		const search = params.toString();
		return ownerRequest(`${one(id)}/uses${search ? `?${search}` : ''}`);
	}
};
