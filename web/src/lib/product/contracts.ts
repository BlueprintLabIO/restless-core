export type CompanyRuntimeStatus =
	| 'pending'
	| 'provisioning'
	| 'running'
	| 'stopped'
	| 'sleeping'
	| 'scaling'
	| 'replacing'
	| 'deleting'
	| 'failed'
	| 'absent'
	| 'unavailable';

export type CompanyLifecycleStatus = 'active' | 'archived' | 'deleting';

/** Canonical company identity rendered by the Core-owned portfolio. Platform
 * adapters may add control-plane facts, but they may not replace this shape
 * with a deployment-specific company card. */
export interface CompanyCatalogEntry {
	id: string;
	name: string;
	mission: string;
	model: string;
	spend_ceiling_usd: number | null;
	runtime_status: CompanyRuntimeStatus;
	lifecycle_status: CompanyLifecycleStatus;
	role?: 'owner' | 'admin' | 'member';
	unstartable_reason?: string;
}

export type PortfolioProjection = {
	attentionCount: number | null;
	nextProof: string | null;
	nextProofDetail: string;
	spendAccounted: number | null;
};

export type ProductNotice = {
	title: string;
	detail: string;
};

/** Authority-bearing behavior is supplied by the current platform. The
 * shared UI never imports a Cloud database client or a self-hosted daemon. */
export interface CompanyPortfolioPlatform {
	companyHref(company: CompanyCatalogEntry): string;
	archive(company: CompanyCatalogEntry): Promise<void>;
	restore(company: CompanyCatalogEntry): Promise<void>;
}
