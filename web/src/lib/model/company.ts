import type { MoneyEnvelope } from './cockpit';
import { getApplianceStatus } from './appliance';
import { ownerJson } from './failure.ts';

export type CompanySourceStatus = 'available' | 'unavailable' | 'stale' | 'absent';

export interface CompanySourceObservation {
	status: CompanySourceStatus;
	observed_at: string;
	detail?: string;
}

export type OutcomeStandard = 'fast' | 'thorough' | 'exceptional' | 'frontier';
export type AgentHarness = 'restless-managed' | 'codex' | 'claude-agent';

export interface CompanyView {
	company: { id: string; name: string };
	sources: Record<'authority' | 'orgintel' | 'runtime', CompanySourceObservation>;
	charter: {
		purpose: string;
		source: string;
		revision: string;
		effective_at?: string;
		legal_identity?: {
			legal_name: string;
			trading_name?: string | null;
			entity_type: string;
			jurisdiction: string;
			owner_asserted_at: string;
		};
		current_direction?: {
			id: string;
			title: string;
			body: string;
			href: string;
			observed_at: string;
		};
		current_direction_status: string;
	};
	limits: {
		status: string;
		independently: CompanyLimitStatement[];
		asks_owner: CompanyLimitStatement[];
		cannot: CompanyLimitStatement[];
		approved_parties: string[];
		spend: {
			model: string;
			accounted_usd: number;
			ceiling_usd: number;
			remaining_usd?: number | null;
			status: 'available' | 'exhausted' | 'metering_unknown';
			/** One running total, or a budget for each UTC month. */
			period: 'total' | 'monthly';
			period_start?: string | null;
			/** The share of the limit, in percent, at which the Inbox warns. */
			warning_percent: number;
			/** Spend since the company started, by actor and model, largest first. */
			by_actor?: Array<{ actor: string; model: string; spent_usd: number }>;
			/** Model calls that reported no price, newest first; they make the total a lower bound. */
			unpriced?: Array<{ actor: string; model: string; occurred_at: string }>;
			/** What the last seven days cost, for the weekly review. */
			last_7_days_usd?: number;
		};
		/** Native-harness use (Codex or Claude on their own sign-in) this UTC month. */
		native: {
			month_utc: string;
			turns: number;
			tokens: number;
			/** Turns with no reported token count; `tokens` is then a floor. */
			turns_without_tokens: number;
			/** The harnesses' own reported price; null when none reported one. */
			estimated_usd: number | null;
			turns_without_estimate: number;
			monthly_turn_limit: number | null;
			monthly_token_limit: number | null;
			status: 'available' | 'exhausted';
		};
		runtime: {
			/** Stored setting: null follows the default, 0 never sleeps. */
			auto_sleep_after_minutes: number | null;
			/** The timeout the company actually sleeps after; null never sleeps. */
			sleep_after_minutes: number | null;
			/** Stopped by the sleep policy, so owed work wakes it. */
			asleep: boolean;
			monthly_runtime_cap_hours: number | null;
			usage_status: 'available' | 'unavailable';
			usage: {
				status: 'running' | 'stopped' | 'absent';
				month_utc: string;
				used_seconds: number;
				monthly_cap_hours: number | null;
				remaining_seconds: number | null;
				complete: boolean;
				observed_at: string;
			} | null;
		};
		money_envelopes: MoneyEnvelope[];
		/** The payment provider as the owner recorded it; whether it works is a live check. */
		payment_provider?: PaymentProvider | null;
		/** The owner's company-wide pause of outside actions, while it is in force. */
		outside_actions_paused?: {
			paused_at: string;
			paused_by: string | null;
			frozen_connections: string[];
			frozen_currencies: string[];
		} | null;
	};
	harnesses: {
		coordination: AgentHarness;
		worker: AgentHarness;
		options: AgentHarnessOption[];
	};
	resources: {
		status: string;
		items: CompanyResource[];
	};
	external_actions: {
		status: string;
		items: CompanyExternalAction[];
	};
	computer: {
		doctor: CompanyDoctor;
		/** The newest recovery of the computer: an owner Rebuild or a plane's release roll-forward. */
		last_recovery?: {
			action?: string;
			state?: 'requested' | 'failed' | 'succeeded';
			error?: string;
			recorded_at?: string;
			actor?: string | null;
		};
		runtime?: RuntimeDoctor;
		generation?: string;
	};
	attention_href: string;
	refreshed_at: string;
}

export interface AgentHarnessOption {
	id: AgentHarness;
	label: string;
	transport: string;
	expected_build: string;
	observed_build?: string;
	native_agent_build?: string;
	status: 'ready' | 'not_ready' | 'incompatible';
	detail: string;
	authentication: string;
	limitations: string[];
}

export interface CompanyLimitStatement {
	title: string;
	explanation: string;
}

export interface CompanyResource {
	id: string;
	label: string;
	kind: string;
	source: string;
	status: string;
	observed_at: string;
	detail?: string;
	metadata?: Record<string, unknown>;
	launch?: ArtifactLaunchDescriptor;
}

export type ArtifactLaunchShape = 'embedded_web' | 'native_client' | 'company_computer';

export interface ArtifactLaunchDescriptor {
	contract_version: 'artifact-launch.v1';
	shape: ArtifactLaunchShape;
	availability:
		| 'ready'
		| 'preparing'
		| 'mac_must_remain_awake'
		| 'requires_always_on_runner'
		| 'unavailable'
		| 'expired'
		| 'stopped';
	detail: string;
	open_endpoint: string;
	artifact_digest?: string;
	candidate_digest?: string;
	work_id?: string;
	attempt_id?: string;
	audience?: string;
	expires_at?: string;
	platform?: string;
	publication_id?: string;
	runtime_generation?: string;
}

export type ArtifactOpenOutcome =
	| { kind: 'embedded'; href: string; expires_at: string; reused: boolean }
	| { kind: 'native'; state: string; handle: string; expires_at: string; reused: boolean }
	| { kind: 'company_computer'; href: string }
	| { kind: 'external'; href: string; reason: string };

export interface CompanyExternalAction {
	id: string;
	title: string;
	effect_class: string;
	source: string;
	state: string;
	evidence:
		| 'provider_confirmed'
		| 'self_attested'
		| 'reconciled'
		| 'legacy_unverified'
		| 'unknown'
		| 'authority_recorded';
	actor?: string;
	party?: string;
	receipt_ref?: string;
	detail?: string;
	/** The Work this action served, when its record says so. */
	work_id?: string;
	observed_at: string;
}

export interface CompanyDoctor {
	status: 'healthy' | 'degraded' | 'unknown' | 'unavailable';
	observed_at: string;
	checks: Array<{
		id: string;
		label: string;
		source: string;
		status: 'healthy' | 'degraded' | 'unknown' | 'unavailable';
		summary: string;
		detail?: string;
	}>;
	actions: Array<{
		id: RecoveryAction;
		label: string;
		consequence: string;
		confirmation: string;
	}>;
}

export interface RuntimeDoctor {
	company: string;
	container: 'Running' | 'Stopped' | 'Absent';
	volume: string;
	volume_exists: boolean;
	volume_mounted: boolean;
	reconciliation: 'current' | 'required' | 'unknown';
	release?: {
		core_version: string;
		source_revision: string;
		api_contract_version: number;
		assertion_contract_version: number;
		schema_version: number;
		harnesses: Record<string, string>;
		harness_agents: Record<string, string>;
		harness_dependencies: Record<string, string>;
	};
	coordination?: { status: string; detail?: string };
	supervisor?: { status: string; services: Array<{ name: string; state: string }> };
	browser?: {
		status: string;
		desktop: string;
		chromium: string;
		automation: string;
		web_transport: string;
		controller: string;
	};
}

export interface BrowserStatus {
	generation: string | null;
	browser: RuntimeDoctor['browser'] | null;
	control: {
		controller?: string;
		client_id?: string;
		requester?: string;
		requesting_actor?: string;
		expires_at?: string;
	} | null;
	/** The agent driving the shared browser right now, if any. */
	agent?: { actor: string; display?: string | null; work_id?: string | null } | null;
}

export type RecoveryAction = 'start' | 'restart' | 'reconcile';

export interface CharterRevisionOutcome {
	company: CompanyView;
	message: string;
	runtime_projection: {
		status: 'updated' | 'deferred' | 'failed' | 'unchanged';
		detail?: string;
	};
	evidence_status: 'recorded' | 'incomplete' | 'unchanged';
}

const ownerResponse = ownerJson;

/** A page of the governance record: receipts, outbound email and owner setting changes (source
 * `owner_setting`), newest first, older than `before`. */
export async function getGovernanceActivity(
	company: string,
	before?: number
): Promise<{ items: CompanyExternalAction[]; next_before?: number }> {
	const query = before == null ? '' : `?before=${before}`;
	return ownerResponse(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/activity${query}`, {
			credentials: 'same-origin',
			cache: 'no-store'
		})
	);
}

export async function getCompany(company: string, probeCredentials = false): Promise<CompanyView> {
	const query = probeCredentials ? '?probe_credentials=true' : '';
	return ownerResponse<CompanyView>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company${query}`, {
			credentials: 'same-origin',
			cache: 'no-store'
		})
	);
}

export async function openCompanyResource(
	company: string,
	resource: CompanyResource
): Promise<ArtifactOpenOutcome> {
	if (!resource.launch || resource.launch.availability !== 'ready') {
		throw new Error(resource.launch?.detail ?? 'This resource cannot be opened.');
	}
	return ownerResponse<ArtifactOpenOutcome>(
		await fetch(resource.launch.open_endpoint, {
			method: 'POST',
			credentials: 'same-origin'
		})
	);
}

export async function recoverCompany(
	company: string,
	action: RecoveryAction
): Promise<{ action: RecoveryAction; message: string; doctor: RuntimeDoctor }> {
	return ownerResponse(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/recover`, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ action }),
			credentials: 'same-origin'
		})
	);
}

/** Put a quiet computer to sleep now; refused while work runs. */
export async function sleepCompany(company: string): Promise<{ message: string }> {
	return ownerResponse(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/sleep`, {
			method: 'POST',
			credentials: 'same-origin'
		})
	);
}

export async function reviseCompanyCharter(
	company: string,
	markdown: string,
	baseRevision: string,
	proposalId?: string,
	/** Where the words came from, for the charter's history: agreed at `setup` with Exec, or a
	 * `restore` of an earlier revision. Plain saves leave it out. */
	source?: 'setup' | 'restore'
): Promise<CharterRevisionOutcome> {
	return ownerResponse<CharterRevisionOutcome>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/charter`, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({
				markdown,
				base_revision: baseRevision,
				proposal_id: proposalId,
				source
			}),
			credentials: 'same-origin'
		})
	);
}

/** On Cloud the account owns a company's name, and the plane keeps the copy the cockpit shows: a
 * typed rename is made here before that copy is saved, a name copy after it is made. Locally the
 * plane's copy is the name. */
export async function renameInAccount(company: string, name: string): Promise<void> {
	if (!(await getApplianceStatus()).hosted) return;
	const response = await fetch('/account/company-name', {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		credentials: 'same-origin',
		body: JSON.stringify({ company, name })
	});
	if (response.ok) return;
	const body = await response.json().catch(() => null);
	throw new Error(body?.error ?? 'The name was not saved. Try again.');
}

export async function getBrowserStatus(company: string): Promise<BrowserStatus> {
	return ownerResponse<BrowserStatus>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/browser/status`, {
			credentials: 'same-origin',
			cache: 'no-store'
		})
	);
}

/** The company's setup as the name editor holds it: the name, plus the mission and model sent
 * back exactly as the server holds them, so a rename can never change either. */
export type CompanySetup = {
	display_name: string;
	mission: string;
	model: string;
	revision: string;
};

export async function readCompanySetup(company: string): Promise<CompanySetup> {
	return ownerResponse<CompanySetup>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/setup`, {
			cache: 'no-store',
			credentials: 'same-origin'
		})
	);
}

export async function saveCompanySetup(
	company: string,
	setup: CompanySetup
): Promise<{ revision: string }> {
	return ownerResponse<{ revision: string }>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/setup`, {
			method: 'PUT',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(setup),
			credentials: 'same-origin'
		})
	);
}

const companyApi = (company: string) => `/api/companies/${encodeURIComponent(company)}`;

async function postCompany<T>(company: string, path: string, body: unknown): Promise<T> {
	return ownerResponse<T>(
		await fetch(`${companyApi(company)}${path}`, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(body),
			credentials: 'same-origin'
		})
	);
}

/** Native harness usage limits per month; the answer is the company view. */
export function saveNativeLimit(
	company: string,
	body: {
		monthly_turn_limit: number | null;
		monthly_token_limit: number | null;
		expected_monthly_turn_limit: number | null;
		expected_monthly_token_limit: number | null;
	}
) {
	return postCompany<CompanyView>(company, '/company/native-limit', body);
}

/** When the company computer sleeps, and its monthly runtime cap; the answer is the company view. */
export function saveRuntimePolicy(
	company: string,
	body: {
		auto_sleep_after_minutes: number | null;
		expected_auto_sleep_after_minutes: number | null;
		monthly_runtime_cap_hours: number | null;
		expected_monthly_runtime_cap_hours: number | null;
	}
) {
	return postCompany<CompanyView>(company, '/company/runtime-policy', body);
}

/** The model spend limit, its period and its warning share; the answer is the company view. */
export function saveSpendLimit(
	company: string,
	body: {
		ceiling: string;
		expected_ceiling: unknown;
		period?: 'total' | 'monthly';
		warning_percent?: number;
	}
) {
	return postCompany<CompanyView>(company, '/company/spend-limit', body);
}

/** Resume exact spend accounting after unpriced model calls; the answer is the company view. */
export function resumeExactSpend(company: string) {
	return postCompany<CompanyView>(company, '/company/spend-exact', {});
}

/** Pause or resume every outside action; the answer is the company view. */
export function saveOutsideActions(company: string, paused: boolean) {
	return postCompany<CompanyView>(company, '/company/outside-actions', { paused });
}

/** Change a payment allowance's limits (minor units) or freeze it; the answer is the company view. */
export function savePaymentAllowance(
	company: string,
	body: {
		currency: string;
		per_payment_limit_minor: number;
		aggregate_limit_minor: number;
		frozen: boolean;
		/** Creating an allowance names the account it pays from. */
		source_account_ref?: string;
		/** The payees it may pay; absent keeps them. */
		beneficiary_refs?: string[];
	}
) {
	return postCompany<CompanyView>(company, '/company/payment-allowance', body);
}

export interface PaymentProvider {
	environment: 'sandbox' | 'live';
	api_version: string;
	client_id: string;
	account_ref: string;
	approval_url: string;
	read_scopes: string[];
	submit_scopes: string[];
	approval_workflow_observed: boolean;
	observed_at?: string | null;
	updated_at: string;
	keys_bound: boolean;
}

/** What connecting or checking the payment provider found: the probe only when it passed. */
export interface PaymentRailResult {
	company: CompanyView;
	probe?: { observed_at: string; account_ref: string; environment: string };
	probe_error?: string;
}

export function savePaymentRail(
	company: string,
	body: {
		environment: 'sandbox' | 'live';
		api_version: string;
		client_id: string;
		account_ref: string;
		approval_url: string;
		read_scopes: string[];
		submit_scopes: string[];
		approval_workflow_observed: boolean;
		read_key?: string;
		submit_key?: string;
	}
) {
	return postCompany<PaymentRailResult>(company, '/company/payment-rail', body);
}

export function probePaymentRail(company: string) {
	return postCompany<PaymentRailResult>(company, '/company/payment-rail/probe', {});
}

/** The owner's answer to a payee request from the Inbox. */
export function decidePayee(company: string, proposal: string, approve: boolean, note?: string) {
	return postCompany<{ decided: boolean }>(
		company,
		`/company/payees/${encodeURIComponent(proposal)}/decide`,
		{ approve, note }
	);
}

/** Preview or apply copying setup sections from another company. */
export function copyCompanySetup<T>(company: string, preview: boolean, body: unknown) {
	return postCompany<T>(company, `/copy-setup${preview ? '/preview' : ''}`, body);
}

/** The company's legal details as the owner asserts them: only what is safe on ordinary
 * invoices, contracts and provider forms. */
export type LegalProfileInput = {
	legal_name: string;
	trading_name?: string | null;
	entity_type: string;
	jurisdiction: string;
	registration_identifier: { kind: string; value: string };
	approved_business_address: string;
	invoice_email?: string | null;
};

export async function readLegalProfile(company: string): Promise<LegalProfileInput | null> {
	const body = await ownerResponse<{ profile: LegalProfileInput | null }>(
		await fetch(`${companyApi(company)}/company/legal`, {
			cache: 'no-store',
			credentials: 'same-origin'
		})
	);
	return body.profile;
}

/** Save the legal profile; the answer is the company view. */
export async function saveLegalProfile(
	company: string,
	profile: LegalProfileInput
): Promise<CompanyView> {
	return ownerResponse<CompanyView>(
		await fetch(`${companyApi(company)}/company/legal`, {
			method: 'PUT',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(profile),
			credentials: 'same-origin'
		})
	);
}
