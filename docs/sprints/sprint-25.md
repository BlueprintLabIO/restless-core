# Sprint 25 — Separate the account plane from the company cell

**Status:** Active — T1–T3 and T5–T7 landed and are verified against real runs. T4 is blocked on an
external capability (below); T8 remains.

**Date:** 29 August 2026

**Target:** [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §7.4 ·
[`docs/CELL_ARCHITECTURE.md`](../CELL_ARCHITECTURE.md) ·
[cross-layer contract §1.4](../specs/cross-layer-contract.md)

---

## Why this sprint exists

The specs already say cell-per-company. `orgintel.md:1166` calls for "one per-company OrgIntel service
and OrgIntel Postgres"; `company-runtime.md:82` calls for "one per-company Docker Compose deployment".
**The code says something else**, and the divergence was invisible until an owner tried to list their
companies:

- One `restlessd` process serves every company on an installation. It simultaneously holds the
  company's OrgIntel connection, the *owner's* provider credentials and OAuth, and the *host's* Docker
  socket — three trust domains fused into one process and one failure boundary.
- OrgIntel isolation is a Postgres schema per company inside one database behind one role
  (`0001_init.sql:1`). Any cell's connection can read every other schema. That is a convention, not a
  boundary.
- The spend ledger is one `spend.jsonl` per installation with a `companyId` column — the same
  shared-store-with-a-company-column pattern.

Three observed failures, all the same defect seen from different ends:

1. **One stale company config bricked every company.** Two abandoned `_test` companies named a
   provider with no credential, and `aris` named `anthropic` with no OAuth in the broker. The daemon
   validated every company's full model chain at boot and refused to start at all — including for the
   companies whose own credentials were fine.
2. **The CLI reported "is restlessd running?" while three daemons were running.** Each was on its own
   `RESTLESS_HOME` and `RESTLESS_PORT_OFFSET`, with no registry, so the only way to find the right one
   was `lsof` and `ps -E`. The error was not unhelpful; it was false.
3. **`V0 model gateway refuses different {provider} credentials across companies`** is a bail in
   `model_gateway.rs`. It reads as an isolation boundary and is actually the opposite: an
   **owner-scoped** resource that has not learned to serve companies with differing needs.

## Outcome

The deployment tiers are the plane boundaries, enforced by one rule:

> **Effects execute where the credential lives. The cell requests; it never holds.**

- **Cell — per company:** OrgIntel in its own database, Company Runtime in its own container.
- **Account plane — per owner:** the Authority Plane. Credentials, effect execution, approvals, budget
  enforcement, cockpit, CLI endpoint.
- **Fleet — per host:** container lifecycle. No credential, no company state.

Tested by: **each tier must be independently restartable without losing data or work in the others.**

## Acceptance criteria

1. A company whose model route cannot be resolved is marked unstartable with the exact reason, and
   **every other company still starts**. Headless: boot a plane whose config includes one unroutable
   company; the plane listens, the owner API reports `unstartable_reason` for that company only, and
   waking it fails with that reason.
2. A CLI pointed at a home with no plane names the live planes and how to reach them. Headless: run
   any `restless` verb with `RESTLESS_HOME` set to an empty path while a plane runs elsewhere.
3. Each cell holds a dedicated database with its own role. Headless: connect with cell A's credentials
   and fail to read cell B's tables.
4. Restarting the account plane does not stop a running company. Headless: wake a company, restart the
   plane, observe the company still running and its next Attempt proceeding.
5. No owner surface auto-wakes a cell, and the cockpit renders every company with all cells asleep.

## Slice per layer

- **Authority Plane / account plane** — admission, credential custody per company, plane registration.
- **OrgIntel / cell** — per-company database and role; `LISTEN/NOTIFY` wake path proven per-database.
- **Runtime / fleet** — cell lifecycle extracted from the plane; supervisor registration.
- **Out of scope** — splitting the binary into three crates. Do it when a proved slice needs the
  ownership boundary (§16.1), not as a prerequisite.

## Ticket decomposition

Status lives only in this checklist; ticket files record scope and closure evidence, not a second
status system.

| Status | Ticket | Slice | Outcome or friction served | Prior machinery made deletable |
| --- | --- | --- | --- | --- |
| [x] | **S25-T1 · Admit companies per cell, not per plane** | Authority | One stale config bricked every company | Whole-plane boot validation of every company's chain |
| [x] | **S25-T2 · Publish and discover account planes** | Authority + CLI | The CLI denied that running daemons existed | `lsof`/`ps` archaeology; `RESTLESS_HOME` guesswork |
| [x] | **S25-T3 · Give each cell its own database and role** | OrgIntel | Schema tenancy behind one role is not a boundary | Shared-role schema separation; cross-schema reachability; the single admin wake listener |
| [!] | **S25-T4 · Per-company credential custody** | Authority | The multi-account bail blocks a real owner need | `V0 model gateway refuses different {provider} credentials` |
| [x] | **S25-T5 · Per-cell spend ledger with plane rollup** | Authority | One installation-wide `spend.jsonl` with a company column | Shared ledger file; company-column filtering |
| [x] | **S25-T6 · Supervise the plane; never auto-wake a cell** | Authority + Runtime | Two-terminal dance; risk of silent spend | Manual `restlessd` launch as the normal path |
| [x] | **S25-T7 · Cockpit readable with every cell asleep** | Owner Cockpit | Owner pays to look at their own business | Wake-on-view; live-only cockpit reads |
| [ ] | **S25-T8 · Prove plane restart does not stop a cell** | Evaluation | The invariant that tests whether the split is real | Assumed independence; untested restart paths |

## Evidence

T1 and T2 were verified against the exact configuration that previously refused to boot
(`~/.restless`, containing `aris` with an `anthropic` failover and no broker OAuth, and
`exp12_attio_test` with an unroutable primary):

```text
WARN provider not admitted: host model broker has 0 OAuth credentials for anthropic
WARN dropping failover candidate: no usable host credential for provider anthropic
       company=aris model="anthropic/claude-sonnet-4-5"
WARN company cannot start: no usable host credential for model openai-codex/gpt-5.6-sol
       company="exp12_attio_test"
INFO restlessd listening socket=/Users/yao/.restless/restlessd.sock
```

- Owner API reported `unstartable_reason` on `exp12_attio_test` alone; the other six companies were
  clean.
- `restless up -c exp12_attio_test` → `Error: company exp12_attio_test cannot start: no usable host
  credential for model openai-codex/gpt-5.6-sol; set credentials.model.inference.openai-codex`
- `restless status -c aris` → `aris: Stopped` (admitted, its dead failover candidates dropped).
- `RESTLESS_HOME=/tmp/restless-nonexistent restless company list` named the live plane, its pid and
  its seven companies.
- 185 daemon unit tests pass, including the new admission invariant.

**S25-T3** — provisioned and verified on an isolated home with one throwaway company:

- `restless_cell_celltest_test` database created, role created, `cells/<company>/database.url` at
  `0600`; migrations applied inside the `celltest_test` schema in that database.
- **Isolation:** connecting as the cell role and reading a neighbour returns
  `ERROR: permission denied for schema aris`.
- **Wake path proven, not assumed.** A raw `LISTEN restless_orgintel` on the cell database received
  `{"company":"celltest_test","kind":"message",...}` — `TG_TABLE_SCHEMA` attribution survives the move
  to a per-cell database. The scheduler then logged `cell wake received` for the same payload, so
  delivery was confirmed end to end rather than inferred from the trigger firing.
- The single admin-connection listener was replaced by one listener per cell, multiplexed; a single
  listener would have heard nothing and silently degraded every wake to the 5s scan.

**S25-T5** — `each_cell_ledger_holds_only_its_own_history` proves a cell takes only its own rows out
of the legacy shared spool and keeps its accumulated total (starting a migrated company from zero
would silently raise its budget). The shared spool is left in place for operator verification. A cell
whose ledger cannot be opened reports `MeteringUnknown`, never "spent nothing".

**S25-T6** — with no plane running, `restless company list` started the plane and answered in 5.5s;
the second invocation answered in 5ms. With a plane running on another home, the CLI refused to start
a second and named the running one. `infra/launchd/io.restless.plane.plist` supervises the plane so
the fallback is rarely reached.

**S25-T7** — loading the full cockpit projection for a company whose container has never existed
returned `source_health: {orgintel: available, runtime: absent}` with **0 containers started and 0
spend rows written**. The cell's database is a host service, so a sleeping cell is still readable.

**S25-T3 on real owner data (30 August 2026).** The six configured companies in `~/.restless` were
migrated by booting the plane; a per-table baseline was captured first and every schema dumped to
scratch as a belt-and-braces backup.

- 114 tables across six companies compared before and after. **Every count matched**, except `aris`,
  which gained one `events`, one `messages` and one `external_message_sources` row — a genuine inbound
  external message that arrived after the baseline snapshot.
- **The legacy schemas stayed frozen at the baseline** (`aris` legacy: 3338 events / 272 messages / 6
  sources) while the new rows landed in the cell. That is the migration's real proof: the new path is
  live and the old one is dormant, rather than both being written.
- **Spend:** `aris` had 515 rows in the installation-wide spool and has 515 in its own cell ledger,
  containing no other company's `companyId`. The cockpit reports `accounted_usd: 71.755` against a
  `200.0` ceiling — had extraction failed this would read `0` and silently grant a full budget.
- **Isolation, both directions:** the `aris` cell role reading `cosmon.actors` →
  `permission denied for schema cosmon`; opening `restless_cell_cosmon` →
  `FATAL: permission denied for database ... User does not have CONNECT privilege`.
- **Idempotent:** a second boot performed **0** re-imports, all six cells re-attached their wake
  listeners, and row counts and the spend ledger were unchanged.
- The shared spool and every legacy schema remain in place for owner verification; nothing was dropped.

**Blocked — S25-T4.** Per-company credential custody needs the model path to select a credential per
request. The relay forwards one bearer to one OMP `auth-gateway`, and OMP's broker canonicalises to
exactly one credential per provider, so per-company selection needs either an OMP capability we have
not probed or one gateway process per credential set. Not guessed at: the bail stays until the
external capability is established.

## Deletion record

- Whole-plane boot validation of every company's model chain — replaced by per-company admission.
- `Processes.unstartable` field — one fact, one accessor; the struct field was a second source.
- The `bail!` on broker canonicalisation failure — a provider that cannot canonicalise is now dropped
  and its dependent companies marked, rather than stopping the plane.

Still to delete, once their tickets land: `RESTLESS_PORT_OFFSET` (T6 — a supervised per-owner plane
plus compose-isolated test cells removes the need to rewrite ports on one host), the multi-account
credential bail (T4), and the installation-wide spend ledger (T5).
