# Sprint 15 — Trusted runtime boundary

**Status:** Active — security and evidence repair following the 24 August 2026 codebase audit.

**Date:** 24 August 2026

**Depends on:** Sprint 14's preserved Rust consolidation work. Sprint 12's connected desktop/mobile
cockpit review remains a separate release gate and is not silently closed here.

**Spec refs:** `ARCHITECTURE.md` §2–§5, §9 and §16; authority-plane §2.5–§2.6, §4–§6,
§12 and §16; company-runtime §2, §4, §5 and §11; cross-layer-contract §3–§5, §8 and
§14; owner-cockpit §2 and §14; and evaluation-dogfood §2–§4.

---

## Observed friction

A read-only audit against the current `dev` worktree found six concrete failures or liabilities:

1. The company coordination listener binds `0.0.0.0:7791`, while a JSON request supplies its own
   `principal`. Any process that reaches the listener can claim `owner`.
2. Every runtime receives the same OMP auth-gateway bearer. It is not bound to a company, actor,
   session, model policy, or spend envelope, and direct gateway traffic is not canonical spend evidence.
3. A hard model ceiling is a raw `f64`: `inf` is accepted and becomes `u64::MAX` when enforced.
4. The Sprint 14 wire move retained a flat, all-domain request and a nearly 2,000-line dispatcher.
5. The owner BFF builds dynamic JSON separately from the Svelte read models, with no checked response
   contract at the high-value cockpit boundary.
6. Ordinary workspace tests intentionally skip live Postgres scenarios when their URL is absent; the
   explicit verifier exists but is not a required release/checkpoint gate.

These are observed defects, not a pretext for a new control-plane framework.

## Founders' decision

The chosen canon is a small signed capability boundary, not a general identity service or a return to
a universal command protocol:

```text
owner on local Unix socket
  → local-owner identity derived from the listener

Company Runtime / supervised actor
  → short-lived signed bridge or session capability
  → daemon derives company, principal and actor
  → bounded coordination/model operation

scoped model request
  → host-side admission and metering
  → host-only OMP provider gateway
```

We considered three routes before choosing this canon:

| Candidate | Result |
| --- | --- |
| Private bind or a stronger `OWNER_ONLY` list | Rejected: it leaves caller-supplied identity and a reusable model bearer intact. |
| Signed runtime/session capabilities plus a narrow host relay | **Chosen:** closes the observed impersonation and cross-company scope gap without adding a service or policy language. |
| A new mTLS identity service, per-worker containers and a generic policy engine | Deferred: V0 has no evidence that this added machinery is needed once the current capability boundary is real. |

## Outcome

An owner can run a company through the local appliance while the Runtime can perform its bounded work,
but neither a reachable socket client nor one company session can impersonate the owner, cross a
company/session boundary, or silently turn an invalid ceiling into uncapped spend. Owner projections
and release evidence have explicit, checkable contracts.

## Success contract

1. **Listener-derived owner identity.** A Unix-socket request is local-owner access; a TCP request
   needs a signed, unexpired runtime/session capability. Its company and principal are derived by the
   daemon, never accepted from `principal` JSON.
2. **Narrow Runtime capability.** A runtime capability cannot perform an owner-only command, cannot
   name another company, and an actor session cannot claim a different actor. Expiry and tampering are
   refused before dispatch.
3. **Scoped model access.** OMP's root gateway bearer stays loopback-only. A Runtime receives only a
   short-lived model capability bound to one company, actor, session and configured provider. The host
   rejects a mismatched model, expired/tampered grant, and a company already at its model ceiling.
4. **Canonical charged usage.** Metered model responses are attributed at the host gateway by company,
   actor and session. Missing terminal charged usage fails closed rather than creating invisible spend.
   Subscription traffic remains zero charged dollars but remains attributable.
5. **Finite exact ceiling.** Company config and CLI accept only a non-negative, finite ceiling and the
   enforcement path uses exact micro-USD rather than float-to-integer saturation.
6. **Narrower transport.** Command decoding selects a domain-specific input view and rejects fields
   foreign to that command. Domain handlers own lifecycle/authority, OrgIntel and owner dispatch; no
   universal command algebra or second writer is introduced.
7. **Checked cockpit projection.** The high-value cockpit response has a serializable Rust DTO and a
   checked client contract; a router-level test proves the response shape instead of merely compiling
   two unrelated models.
8. **Honest verification.** The live-Postgres verifier is required by the documented checkpoint/release
   command and fails loudly when no scratch URL is supplied. Fast no-DB tests remain fast and are not
   called live-DB evidence.
9. **Checkpoint discipline.** Every completed slice is committed after its named verification. Pushes
   occur only under the owner's explicit authorisation and never include unrelated dirty work.

## Layer slices

| Concern | Authoritative layer | Sprint 15 change |
| --- | --- | --- |
| Principal, session capability, ceilings and model admission | Authority | Signed capability verification, exact budget value and host-side charged-use admission |
| Actor attribution and coordination operations | OrgIntel boundary | Derive actor/company from the session capability; preserve existing OrgIntel writers |
| Container and ACP launch | Runtime / Runtime Bridge | Materialise only bounded bridge/session/model grants, never a root bearer |
| Owner projection | Owner cockpit | Typed projection contract and endpoint evidence; no visual redesign |
| Verification | Evaluation | Required live-Postgres preflight in the release/checkpoint path |

## Risks and dispositions

| Risk | Disposition | Why |
| --- | --- | --- |
| A bearer copied by a same-runtime process is used during its own live session | **Accepted for V0** | The V0 runtime is not hostile per-worker isolation; the grant is bounded to company, actor, provider and expiry. A demonstrated hostile-process workload is the trigger for stronger isolation. |
| An unpriced streamed request has consumed provider money | **Invariant** | Poison/block the company before another charged request; invisible spend is worse than temporary interruption. |
| HMAC capability code grows into a general identity/permissions product | **Invariant** | Exactly two callers and fixed claims; no account table, role registry, policy DSL or token catalogue. |
| A new proxy duplicates OMP | **Guarded** | It validates scope, enforces admission and meters the pi-native stream; OMP remains the provider-compatible gateway. |
| Hard ceiling still has one in-flight response overshoot | **Accepted** | Provider usage arrives after inference. The gateway blocks subsequent requests and reports the bound honestly; no speculative reservation engine. |
| Transport movement changes organisation semantics | **Guarded** | Move handlers behind existing commands and replay the direct-message, Work, effect and recovery scenarios. |
| Owner surface drifts visually | **Accepted** | This sprint changes contracts, not appearance; Sprint 12's connected visual review remains separately open. |

## Non-goals

- hosted accounts, multi-user identity, mTLS fleet infrastructure or a generic authorization service;
- per-worker hostile-process isolation, per-request approval, a policy DSL, or a model scheduler;
- a custom provider router, model catalogue, durable reservation table or workflow engine;
- a second owner UI, React runtime or visual identity change;
- turning every local test into a Postgres test; and
- rewriting historical coordination or spend records.

## Tickets

Ticket status lives only in this checklist.

| Status | Ticket | Slice | Observed friction served | Prior machinery made deletable |
| --- | --- | --- | --- | --- |
| [ ] | [**S15-T0 · Freeze trust-boundary contract and checkpoint baseline**](sprint-15/t00-contract-and-checkpoints.md) | Cross-layer + evaluation | An accepted identity risk expired before real effects existed | Caller-claim trust comments and ambiguous checkpoint practice |
| [ ] | [**S15-T1 · Authenticate the Runtime coordination channel**](sprint-15/t01-runtime-coordination-capability.md) | Authority + Runtime Bridge | TCP caller can claim `owner` | TCP trust-by-JSON-principal |
| [ ] | [**S15-T2 · Scope and meter host model access**](sprint-15/t02-scoped-model-gateway.md) | Authority + Runtime | One gateway bearer and invisible direct spend | Runtime exposure of the OMP root bearer and ACP-only charged accounting |
| [ ] | [**S15-T3 · Make model ceilings exact and finite**](sprint-15/t03-exact-model-ceiling.md) | Authority | `inf` disables the hard ceiling | Float saturation in the decision path |
| [ ] | [**S15-T4 · Finish command-domain transport decomposition**](sprint-15/t04-domain-transport.md) | Daemon transport | Flat all-domain payload and god dispatcher remain | Cross-domain optional-field bag and duplicate command construction |
| [ ] | [**S15-T5 · Check the owner cockpit projection contract**](sprint-15/t05-owner-projection-contract.md) | Owner cockpit | Dynamic Rust JSON and handwritten Svelte model can drift | Unchecked high-value BFF shape |
| [ ] | [**S15-T6 · Require live-Postgres evidence at checkpoint exit**](sprint-15/t06-live-db-gate.md) | Evaluation + OrgIntel | Green workspace test can mean skipped DB scenarios | Implicit use of `cargo test` as live-DB proof |
| [ ] | [**S15-T7 · Run the boundary scenario, purge and report**](sprint-15/t07-boundary-run-and-purge.md) | All | Security code can compile without proving the company boundary | Superseded comments, duplicate meters and stale docs |

## Exit evidence

Sprint 15 exits only with recorded command/output for:

1. a raw TCP request that claims `owner` and is refused before dispatch;
2. a valid Runtime/actor capability that permits its normal coordination action and rejects another
   company, another actor, expiry and signature tampering;
3. a scoped model request accepted for the configured provider and refused for a mismatched provider,
   expired grant and exhausted company ceiling;
4. a metered stream whose terminal usage creates one attributed host-side record, plus a missing
   terminal-usage scenario that blocks the company;
5. finite ceiling parsing including rejected `NaN`, `inf` and negative values;
6. focused command-domain, owner projection and live-DB preflight checks;
7. formatting, strict Clippy, workspace tests, web checks and a real `restless doctor -c <test company>`
   probe; and
8. narrow checkpoint commits pushed to `dev` under this sprint's explicit owner authorisation.

No test suite or sprint prose alone substitutes for these observed outcomes.

## Checkpoint command

Run `RESTLESS_TEST_DATABASE_URL=postgresql:///restless scripts/verify-sprint-checkpoint` against a
local scratch database when preparing a Sprint 15 checkpoint or release candidate. It first invokes
the guarded OrgIntel live-Postgres verifier, then runs Rust formatting, strict Clippy, the workspace
tests, and Svelte checks. A missing or non-local scratch URL fails before any Rust compiler command.

Ordinary `cargo test` remains the fast iteration loop. It is not evidence that the Postgres-backed
OrgIntel scenarios ran; record the checkpoint command's actual output before making that claim.
