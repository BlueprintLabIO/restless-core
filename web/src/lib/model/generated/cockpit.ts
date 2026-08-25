// GENERATED — do not edit.
//
// Source: crates/restlessd/src/owner.rs (the owner projection writer).
// Regenerate: RESTLESS_WRITE_COCKPIT_BINDINGS=1 cargo test -p restlessd cockpit_typescript_bindings_match
//
// This is the cockpit response contract, not a client-side view-model.

export type JsonValue = number | string | boolean | Array<JsonValue> | { [key in string]: JsonValue } | null;

export type CockpitCompany = { id: string, name: string, mission: string, model: string, };

export type CockpitModelCooldown = { model: string, kind: string, reason: string, retry_at: string, };

export type CockpitPerson = { actor_id: string, kind: string, role: string, display: string, model: string | null, team_id: string | null, spent_usd: number, session_running: boolean, session_observed_at: string | null, model_cooldown: CockpitModelCooldown | null, };

export type CockpitTeam = { id: string, name: string, brief: string, lead_actor_id: string, created_by: string, created_at: string, member_count: number, in_motion_count: number, blocked_count: number, };

export type CockpitGoal = { id: string, title: string, body: string, created_by: string, created_at: string, closed_at: string | null, };

export type CockpitSpend = { accounted_usd: number, ceiling_usd: number, remaining_usd: number | null, status: string, };

export type CockpitCredential = { binding: string, status: string, detail: string | null, };

export type CockpitRegistrationIdentifier = { kind: string, value: string, };

export type CockpitRegistryObservation = { source: string, status: string, observed_at: string, legal_name: string | null, entity_type: string | null, jurisdiction: string | null, registration_identifier: CockpitRegistrationIdentifier | null, detail: string | null, };

export type CockpitLegalProfile = { legal_name: string, trading_name: string | null, entity_type: string, jurisdiction: string, registration_identifier: CockpitRegistrationIdentifier, approved_business_address: string, invoice_email: string | null, owner_asserted_by: string, owner_asserted_at: string, registry_observation: CockpitRegistryObservation | null, };

export type CockpitLegal = { status: string, profile: CockpitLegalProfile | null, detail?: string, };

export type CockpitProviderConnection = { environment: string, account_ref: string, api_version: string, read_scopes: Array<string>, submit_scopes: Array<string>, approval_workflow_observed: boolean, observed_at: string | null, updated_at: string, };

export type CockpitProvider = { status: string, connection: CockpitProviderConnection | null, detail?: string, };

export type CockpitMoneyEnvelope = { source_account_ref: string, currency: string, beneficiary_refs: Array<string>, per_payment_limit_minor: number, aggregate_limit_minor: number, frozen: boolean, period_started_at: string, updated_by: string, updated_at: string, };

export type CockpitPaymentIntent = { work_id: string, owner_handoff_id: string, source_account_ref: string, provider_beneficiary_ref: string, amount_minor: number, currency: string, purpose: string, evidence_refs: Array<string>, idempotency_key: string, requesting_actor: string, state: string, provider: string, provider_transfer_id: string | null, raw_provider_status: string | null, provider_approval_url: string | null, settled_at: string | null, created_at: string, updated_at: string, };

export type CockpitBalanceObservation = { observed_at: string, body: JsonValue, };

export type CockpitFinance = { status: string, envelopes: Array<CockpitMoneyEnvelope>, payments: Array<CockpitPaymentIntent>, last_balance_observation: CockpitBalanceObservation | null, detail?: string, };

export type CockpitEvidenceQuality = "governed" | "legacy_unverified";

export type CockpitEffectReceipt = { id: number, effect_class: JsonValue | null, tool: JsonValue | null, success: JsonValue | null, party: JsonValue | null, actor: JsonValue | null, outcome: JsonValue | null, evidence_quality: CockpitEvidenceQuality, at: string, };

export type CockpitAuthority = { approved_parties: Array<string>, credentials: Array<CockpitCredential>, legal: CockpitLegal, provider: CockpitProvider, finance: CockpitFinance, };

export type CockpitView = { company: CockpitCompany, source_health: { [key in string]: string }, people: Array<CockpitPerson>, teams: Array<CockpitTeam>, goals: Array<CockpitGoal>, spend: CockpitSpend, authority: CockpitAuthority, receipts: Array<CockpitEffectReceipt>, refreshed_at: string, };
