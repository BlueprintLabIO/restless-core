import type { MoneyEnvelope } from './cockpit';
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

export interface McpReadReceipt {
	call_id: string;
	phase: 'started' | 'terminal';
	actor: string;
	work_id: string;
	attempt_id: string;
	connection_name: string;
	tool_name: string;
	tool_contract_digest: string;
	policy_revision: string;
	request_digest: string;
	result_digest: string | null;
	subject: { kind?: string; site?: string; repository?: string } | null;
	status: string;
	provider_status: string | null;
	observed_at: string;
	wall_ms: number | null;
	error_class: string | null;
}

export interface McpRepinOutcome {
	name: string;
	assigned_actor: string;
	work_id: string;
	tool_contract_digest: string;
	policy_revision: string;
	last_observed_at: string;
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

export async function disableCompanyMcp(company: string, name: string): Promise<void> {
	await ownerResponse<unknown>(
		await fetch(
			`/api/companies/${encodeURIComponent(company)}/company/mcp/${encodeURIComponent(name)}/disable`,
			{ method: 'POST', credentials: 'same-origin' }
		)
	);
}

export async function getCompanyMcpReceipts(
	company: string,
	name: string
): Promise<McpReadReceipt[]> {
	return ownerResponse<McpReadReceipt[]>(
		await fetch(
			`/api/companies/${encodeURIComponent(company)}/company/mcp/${encodeURIComponent(name)}/receipts`,
			{ credentials: 'same-origin', cache: 'no-store' }
		)
	);
}

export async function repinCompanyMcp(
	company: string,
	name: string,
	workId: string
): Promise<McpRepinOutcome> {
	return ownerResponse<McpRepinOutcome>(
		await fetch(
			`/api/companies/${encodeURIComponent(company)}/company/mcp/${encodeURIComponent(name)}/repin`,
			{
				method: 'POST',
				credentials: 'same-origin',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ work_id: workId })
			}
		)
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

export async function reviseCompanyCharter(
	company: string,
	markdown: string,
	baseRevision: string
): Promise<CharterRevisionOutcome> {
	return ownerResponse<CharterRevisionOutcome>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/charter`, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ markdown, base_revision: baseRevision }),
			credentials: 'same-origin'
		})
	);
}

export async function setCompanyHarnesses(
	company: string,
	coordinationHarness: AgentHarness,
	workerHarness: AgentHarness
): Promise<CompanyView> {
	return ownerResponse<CompanyView>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/harnesses`, {
			method: 'POST',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({
				coordination_harness: coordinationHarness,
				worker_harness: workerHarness
			}),
			credentials: 'same-origin'
		})
	);
}

export async function getBrowserStatus(company: string): Promise<BrowserStatus> {
	return ownerResponse<BrowserStatus>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/browser/status`, {
			credentials: 'same-origin',
			cache: 'no-store'
		})
	);
}
