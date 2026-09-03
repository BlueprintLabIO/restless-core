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
	manageHref?(company: CompanyCatalogEntry): string;
	archive?(company: CompanyCatalogEntry): Promise<void>;
	restore?(company: CompanyCatalogEntry): Promise<void>;
}

export type WorkSurfaceStatus = 'proposed' | 'active' | 'blocked' | 'completed' | 'abandoned';
export type WorkSurfaceLens = 'map' | 'board';

export interface WorkSurfaceGoal {
	id: string;
	title: string;
	body: string;
	closedAt: string | null;
}

/** A source-neutral Work projection. Self-hosted Core derives it from
 * OrgIntel; a hosted runtime may derive it from its isolated durable store.
 * Neither adapter gets to replace the product-owned presentation. */
export interface WorkSurfaceItem {
	id: string;
	title: string;
	status: WorkSurfaceStatus;
	revision: number;
	priority: number;
	goalId: string | null;
	ownerId: string;
	ownerName: string;
	updatedAt: string;
	attemptState: string;
	artifactCount: number;
	gatesPassed: number;
	gatesTotal: number;
}

export interface WorkSurfaceEdge {
	id: string;
	fromWorkId: string;
	toWorkId: string;
	kind: 'requires' | 'revises';
}

export interface WorkSurfaceView {
	goals: WorkSurfaceGoal[];
	work: WorkSurfaceItem[];
	edges: WorkSurfaceEdge[];
}

export interface WorkSurfaceSelection {
	goalId: string;
	lens: WorkSurfaceLens;
}

export interface WorkSurfacePlatform {
	workHref(workId: string, selection: WorkSurfaceSelection): string;
}

export type WorkDetailAttemptState =
	| 'running'
	| 'produced'
	| 'changes_requested'
	| 'blocked'
	| 'failed'
	| 'abandoned'
	| 'superseded'
	| 'idle'
	| 'completed';

export interface WorkDetailAttempt {
	attemptNo: number;
	revision: number;
	state: WorkDetailAttemptState;
	summary: string;
	model: string | null;
	startedAt: string | null;
}

export interface WorkDetailArtifact {
	id: string;
	kind: string;
	label: string;
	note: string;
	uri: string;
	state: 'available' | 'stale' | 'missing' | 'superseded' | 'unknown';
	openHref?: string;
}

export interface WorkDetailGate {
	id: string;
	name: string;
	passed: boolean;
}

export interface WorkDetailRelation {
	id: string;
	title: string;
	revision: number;
	status: WorkSurfaceStatus;
	href: string;
}

export interface WorkDetailRecovery {
	summary: string;
	artifacts: WorkDetailArtifact[];
	preservedCandidate: WorkDetailArtifact | null;
}

/** Complete product-owned Work detail projection. Adapters translate their
 * durable source into this contract; presentation and evidence semantics stay
 * identical across self-hosted and hosted Core. */
export interface WorkDetailView {
	id: string;
	title: string;
	status: WorkSurfaceStatus;
	goalTitle: string;
	readerSummary: string;
	readerSummaryLabel: string;
	executionContract: string;
	ownerName: string;
	accountableLeadName: string;
	staffResponsibilityName: string | null;
	updatedAt: string;
	expectedArtifact: string;
	workspace: string;
	integrationBranch: string;
	attempt: WorkDetailAttempt | null;
	artifacts: WorkDetailArtifact[];
	gates: WorkDetailGate[];
	prerequisites: WorkDetailRelation[];
	dependents: WorkDetailRelation[];
	revisions: WorkDetailRelation[];
	recovery: WorkDetailRecovery | null;
}

export interface WorkDetailPlatform {
	backHref: string;
}
