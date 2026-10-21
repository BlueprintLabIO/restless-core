/* Account headroom (S100): how full each AI account's limit windows are, when they reset, who used
 * them, and when they run out at today's pace. Read from what the accounts report on calls the
 * companies were making anyway; an account that reports nothing is unknown, never guessed. */
import { ownerRequest } from './setting.svelte';

export interface UsageWindow {
	name: string;
	label: string;
	used_percent: number;
	window_minutes: number | null;
	resets_at: string | null;
	observed_at: string;
	runs_out_at: string | null;
	by_company: [string, number][];
}

export interface AccountUsage {
	account: string;
	windows: UsageWindow[];
}

export async function readAccountUsage(): Promise<AccountUsage[]> {
	return (await ownerRequest<{ accounts: AccountUsage[] }>('/api/accounts/usage')).accounts;
}

export interface CompanyHeadroom {
	model: string | null;
	account: string | null;
	/** The sign-in's own name, so the meters say whose limits they are. */
	account_label?: string | null;
	windows: UsageWindow[];
}

export function readCompanyHeadroom(company: string): Promise<CompanyHeadroom> {
	return ownerRequest<CompanyHeadroom>(`/api/companies/${encodeURIComponent(company)}/model/headroom`);
}

export { connectionOf, freshness, meterTone, resetsIn, timeOfDay } from './account-usage-view';
