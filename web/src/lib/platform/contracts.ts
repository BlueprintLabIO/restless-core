import type { CompanyCatalogEntry, PortfolioProjection } from '../product/contracts';

export const PLATFORM_SCHEMA_VERSION = 1 as const;

export type PlatformMode = 'self_hosted' | 'cloud_fleet' | 'cloud_company';
export type MembershipRole = 'owner' | 'admin' | 'member';
export type PlatformCapability =
	| 'company.create'
	| 'company.open'
	| 'company.manage'
	| 'company.archive'
	| 'company.restore'
	| 'account.support'
	| 'account.sign_out';

export interface PlatformIdentity {
	userId: string;
	displayName: string;
	role: MembershipRole;
}

export type PlatformScope = { kind: 'owner' } | { kind: 'company'; companyId: string };

export interface PlatformNavigation {
	portfolioHref: string;
	supportHref?: string;
	signOutHref?: string;
}

export interface CoreReleaseIdentity {
	core_version: string;
	source_revision: string;
	api_contract_version: number;
	assertion_contract_version: number;
	schema_version: number;
}

/**
 * The only deployment-specific input accepted by the canonical SPA.
 *
 * Cloud may implement this contract in its private BFF, but private endpoints,
 * infrastructure identities, credentials and signing material are forbidden
 * from the response. Fleet-only projections are optional because self-hosted
 * Core already owns the live company projection APIs.
 */
export interface PlatformContext {
	schemaVersion: typeof PLATFORM_SCHEMA_VERSION;
	mode: PlatformMode;
	identity: PlatformIdentity;
	scope: PlatformScope;
	capabilities: PlatformCapability[];
	navigation: PlatformNavigation;
	release?: CoreReleaseIdentity;
	companies?: CompanyCatalogEntry[];
	projections?: Record<string, PortfolioProjection>;
}

const MODES = new Set<PlatformMode>(['self_hosted', 'cloud_fleet', 'cloud_company']);
const ROLES = new Set<MembershipRole>(['owner', 'admin', 'member']);
const CAPABILITIES = new Set<PlatformCapability>([
	'company.create',
	'company.open',
	'company.manage',
	'company.archive',
	'company.restore',
	'account.support',
	'account.sign_out'
]);
const RUNTIME_STATUSES = new Set([
	'pending',
	'provisioning',
	'running',
	'stopped',
	'sleeping',
	'scaling',
	'replacing',
	'deleting',
	'failed',
	'absent',
	'unavailable'
]);
const LIFECYCLE_STATUSES = new Set(['active', 'archived', 'deleting']);

function record(value: unknown): Record<string, unknown> | null {
	return value !== null && typeof value === 'object' && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: null;
}

function nonEmpty(value: unknown): value is string {
	return typeof value === 'string' && value.trim().length > 0;
}

function safeSameOriginPath(value: unknown): value is string {
	return nonEmpty(value) && value.startsWith('/') && !value.startsWith('//');
}

function validCompany(value: unknown): value is CompanyCatalogEntry {
	const company = record(value);
	return Boolean(
		company &&
		nonEmpty(company.id) &&
		nonEmpty(company.name) &&
		typeof company.mission === 'string' &&
		typeof company.model === 'string' &&
		(company.spend_ceiling_usd === null ||
			(typeof company.spend_ceiling_usd === 'number' &&
				Number.isFinite(company.spend_ceiling_usd) &&
				company.spend_ceiling_usd >= 0)) &&
		RUNTIME_STATUSES.has(company.runtime_status as string) &&
		LIFECYCLE_STATUSES.has(company.lifecycle_status as string) &&
		(company.role === undefined || ROLES.has(company.role as MembershipRole)) &&
		(company.unstartable_reason === undefined || typeof company.unstartable_reason === 'string')
	);
}

function validProjection(value: unknown): value is PortfolioProjection {
	const projection = record(value);
	return Boolean(
		projection &&
		(projection.attentionCount === null ||
			(typeof projection.attentionCount === 'number' &&
				Number.isInteger(projection.attentionCount) &&
				projection.attentionCount >= 0)) &&
		(projection.nextProof === null || typeof projection.nextProof === 'string') &&
		typeof projection.nextProofDetail === 'string' &&
		(projection.spendAccounted === null ||
			(typeof projection.spendAccounted === 'number' &&
				Number.isFinite(projection.spendAccounted) &&
				projection.spendAccounted >= 0))
	);
}

function validCloudPortfolio(context: Record<string, unknown>): boolean {
	if (context.mode !== 'cloud_fleet') {
		return context.companies === undefined && context.projections === undefined;
	}
	if (!Array.isArray(context.companies) || !context.companies.every(validCompany)) return false;
	const ids = context.companies.map((company) => company.id);
	if (new Set(ids).size !== ids.length) return false;
	const projections = record(context.projections);
	if (!projections || Object.keys(projections).some((id) => !ids.includes(id))) return false;
	return ids.every((id) => validProjection(projections[id]));
}

/** Fail closed on an incompatible platform response instead of silently
 * rendering a locally assumed capability. */
export function parsePlatformContext(value: unknown): PlatformContext {
	const context = record(value);
	const identity = record(context?.identity);
	const scope = record(context?.scope);
	const navigation = record(context?.navigation);
	if (
		!context ||
		context.schemaVersion !== PLATFORM_SCHEMA_VERSION ||
		!MODES.has(context.mode as PlatformMode) ||
		!identity ||
		!nonEmpty(identity.userId) ||
		!nonEmpty(identity.displayName) ||
		!ROLES.has(identity.role as MembershipRole) ||
		!scope ||
		(scope.kind !== 'owner' && scope.kind !== 'company') ||
		(scope.kind === 'company' && !nonEmpty(scope.companyId)) ||
		!Array.isArray(context.capabilities) ||
		!context.capabilities.every((capability) =>
			CAPABILITIES.has(capability as PlatformCapability)
		) ||
		new Set(context.capabilities).size !== context.capabilities.length ||
		!navigation ||
		!safeSameOriginPath(navigation.portfolioHref) ||
		(navigation.supportHref !== undefined && !safeSameOriginPath(navigation.supportHref)) ||
		(navigation.signOutHref !== undefined && !safeSameOriginPath(navigation.signOutHref)) ||
		!validCloudPortfolio(context)
	) {
		throw new Error('The platform response is incompatible with this Core release.');
	}
	return context as unknown as PlatformContext;
}
