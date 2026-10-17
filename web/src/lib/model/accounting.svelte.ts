import { createQuery, useQueryClient } from '@tanstack/svelte-query';
import { ownerJson } from './failure';
import type { AccountingInput, AccountingRecord } from './accounting';
export interface AccountingReport {
	records: AccountingRecord[];
	payments: {
		id: string;
		purpose: string;
		work_id: string;
		amount_minor: number;
		currency: string;
		state: string;
		occurred_at: string;
	}[];
	payments_status: 'available' | 'unavailable';
}
export function accountingReportQuery(
	company: () => string,
	enabled: () => boolean,
	identity: () => readonly (string | undefined)[]
) {
	const key = () => ['company-accounting', company(), identity()];
	const client = useQueryClient();
	const query = createQuery(() => ({
		queryKey: key(),
		queryFn: async ({ signal }) =>
			ownerJson<AccountingReport>(
				await fetch(`/api/companies/${encodeURIComponent(company())}/company/accounting`, {
					signal,
					credentials: 'same-origin',
					cache: 'no-store'
				})
			),
		enabled: !!company() && enabled(),
		staleTime: 30000,
		retry: false,
		refetchOnWindowFocus: false
	}));
	return {
		get view() {
			return query.data;
		},
		get failure() {
			return query.error;
		},
		get reading() {
			return query.isFetching;
		},
		refresh: () => query.refetch(),
		async save(id: string, version: number, record: AccountingInput) {
			const target = company(),
				cacheKey = key();
			const result = await ownerJson<AccountingRecord>(
				await fetch(`/api/companies/${encodeURIComponent(target)}/company/accounting/${id}`, {
					method: 'PUT',
					credentials: 'same-origin',
					headers: { 'Content-Type': 'application/json' },
					body: JSON.stringify({ version, record })
				})
			);
			client.setQueryData<AccountingReport>(cacheKey, (current) =>
				current
					? {
							...current,
							records: [result, ...current.records.filter((row) => row.id !== result.id)].sort(
								(a, b) => b.occurred_on.localeCompare(a.occurred_on) || a.id.localeCompare(b.id)
							)
						}
					: undefined
			);
			void client.invalidateQueries({ queryKey: cacheKey });
			return result;
		}
	};
}
