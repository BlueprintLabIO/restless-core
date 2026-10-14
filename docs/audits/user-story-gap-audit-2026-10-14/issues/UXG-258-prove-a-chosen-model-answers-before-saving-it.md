# UXG-258 — Prove a chosen model answers before saving it

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Company computer & intelligence | medium | spec-drift | US-150 |

## Problem

Saving an agent's model checks only that the model ID is non-blank and that the connection's credential is present. A mistyped custom model ID, a model the key cannot reach or a model missing a required modality is accepted with "Intelligence saved. Applies to the next session", and the failure surfaces later as a stalled agent. CLAUDE.md requires admitting the exact model with one real request. Distinct from UXG-103, which asks for confirmation and timing copy.

## Evidence

- `crates/restless-owner/src/owner.rs:11485-11490` (model ID shape only)
- `crates/restless-owner/src/owner.rs:11502-11504` (credential presence probe)
- `crates/restless-owner/src/owner.rs:11529-11533`
- `web/src/lib/components/AgentIntelligence.svelte:145-163` (save then success notice)

## Suggested direction

Run one small admission request for the exact connection and model on save, and show its result before committing.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
