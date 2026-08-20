import type { MoneyEnvelope } from './cockpit';

export type CompanySourceStatus = 'available' | 'unavailable' | 'stale' | 'absent';

export interface CompanySourceObservation {
	status: CompanySourceStatus;
	observed_at: string;
	detail?: string;
}

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
			poisoned: boolean;
		};
		money_envelopes: MoneyEnvelope[];
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
}

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

async function ownerResponse<T>(response: Response): Promise<T> {
	if (!response.ok) {
		let message = `${response.status} ${response.statusText}`;
		try {
			const body = await response.json();
			message = body.message ?? message;
		} catch {
			// Preserve the transport status when an intermediary returns non-JSON.
		}
		throw Object.assign(new Error(message), { status: response.status });
	}
	return response.json() as Promise<T>;
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

export async function getBrowserStatus(company: string): Promise<BrowserStatus> {
	return ownerResponse<BrowserStatus>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/browser/status`, {
			credentials: 'same-origin',
			cache: 'no-store'
		})
	);
}
