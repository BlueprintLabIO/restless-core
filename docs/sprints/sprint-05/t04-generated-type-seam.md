# S05-T4 · The OrgIntel → owner-surface type seam, generated

**Layer:** OrgIntel, at its boundary with the owner surface. No new owned concept.
**Serves:** `cross-layer-contract` §3.1 — one concept, one authoritative writer. The Rust row types
are that writer; the TypeScript stops being a second one.
**Depends on:** nothing. Landed ahead of S05-T1/T2 because both add owner-surface reads over these
same rows.
**Makes deletable:** nothing yet. It removes a *class of future defect*, not existing machinery —
see "What this does not claim" below.

**Status: landed.** This ticket documents work already implemented and verified, not work proposed.

---

## The friction

`web/` is TypeScript, OrgIntel is Rust, and the read model crosses that boundary. Today each side is
hand-written. The failure is silent and one-directional: a field is renamed in Rust, the surface keeps
rendering the old name, and nothing fails anywhere — no compiler, no test, no runtime error. The owner
reads a blank cell and has no way to know it is a bug rather than an empty value.

`owner-cockpit` §2.7 is titled *evidence before self-report*, and this repo has paid for its violation
twice. A surface that silently renders a stale field is the same defect wearing different clothes: the
owner cannot tell absence from breakage.

## What was found first, and why it bounds the scope

The investigation corrected the premise the ticket started from. **The seam is not two sides diverging.
It is two sides not connected.**

- `restlessd` has **no HTTP API**. It speaks a JSON line protocol over a unix socket, and `Response`
  carries `data: serde_json::Value` — deliberately untyped (`main.rs:179`).
- `web/` renders entirely from `$lib/fixtures/cosmon.ts`. Its only real `fetch` is a leftover
  `/api/auth/sign-out` in `AppShell.svelte:158`, inherited from the source system.
- `view.ts`'s `DeskView` has **no Rust counterpart to generate from**. It is a hand-written 559-line
  contract, and `view.ts:1-10` says so deliberately: *"the shape the UI needs is a contract in its own
  right, not a projection of whatever the store happens to hold."*

So `DeskView` codegen is not available and should not be attempted. It becomes possible when the
OrgIntel read API exists — and per `view.ts`'s own header, the right move even then is to generate its
*inputs*, not to replace it.

What is generatable today is the layer underneath: the row types that already cross the wire and that
S05-T1's projection and S05-T2's reads will both assemble from.

## Scope

1. **`ts_rs::TS` derived on the OrgIntel read model** — `ActorRow`, `GoalRow`, `WorkRow`,
   `WorkEdgeRow`, `WorkAttemptRow`, artifact/gate/handoff rows, `MessageRow`, `EventRow`, and their enums.
2. **Generated output committed** at `web/src/lib/model/generated/orgintel.ts`, with a
   `web/.prettierignore` so the generator owns its formatting and `npm run lint` stays honest.
3. **A drift guard that runs by default** — `crates/restless-orgintel/tests/bindings.rs` renders the
   bindings and compares against the committed file. `cargo test` fails on mismatch;
   `RESTLESS_WRITE_BINDINGS=1 cargo test -p restless-orgintel` regenerates.

**Not in scope:** `DeskView` (unavailable — see above); the daemon's `ErrorBody`/`Response` envelope
(the CLI consumes it, not the web; it becomes part of this seam when S03-T8's wire contract lands);
generating anything the SPA does not yet read.

## Two decisions worth recording

**A test, not a build script.** A build script would rewrite the checked-in file during an ordinary
`cargo build`, which *hides* drift instead of reporting it — the working copy silently self-heals and
the disagreement never surfaces. This repo has no CI, so `cargo test` is the only place a guard
actually runs, and it must fail rather than fix.

**`i64` → `number`, overriding ts-rs's default `bigint`.** These rows arrive through `JSON.parse`,
which cannot produce a bigint — serde writes a bare JSON number and the browser reads one back.
`bigint` would typecheck against a value that never exists at runtime. Bigserial ids stay exact well
past any company's message count.

## Acceptance — observed, not asserted

1. **The guard catches drift.** `resolution` was renamed to `resolution_text` in Rust with the
   TypeScript left untouched. `cargo test -p restless-orgintel` failed, naming the line:

   ```
   line 28
     committed: ... status: WorkStatus, resolution: string, ...
     generated: ... status: WorkStatus, resolution_text: string, ...
   ```

   Reverted, and the test passes. *A check that happens to pass is not evidence* — this one was
   observed failing for the right reason before it was believed.
2. **The generated TypeScript is valid.** `npm run check` — svelte-check, 336 files, **0 errors,
   0 warnings**.
3. **Nothing else broke.** `cargo test --workspace` — 53 tests, all passing.
4. **Lint is unaffected.** The generated file passes `prettier --check` directly. The 24 pre-existing
   warnings are `.svelte` route files in the lifted SPA plus gitignored `.svelte-kit` build artifacts;
   no `.svelte` file was touched.

## What this does not claim

This removes a class of defect that cannot occur yet, because the SPA is not wired. That is the point
of landing it now rather than later: S05-T1 and S05-T2 both add owner-surface reads over these rows,
and the guard is cheapest to install before there are two consumers to reconcile, not after.

It is not a slice under §16.2 — it produces no artifact, decision, or external outcome, and should not
be counted as one. It is infrastructure under an outcome ticket, and the sprint's success contract is
unchanged by it.

---
Sprint spec: [`../sprint-05.md`](../sprint-05.md)
