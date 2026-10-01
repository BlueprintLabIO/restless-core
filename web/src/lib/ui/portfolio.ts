/** Authority and transport stay in the adapter; these are observed display values. */
export type PortfolioAction =
	| { href: string; label?: string }
	| { formAction: string; fields: Record<string, string>; label?: string };

export interface CompanyPortfolioEntry {
	id: string;
	name: string;
	status: string;
	tone?: 'presence' | 'waiting' | 'unavailable';
	focus?: string | null;
	next?: string | null;
	nextHint?: string;
	attentionCount?: number | null;
	needsYou?: string;
	issue?: string;
	entry?: PortfolioAction | null;
}
