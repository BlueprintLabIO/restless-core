/* The owner plane's accounts API (S98-01), as one client the Accounts page is given, so a
 * gallery or a test can hand it another. */
import { ownerRequest } from './setting.svelte';
import type {
	Abilities,
	Account,
	AccountKind,
	AccountUse,
	Grant,
	GrantInput
} from './owner-accounts';

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
	uses(id: string, query?: UseQuery): Promise<UsePage>;
	/** "Fine" or "Freeze" on a notice (S98-10). Freeze pauses that company's grant. */
	decideNotice(id: string, notice: string, decision: 'fine' | 'freeze'): Promise<AccountUse>;
	/** The owner signs in by hand in the account's managed browser (S98-11). */
	signIn(id: string): Promise<{ session: string; desktop_url: string }>;
	signInDone(id: string, session: string): Promise<Account>;
	signInCancel(id: string, session: string): Promise<void>;
	/** Settles an admin's request once the owner saved a grant for it (S98-09). */
	settleRequest(
		request: string,
		body: { decision: 'approved' | 'denied'; grant_id?: string }
	): Promise<void>;
};

export type UseQuery = {
	company?: string;
	actor?: string;
	directed_by?: string;
	tier?: string;
	before?: string;
	limit?: number;
};
export type UsePage = { uses: AccountUse[]; next_before: string | null };

function useSearch(query: UseQuery): string {
	const params = new URLSearchParams();
	for (const [key, value] of Object.entries(query))
		if (value !== undefined && value !== '') params.set(key, String(value));
	const search = params.toString();
	return search ? `?${search}` : '';
}

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
	uses: (id, query = {}) => ownerRequest<UsePage>(`${one(id)}/uses${useSearch(query)}`),
	decideNotice: (id, notice, decision) =>
		ownerRequest<AccountUse>(`${one(id)}/notices/${encodeURIComponent(notice)}`, {
			method: 'POST',
			json: { decision }
		}),
	signIn: (id) =>
		ownerRequest<{ session: string; desktop_url: string }>(`${one(id)}/sign-in`, {
			method: 'POST'
		}),
	signInDone: (id, session) =>
		ownerRequest<Account>(`${one(id)}/sign-in/${encodeURIComponent(session)}/done`, {
			method: 'POST'
		}),
	signInCancel: async (id, session) => {
		await ownerRequest<unknown>(`${one(id)}/sign-in/${encodeURIComponent(session)}`, {
			method: 'DELETE'
		});
	},
	settleRequest: async (request, body) => {
		await ownerRequest<unknown>(`${base}/requests/${encodeURIComponent(request)}`, {
			method: 'POST',
			json: body
		});
	}
};

/** A grant as the company sees it: what it may do with which account (S98-08). Any member. */
export type CompanyAccountGrant = Grant & {
	account: Pick<Account, 'id' | 'kind' | 'label' | 'provider' | 'health'>;
};

export async function companyAccounts(company: string): Promise<CompanyAccountGrant[]> {
	return (
		await ownerRequest<{ grants: CompanyAccountGrant[] }>(
			`/api/companies/${encodeURIComponent(company)}/accounts`
		)
	).grants;
}

/** An admin asks the owner for an account, or for more on one; it lands in the owner's Inbox. */
export function requestCompanyAccount(
	company: string,
	body: ({ account_id: string } | { kind: AccountKind }) & { abilities: Abilities; reason: string }
) {
	return ownerRequest<unknown>(`/api/companies/${encodeURIComponent(company)}/accounts/requests`, {
		method: 'POST',
		json: body
	});
}

/** The owner gives a sign-in code an agent is waiting on (S98-11). Core types it into the waiting
 * page and never stores it, which is why it does not go through the ordinary Inbox reply. */
export function answerAccountCode(company: string, request: string, code: string) {
	return ownerRequest<{ typed: boolean; message: string }>(
		`/api/companies/${encodeURIComponent(company)}/accounts/codes/${encodeURIComponent(request)}`,
		{ method: 'POST', json: { code } }
	);
}

/** What agents in this company did with its accounts, newest first (S98-09). Any member. */
export function companyAccountUses(
	company: string,
	query: { before?: string; limit?: number } = {}
) {
	return ownerRequest<UsePage>(
		`/api/companies/${encodeURIComponent(company)}/accounts/uses${useSearch(query)}`
	);
}
