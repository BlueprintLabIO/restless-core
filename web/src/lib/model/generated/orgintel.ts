// GENERATED — do not edit.
//
// Source: crates/restless-orgintel/src/lib.rs (the single writer).
// Regenerate: RESTLESS_WRITE_BINDINGS=1 cargo test -p restless-orgintel
//
// These are OrgIntel rows as they cross the wire, not the owner-surface
// view model. `$lib/model/view.ts` stays hand-written: it is a contract
// in its own right (what the surfaces need), and these are its inputs.

export type JsonValue = number | string | boolean | Array<JsonValue> | { [key in string]: JsonValue } | null;

export type CommitmentState = "proposed" | "active" | "blocked" | "completed" | "abandoned";

export type ActorRow = { id: string, 
/**
 * The actor's durable role — `copywriter`, `critic`, `exec`, `owner`.
 * S04-T5 stopped flattening every worker to the literal `"staff"`, which
 * is why AC5 can ask for rows whose kind is not `"staff"`.
 */
kind: string, display: string, 
/**
 * NULL means inherited or not applicable, never "unknown".
 */
model: string | null, created_at: string, };

export type GoalRow = { id: string, title: string, body: string, created_by: string, created_at: string, closed_at: string | null, };

export type CommitmentRow = { id: string, goal_id: string | null, owner_id: string, title: string, body: string, state: CommitmentState, resolution: string, created_at: string, updated_at: string, };

export type MessageRow = { id: number, from_actor: string, to_actor: string | null, body: string, created_at: string, read_at: string | null, };

export type EventRow = { id: number, kind: string, actor_id: string | null, body: JsonValue, created_at: string, };

