# Sprint 27 — Network owner entry and pinnable releases

**Status:** Active — T1–T3 closed against a live plane; T4 half-landed (identity yes, images no); T5 open

**Date:** 30 August 2026

**Target:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §7.4 ·
[`docs/CELL_ARCHITECTURE.md`](../CELL_ARCHITECTURE.md) §2, §5 ·
[ADR 0007](../adr/0007-network-owner-entry-by-verified-assertion.md) ·
[Core/Cloud boundary](../specs/restless-cloud.md)

**Depends on:** S25-T8 (plane restart does not stop a cell). Network entry to a plane whose restart
kills companies is a worse product, not a bigger one.

**Independent of Sprint 26.** Sprint 26 makes unattended company work exact and gates EXP-16; this
sprint makes the plane reachable and gates every Cloud sprint. They touch different code and can run
in either order or concurrently. Founders sequence them by which gate matters sooner.

---

## Why this sprint exists

Sprint 25 split the tiers. This sprint makes the account plane reachable by someone who is not sitting
at the machine — the last thing standing between Restless and a hosted product.

Restless Cloud has specified an entire hosted product against a Core that no network client can reach.
Core's owner gateway refuses a non-loopback bind (`crates/restlessd/src/owner.rs:717`) and refuses any
request carrying a forwarding header (`owner.rs:850`). Both are deliberate — ADR 0001 chose them and
named the condition that would end them. Neither has ended.

The consequence is concrete and blocking. Every Cloud sprint from its first hosted cell onward is
gated on this, and until it exists the only ways to reach a hosted Restless are a login page that
gates nothing or a tunnel in front of a gateway that trusts network position. Cloud's roadmap
prohibits the second and would be embarrassed by the first.

Reviewing the Cloud specification set to write this sprint surfaced a second defect worth recording,
because it is the same class of error Sprint 25 fixed in the code. Cloud described **two** tiers —
Fleet and cells — with no account plane; the phrase appeared zero times in that repository. The
cockpit had been attributed to the cell, and credential custody had nowhere to live but a shared
multi-owner service, contradicting the structural property `CELL_ARCHITECTURE.md` §3 claims. The Cloud
specs, its ADR 0001 and its roadmap are now corrected. Core's tier model was right and is unchanged;
what was missing was that Cloud had not been told.

## Outcome

A real human reaches a Restless owner cockpit over a network, authenticated by a verified assertion
rather than by network position — and Cloud can pin the exact build that did it.

Two halves, both required: the plane verifies, and the artifact is identifiable. Either alone is
unusable — verification nobody can pin cannot be deployed, and a pinned artifact that trusts its
network is not deployable.

## Acceptance criteria

Each is headless-verifiable with stated inputs and observed output.

1. **Network mode admits a valid assertion.** Start a plane configured with an issuer, audience and
   verification key. Present a well-formed assertion; observe the cockpit served and the resolved
   principal recorded in the audit attribution as the same stable owner principal local mode produces.
2. **Every adversarial case is refused with its own reason.** Expired, not-yet-valid, wrong audience,
   unknown issuer, unknown key version, unsupported contract version, wrong plane route, tampered
   signature and replayed single-use identity each yield a distinct refusal. Ten inputs, ten recorded
   refusals — not one catch-all.
3. **Scope comes from the assertion, not the route.** An assertion scoped to company A, replayed
   against company B's URL on the same plane, is refused. Verified by request, not only at entry.
4. **Local mode is unchanged.** With no network configuration, the plane binds loopback, refuses
   forwarding headers exactly as before, and requires no token.
5. **A plane refuses to start in network mode without complete verification configuration.** Missing
   issuer, audience or key is a startup failure naming the missing field, not a silent downgrade to
   trusting the network.
6. **The release is pinnable and self-identifying.** One published manifest names the account-plane
   and Runtime image digests, schema versions, API version and assertion contract version. The
   `/health` probe on a running plane and cell reports the same release identity as the manifest that
   deployed it.
7. **End to end.** From a clean checkout, one command builds the release, boots a plane in network
   mode and a cell, mints a test assertion, and a browser reaches the cockpit over a non-loopback
   address.

## Slice per layer

- **Authority Plane / account plane** — entry modes, assertion verification, single-use consumption,
  session establishment, scope derivation per request.
- **OrgIntel / cell** — unchanged. The cell gains no network entry and no credential; it is reached
  through its plane exactly as today.
- **Runtime / fleet** — release manifest, image digests and the health/version probe on both tiers.
- **Out of scope** — the Fleet lifecycle operation contract (provision/start/stop/snapshot/restore as
  a published API). Cloud 02 is the first thing that needs it; model it then, against a real caller,
  per §16.1. Also out of scope: identity vendor selection, multi-human roles beyond the three Cloud
  membership roles, and splitting the binary into three crates.

## Ticket decomposition

Status lives only in this checklist; ticket files record scope and closure evidence, not a second
status system.

Status symbols: `[x]` closed on its stated evidence · `[~]` implementation landed, some stated
evidence not yet observed · `[ ]` not started.

| Status | Ticket | Slice | Outcome or friction served | Prior machinery made deletable |
| --- | --- | --- | --- | --- |
| [x] | [S27-T1 · Verify an assertion at the plane edge](sprint-27/t1-assertion-verification.md) | Authority | No supported way to reach a plane that is not the machine you are sitting at | The unconditional loopback bail as the only network posture |
| [x] | [S27-T2 · Derive company scope from the assertion](sprint-27/t2-scope-derivation.md) | Authority | Scope taken from a URL is scope a client chooses | Route- and host-derived company scope |
| [x] | [S27-T3 · Refuse the adversarial cases, provably](sprint-27/t3-adversarial-refusal.md) | Authority + Evaluation | A check that happens to pass is not evidence | Trust that verification works because it compiles |
| [~] | [S27-T4 · Publish a pinnable, self-identifying release](sprint-27/t4-pinnable-release.md) | Runtime + Authority | Cloud cannot deploy what it cannot pin, and cannot debug what will not name itself | Mutable tags; "whatever Core was on that day" |
| [ ] | [S27-T5 · Reach the cockpit over a network, end to end](sprint-27/t5-network-cockpit-run.md) | Full slice + Evaluation | Parts that each pass are not a product that works | Component-level confidence as a substitute for a run |

## Evidence

`crates/restlessd/src/entry.rs` is the new entry module; `owner.rs` dispatches on the mode.

**Observed, with the command and its output:**

- **Acceptance 5 — a plane refuses to start in network mode without complete verification
  configuration.** Run against the built binary with progressively fuller environments:

  ```text
  RESTLESS_ENTRY_MODE=network
    → Error: network entry mode requires RESTLESS_ENTRY_ISSUER; refusing to start rather than
      accepting requests on network position alone
  … + RESTLESS_ENTRY_ISSUER
    → Error: network entry mode requires RESTLESS_ENTRY_AUDIENCE; …
  … + AUDIENCE, PLANE, HOST
    → Error: network entry mode requires RESTLESS_ENTRY_KEYS; …
  RESTLESS_ENTRY_MODE=whatever
    → Error: RESTLESS_ENTRY_MODE must be `local` or `network`, not `whatever`
  ```

- **Acceptance 4 — local mode is unchanged, and the loopback bail is conditional rather than
  deleted.** `RESTLESS_OWNER_ADDR=0.0.0.0:7788` in local mode still fails with
  `RESTLESS_OWNER_ADDR must remain loopback-only until network authentication exists`. The same
  address in network mode passes that check and proceeds (it then fails on an unrelated missing model
  provider in an empty home, which is what a temp `RESTLESS_HOME` is expected to do).

- **The test issuer mints against the same wire format the verifier reads.**
  `restlessd mint-entry-assertion` emitted a token whose header is
  `{"alg":"HS256","typ":"restless-entry","kid":"v1"}` and whose claims carry `ver`, `iss`, `aud`,
  `plane`, `owner`, `sub`, `scope`, `role`, `actor`, `iat`, `nbf`, `exp`, `jti`.

- **Acceptance 2 — ten distinct refusals**, plus the inverse check S27-T3 requires: with signature
  verification skipped, a forged assertion passes every other gate and verifies, proving the signature
  check is what rejects it rather than some incidental later check.

- **Acceptance 3 — scope from the assertion, not the route.** A company-scoped session reaches its own
  company and is refused `company_out_of_scope` for another company on the same plane, on both the API
  and desktop paths, and is not widened by an `X-Forwarded-Host` naming the other company.

- `cargo test -p restlessd --bin restlessd` — **219 passed, 0 failed, 5 ignored.**

**Observed against a live plane** booted in network mode on a temp home (torn down in the same
turn), reached over HTTP:

```text
GET  /health          → {"status":"ok","release":{"core_version":"0.0.0",
                          "source_revision":"770fcbf…-dirty","api_contract_version":1,
                          "assertion_contract_version":1,"schema_version":20}}
GET  /api/companies   → 401 {"error":"no_session"}            (no session)
POST /entry  (valid)  → 200 + Set-Cookie: restless_session=…; HttpOnly; Secure; SameSite=Lax
GET  /api/companies   → 200                                   (with that session)
POST /entry  (replay) → 401 {"error":"assertion_replayed"}
POST /entry  (expired)→ 401 {"error":"assertion_expired"}
POST /entry  (wrong aud) → 401 {"error":"assertion_wrong_audience"}
```

With a session scoped to one company, on the same plane:

```text
GET /api/companies/other/cockpit → 403 {"error":"company_out_of_scope"}
GET /desktop/other/observe       → 403 {"error":"company_out_of_scope"}
GET /api/companies/aris/cockpit  → 404   (past the scope gate, into the handler —
                                          which is what makes the 403s above mean something)
```

**A defect found by trying to run it, and fixed.** The plane refused to boot unless at least one
company had a usable model provider: `no configured company model provider is available for the
model gateway`. Under the tier split that is wrong. The account plane is not a company — it serves
the cockpit, holds the owner's credentials and performs no company work, so it must start with zero
startable companies and let each report its own unstartable reason, which is S25-T1's rule applied
one tier up. It also inverts Cloud's provisioning order, where Fleet creates the plane *before* the
first cell, so a freshly provisioned hosted plane could never have started. It now warns and serves.

**The account plane runs as a container and serves its cockpit.** Built from the working tree and
tagged `worktree-verify` — deliberately not a release, and deleted afterwards so it could not be
deployed — then run with its port published:

```text
owner gateway listening addr=0.0.0.0:7788
GET  /health → {"core_version":"0.0.0","source_revision":"83bc1ea…-worktree",
                "api_contract_version":1,"assertion_contract_version":1,"schema_version":20}
GET  /       → HTTP 200, 1434 bytes   (cockpit shell, served from the image)
POST /entry  → 200 + Set-Cookie: restless_session=…; HttpOnly; Secure; SameSite=Lax
```

The image is 43.9 MB against the company image's 4.16 GB — the intended shape for the tier that holds
the owner's credentials.

**Four defects, none of which reading would have found.** Building and running is the whole point of
T4, and it paid for itself in its first three attempts:

1. **`dev` does not compile** (below).
2. **The build context excluded a compile-time dependency.** `context.rs:21` `include_str!`s
   `docs/COMPANY_OPERATING_RULES.md`; `.dockerignore` is an allowlist and the company image never
   needed it, because that image builds `-p restless`, not `-p restlessd`.
3. **Allowlisting it was not enough.** `.dockerignore` permits; the Dockerfile must still `COPY`.
4. **The plane could not serve its cockpit from an image.** The SPA was resolved through
   `runtime::source_root()`, which requires `Cargo.toml` *and* `infra/company-image/Dockerfile`
   because its other caller digests the source to decide whether to rebuild the Runtime image. Two
   unrelated questions shared one answer, so a packaged plane refused to serve.
   `RESTLESS_COCKPIT_DIR` now answers the first independently.

**A gap defect 4 exposes, recorded rather than fixed:** building the company Runtime image is a
**fleet** concern under `CELL_ARCHITECTURE.md`, but it lives in the plane and needs a source
checkout. For Cloud the plane should consume a pinned Runtime digest from the release manifest. That
is a real divergence between the release contract and the code, and it lands on Cloud 02.

**T4's image build found that `dev` does not compile.** Building the account-plane image was the
first thing in this repository to compile HEAD rather than a working tree, and it failed:

```text
error[E0063]: missing fields `attempt_id` and `work_id` in initializer of `StaffRun`
  --> crates/restlessd/src/staff/conversation.rs:437
error: method `interrupt_message` not found for `AgentActivityStreams`
```

`780342a feat(cell): give each company its own database, role and spend ledger` added `work_id` and
`attempt_id` to `StaffRun` in `staff/execution.rs` without updating its caller in
`staff/conversation.rs` in the same commit. The branch has not built since. It predates this sprint —
no commit here touches `execution.rs`, `conversation.rs` or `activity.rs` — and the working tree
carries uncommitted changes to all three that make it compile locally.

This is a live hole in "never report green without running it": every `cargo build` and `cargo test`
in this repository, including this sprint's 221 passing tests, compiles the **working tree**. Nothing
compiles what is committed. A release artifact does, which is why building one found it in its first
minute, and why T4 is worth finishing rather than deferring.

**T4 is therefore blocked on a compiling `dev`, not on disk.** Host headroom was sufficient — the
build ran for 77 seconds and failed on type errors, not space. The image cannot be built from the
working tree instead: the manifest emitter refuses a dirty tree, correctly, because a release names
a revision and a dirty tree is not the revision it names. Fixing it means committing another
session's in-flight substrate work, which is not this sprint's to commit.

**Not yet observed, and therefore not claimed.** T4 stays `[~]`: the release *identity* is live, but
no OCI image digest and no published manifest exist, for the reason above. T5 stays open: the live run above used `curl`, not a
browser against a non-loopback address, and it did not compare audit attribution against a local-mode
run. The scope refusal was checked on a plane holding zero companies, so the 403s prove the gate but
the 404 — not a second company's data — is what sat behind it.

## Non-goals

No hosted deployment, no Cloud infrastructure, no Better Auth, no owner account system and no fleet
automation is claimed by this sprint. Cloud owns the issuing half; this sprint owns the verifying half
and the artifact. A test issuer is sufficient here and must not grow into a Core identity product.
