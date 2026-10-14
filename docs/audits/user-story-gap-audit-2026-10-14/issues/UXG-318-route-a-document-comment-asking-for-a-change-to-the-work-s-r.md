# UXG-318 — Route a document comment asking for a change to the Work's revision

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | medium | spec-drift | US-196 |

## Problem

first-outcome.md asks the owner to comment on the payment terms and then send a revision. A document @mention to an agent is answered as a bounded question in that thread, with no Work id, so "please change this" in a comment produces a reply rather than a revision. The owner has to restate it through the rail's Request changes. The two review surfaces don't meet. Distinct from UXG-085 (the Discuss button's edit rights).

## Evidence

- `crates/restless-engine/src/mentions.rs:15-20` (Document mention work_id() returns None)
- `crates/restless-engine/src/mentions.rs:24-26` (prompt: "Answer the bounded question… do not post another reply")
- `docs/first-outcome.md:26-31` (comment, then separate revision request)

## Suggested direction

When a comment on a Work-bound document asks for a change, offer "Send as change request" on the thread, which feeds the existing request-changes path.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
