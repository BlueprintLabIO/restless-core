import { createInfiniteQuery } from '@tanstack/svelte-query';
import { ownerJson } from './failure';

export interface ModelTotals {
	calls: number;
	tokens: number;
	input_tokens: number;
	output_tokens: number;
	cached_input_tokens: number;
	calls_without_tokens: number;
	calls_without_split: number;
	calls_without_cache: number;
	calls_without_estimate: number;
	unknown_charges: number;
	subscription_calls: number;
	/** Tokens through a subscription sign-in. Their limit is the plan's windows, not dollars. */
	subscription_tokens?: number;
	subscription_input_tokens?: number;
	subscription_cached_input_tokens?: number;
	subscription_output_tokens?: number;
	recorded_usd: number;
	provider_reported_usd: number | null;
	/** API-equivalent value of calls billed per token; subscription calls are not in it. */
	estimated_usd: number | null;
}
/** One usage window of a sign-in the company uses, as last reported by the provider. */
export interface PlanWindow {
	account: string;
	provider: string;
	label: string;
	used_percent: number;
	resets_at: string | null;
	observed_at: string;
	company_points: number | null;
}
export interface SpendGroup {
	key: string;
	label: string;
	totals: ModelTotals;
}
export interface ModelSpendRow {
	id: string;
	occurred_at: string;
	actor: string;
	person: string;
	model: string;
	responsibility: string;
	session_id: string;
	work_id: string | null;
	attempt_id: string | null;
	tokens: number | null;
	input_tokens: number | null;
	output_tokens: number | null;
	cached_input_tokens: number | null;
	reasoning_output_tokens: number | null;
	billing: string;
	billing_inferred: boolean;
	recorded_usd: number | null;
	cost_basis: string;
	estimated_usd: number | null;
	estimate_basis: string | null;
	/** A subscription call's value at API rates, for comparison only. */
	api_equivalent_usd?: number | null;
}
export interface SpendReport {
	observed_at: string;
	since: string | null;
	totals: ModelTotals;
	by_actor: SpendGroup[];
	by_model: SpendGroup[];
	by_day: SpendGroup[];
	activity: ModelSpendRow[];
	next_before: string | null;
	payments: Array<{
		id: string;
		purpose: string;
		actor: string;
		work_id: string;
		amount_minor: number;
		currency: string;
		state: string;
		occurred_at: string;
	}>;
	payment_totals: Array<{ currency: string; settled_minor: number; pending_minor: number }>;
	payments_status: 'available' | 'unavailable';
	pricing_updated_at: number | null;
	/** Current catalogue rates (per million tokens) for the models in this report. */
	prices?: Array<{
		model: string;
		currency: string;
		input_per_million: number;
		output_per_million: number;
		cached_input_per_million: number | null;
	}>;
	/** The subscription plans this company's calls count against. */
	plan?: PlanWindow[];
	/** Team leads' calls in the period and how many ran on the everyday model; absent when none. */
	lead_turns?: LeadSplit | null;
	coverage: string[];
}
export interface LeadSplit {
	calls: number;
	everyday_calls: number;
	tokens: number;
	everyday_tokens: number;
	everyday_model: string;
}
/** "Lead turns · 70% everyday": the share of team leads' calls on the everyday model. */
export function leadSplitLine(split: LeadSplit | null | undefined): string | null {
	if (!split || !split.calls) return null;
	return `Lead turns · ${Math.round((100 * split.everyday_calls) / split.calls)}% everyday`;
}
export interface SpendFilters {
	days: number;
	actor?: string;
	model?: string;
}

export async function getSpendReport(
	company: string,
	filters: SpendFilters,
	before?: string,
	signal?: AbortSignal
): Promise<SpendReport> {
	const query = new URLSearchParams({ days: String(filters.days) });
	if (filters.actor) query.set('actor', filters.actor);
	if (filters.model) query.set('model', filters.model);
	if (before) query.set('before', before);
	return ownerJson(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/spend/report?${query}`, {
			credentials: 'same-origin',
			cache: 'no-store',
			signal
		})
	);
}

/** One company/filter cache owns the whole result. Old-company and cancelled reads cannot land
 * in the new page, and a refresh keeps the current report visible rather than flashing a skeleton. */
export function spendReportQuery(
	company: () => string,
	filters: () => SpendFilters,
	enabled: () => boolean,
	identity: () => readonly (string | undefined)[]
) {
	const query = createInfiniteQuery(() => ({
		queryKey: ['company-spend-report', company(), identity(), filters()],
		initialPageParam: undefined as string | undefined,
		queryFn: ({ pageParam, signal }) => getSpendReport(company(), filters(), pageParam, signal),
		getNextPageParam: (lastPage: SpendReport) => lastPage.next_before ?? undefined,
		enabled: !!company() && enabled(),
		staleTime: 30_000,
		gcTime: 5 * 60_000,
		retry: false,
		refetchOnWindowFocus: false,
		refetchInterval: 60_000,
		refetchIntervalInBackground: false
	}));
	return {
		get view() {
			return query.data?.pages[0] ?? null;
		},
		get rows() {
			return query.data?.pages.flatMap((page) => page.activity) ?? [];
		},
		get failure() {
			return query.error;
		},
		get reading() {
			return query.isFetching;
		},
		get more() {
			return query.hasNextPage;
		},
		get readingMore() {
			return query.isFetchingNextPage;
		},
		refresh: () => query.refetch(),
		loadMore: () => query.fetchNextPage()
	};
}

export function usd(value: number | null): string {
	if (value == null) return 'Unknown';
	if (value > 0 && value < 0.0001) return '<US$0.0001';
	return new Intl.NumberFormat(undefined, {
		style: 'currency',
		currency: 'USD',
		currencyDisplay: 'narrowSymbol',
		minimumFractionDigits: 2,
		maximumFractionDigits: 4
	}).format(value);
}
export const tokenCount = (value: number | null) =>
	value == null ? 'Unknown' : new Intl.NumberFormat().format(value);
/** "Weekly 42% used · this company 18 pts", or the window alone when no share was sampled. */
export function planLine(window: PlanWindow): string {
	const used = `${window.label} ${Math.round(window.used_percent)}% used`;
	return window.company_points == null
		? used
		: `${used} · this company ${Math.round(window.company_points)} pts`;
}
/** A group's or a call's money column: subscription use is not shown as dollars. */
export function usdOrPlan(value: number | null, subscriptionCalls: number, calls: number): string {
	if (value == null && calls > 0 && subscriptionCalls >= calls) return 'Subscription';
	return usd(value);
}
export function paymentAmount(minor: number, currency: string): string {
	try {
		const format = new Intl.NumberFormat(undefined, { style: 'currency', currency });
		const digits = format.resolvedOptions().maximumFractionDigits ?? 2;
		return format.format(minor / 10 ** digits);
	} catch {
		return `${minor} minor units ${currency}`;
	}
}
export function costBasis(row: ModelSpendRow): string {
	if (row.billing === 'subscription')
		return row.billing_inferred
			? 'Subscription route · inferred from older record'
			: 'Included in subscription';
	if (row.billing === 'native_api_unmetered') return 'External API billing';
	return (
		(
			{
				provider_reported: 'Provider-reported charge',
				published_tariff: 'Published-tariff estimate',
				legacy_recorded: 'Recorded · pricing provenance unavailable',
				unreported: 'Charge unknown'
			} as Record<string, string>
		)[row.cost_basis] ?? 'Not separately charged'
	);
}
