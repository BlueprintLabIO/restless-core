// GENERATED — do not edit.
//
// Source: crates/restless-engine/src/direction.rs (the direction contract).
// Regenerate: RESTLESS_WRITE_DIRECTION_CONTRACT=1 cargo test -p restless-engine direction_contract

export type DirectionClass = "content" | "work" | "staffing" | "schedule";

export type DirectionPath = "direct" | "act_then_tell" | "ask_first";

export type DirectionPerson = { actor_id: string, name: string, };

export type DirectionTeam = { id: string, name: string, };

export type DirectionTarget = { "kind": "document", id: string, title: string, } | { "kind": "sheet", id: string, title: string, } | { "kind": "charter" } | { "kind": "team_brief", team: DirectionTeam, } | { "kind": "work", id: string, title: string, } | { "kind": "schedule", id: string, title: string, } | { "kind": "agent", actor_id: string, name: string, };

export type UndoHandle = { "kind": "content", before: unknown, } | { "kind": "work", work_id: string, before: unknown, } | { "kind": "staffing", actor_id: string, before: unknown, } | { "kind": "schedule", schedule_id: string, before: unknown, };

export type UndoState = { "state": "available", until: string, } | { "state": "undone", by: DirectionPerson, at: string, } | { "state": "changed_since" } | { "state": "expired" };

export type DirectionApplied = { id: string, requester: DirectionPerson, 
/**
 * The message the direction came from, when there was one.
 */
source_message: string | null, class: DirectionClass, team: DirectionTeam | null, 
/**
 * One line, in the requester's words: "Website copy speaks in first person as the business".
 */
summary: string, target: DirectionTarget, 
/**
 * Whoever owns the direction: the team's person, or the owner.
 */
routed_to: DirectionPerson, applied_at: string, 
/**
 * The grant that let it apply without asking, when one did.
 */
grant_id: string | null, undo: UndoState, };

export type RequestState = "open" | "approved" | "declined" | "done" | "failed";

export type DirectionDecisionKind = "approve" | "decline";

export type DirectionDecision = { by: DirectionPerson, decision: DirectionDecisionKind, 
/**
 * A decline's optional one-line reason, passed back to the requester.
 */
reason: string | null, decided_at: string, 
/**
 * The standing grant "Approve, and always allow" recorded.
 */
grant_id: string | null, 
/**
 * The person "Make … owner of this team" appointed.
 */
team_owner: DirectionPerson | null, };

export type Grantee = { "kind": "person" } & DirectionPerson | { "kind": "members" };

export type GrantScope = { "kind": "company" } | { "kind": "team" } & DirectionTeam;

export type GrantOffer = { 
/**
 * The button's exact words.
 */
label: string, grantee: Grantee, class: DirectionClass, scope: GrantScope, };

export type RequestOffers = { 
/**
 * "Always allow Bianca to change content for Site Renovation Studio": present only for a
 * grantable class, and only to whoever may grant it.
 */
always_allow: GrantOffer | null, 
/**
 * "Make Bianca owner of this team": present only when the team has no person.
 */
make_team_owner: DirectionPerson | null, };

export type DirectionRequest = { id: string, requester: DirectionPerson, routed_to: DirectionPerson, 
/**
 * None when the change is in no class: always Ask first, never grantable.
 */
class: DirectionClass | null, team: DirectionTeam | null, 
/**
 * One line: "Make customer-facing copy first person".
 */
summary: string, 
/**
 * The exact change Exec will make on approval.
 */
change: string, reason: string, source_message: string | null, filed_at: string, state: RequestState, decision: DirectionDecision | null, 
/**
 * Why carrying it out failed, in a sentence; only when `state` is `failed`.
 */
failure: string | null, 
/**
 * What the change did once carried out.
 */
applied: DirectionTarget | null, offers: RequestOffers, };

export type DirectionGrant = { id: string, grantee: Grantee, class: DirectionClass, scope: GrantScope, granted_by: DirectionPerson, granted_at: string, 
/**
 * The request whose "Approve, and always allow" made it.
 */
source_request: string | null, 
/**
 * The row's words: "Bianca can change content for Site Renovation Studio".
 */
label: string, };

export type DirectionDigest = { 
/**
 * The company-local day, `YYYY-MM-DD`.
 */
day: string, 
/**
 * "Exec applied 4 changes for others · 2 requests waiting"; empty on a day with nothing.
 */
summary: string, applied: Array<DirectionApplied>, 
/**
 * Open requests, oldest first: those older than a day lead.
 */
requests_waiting: Array<DirectionRequest>, };

export type DirectionRequestPage = { items: Array<DirectionRequest>, };

export type DirectionAppliedPage = { items: Array<DirectionApplied>, };

export type DirectionGrantList = { items: Array<DirectionGrant>, };

export type DirectionDecisionInput = { decision: DirectionDecisionKind, reason?: string, 
/**
 * Approve, and record the request's `offers.always_allow` grant.
 */
always_allow?: boolean, 
/**
 * Approve, and appoint the request's `offers.make_team_owner` person to the team.
 */
make_team_owner?: boolean, };

export type DirectionUndoResult = DirectionApplied;

export const DIRECTION_REQUEST_ATTENTION_KIND = "direction_request";
export const DIRECTION_APPLIED_FOR_YOU_KIND = "applied";
export const DIRECTION_DAY_KEY_PREFIX = "direction-day:";
