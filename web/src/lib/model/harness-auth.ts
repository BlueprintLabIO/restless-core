/** Native harness sign-in (Codex, Claude) and the company vault, as the owner manages them. */
import { ownerRequest } from './setting.svelte';

export type HarnessConnection = {
	harness: string;
	configured?: boolean;
	mode: string;
	model: string;
	auth: {
		state: string;
		message?: string;
		verification_url?: string;
		user_code?: string;
		desktop?: boolean;
		account?: { email?: string };
	};
};

const base = (company: string) => `/api/companies/${encodeURIComponent(company)}`;

export const harnessAuthKey = (company: string) => ['harness-auth', company] as const;

export async function readHarnessAuth(company: string, signal?: AbortSignal) {
	const data = await ownerRequest<{ connections: HarnessConnection[] }>(
		`${base(company)}/harness-auth`,
		{ signal }
	);
	return data.connections;
}

export function harnessAuthAction(
	company: string,
	harness: string,
	body: { action: string; model: string; secret?: string }
) {
	return ownerRequest<HarnessConnection>(
		`${base(company)}/harness-auth/${encodeURIComponent(harness)}`,
		{ method: 'POST', json: body }
	);
}

export type VaultSecret = {
	name: string;
	path: string;
	reference: string;
	updated_at: string | null;
};
export type VaultReference = { name: string; reference: string };
export type VaultView = {
	status: string;
	secrets: VaultSecret[] | null;
	references: VaultReference[];
	revision: string;
	message?: string;
};

export const vaultKey = (company: string) => ['vault', company] as const;

export function readVault(company: string, signal?: AbortSignal) {
	return ownerRequest<VaultView>(`${base(company)}/vault`, { signal });
}

export function saveVaultSecret(
	company: string,
	body: { binding: string; secret: string; revision: string; create: boolean }
) {
	return ownerRequest<unknown>(`${base(company)}/vault/secret`, { method: 'POST', json: body });
}
