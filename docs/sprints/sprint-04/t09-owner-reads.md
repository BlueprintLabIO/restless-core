# S04-T9 · The owner can see the team, without psql

**Layer:** Owner surface (CLI) over OrgIntel and Authority read paths
**Serves:** AC5 and AC7 of this sprint, which are currently unowned by any ticket
**Depends on:** T5 (roles and models exist to be read)
**Makes deletable:** "read the Exec's prompt to find out what the company did" as the only way to see a receipt

---

## This is not new scope, it is unassigned scope

Two of this sprint's acceptance criteria already require owner-visible reads, and no ticket builds them:

- **AC5** asks for *"cost reported per role."* `spend.rs:47` records `(company, model, used, cost_usd)`.
  There is no actor dimension anywhere in the ledger. The number does not exist to be printed.
- **AC7** asks that the owner can say *"which role, which model, what it cost, and what it produced —
  answerable from OrgIntel without reading a log."*

T5 landed roles and models. Nothing reads them back. Without this ticket, T8's run would discover
both gaps at the end of the sprint, with the run already spent.

## The observed friction

The receipt ledger is the sharp case. `reconcile::effect_ledger` computes what the company's receipts
actually record, per capability, with outcomes — the strongest observation the system holds, and the
one `context.rs:164` tells the Exec *wins over its own journal*. Its only consumer is `context.rs:39`,
which folds it into the Exec's prompt.

**The Exec can read its own receipts. The owner cannot.** For an owner-facing product whose §2.7
principle is *evidence before self-report*, the evidence is currently addressed to the agent.

Sprint 01 recorded the same shape on the agent side and fixed it: *"no way to ask what [the
capabilities] were, so Aris guessed ~95 names and gave up"* (`sprint-01/run-report.md:42`), which is
why `effect::available_capabilities` exists. This is that finding, pointed at the human.

## Scope

Three reads and one source change. The nouns are not invented here — they are `owner-cockpit` §4.1's
vocabulary table and §14.6's identifier list, already settled, which is why this does not wait on
S03-T8.

1. **`restless receipts [-c] [--capability <cap>] [--limit N]`** — over the existing
   `reconcile::effect_ledger` and the receipt rows behind it. Per row: capability, party, provider,
   outcome, idempotency key, cost. Outcome uses `reconcile::Outcome`, not prose.
2. **`restless people [-c]`** — actor rows: `actor_id`, kind, role, model, display name, and whether a
   session is currently running. Requires `list_actors` on `OrgIntel`; today only `add_actor` exists.
   The `kind != "staff"` property AC5 asks to observe is read here.
3. **`restless spend [-c]`** — company total against ceiling, remaining, poison state, broken down by
   model **and by role**. The poison state is already reachable only by clearing it
   (`clear-poison`); this makes it legible before the owner decides to clear it.
4. **Source change: the spend ledger gains an actor dimension.** `SpendRecord` and
   `SpendLedger::record` carry `actor_id`; the role is joined from OrgIntel at read time rather than
   copied into the ledger, so OrgIntel stays the single writer of role (`cross-layer` §3.1). Every
   call site of `record_turn` passes the actor whose turn it was.

**Not in scope:** attention (T11), approvals listing (T11 covers the outstanding ones), operating
phase, goal stage, any write. These are reads.

## The rule this ticket is tested against

> The owner never needs `psql` to **operate** the company.

With one deliberate carve-out: `infra/crash-harness.sh:32` queries Postgres directly and must keep
doing so. It verifies that state survived a crash *independently of the code that claims it did*. If
the CLI became the only way to check, the check would be circular. Out-of-band verification reads the
database; operating the company does not.

## Acceptance

Headless, with observed output recorded in the run report.

1. For the change shipped by T3, the four AC7 questions are answered entirely from
   `restless people`, `restless receipts` and `restless spend` — **which role, which model, what it
   cost, what it produced** — with no `psql` invocation in the transcript.
2. `restless spend` reports a non-zero cost against at least two distinct roles, satisfying AC5's
   *"cost reported per role"* from the ledger rather than from an estimate.
3. `restless people` shows at least two actors whose `kind` is not `"staff"`, with distinct roles and
   the configured `moonshot/kimi-k3` model — AC5's first half, read rather than asserted. Independence
   comes from the critic's adversarial role and withheld drafting context, not an unavailable second
   provider.
4. `restless receipts` shows the `repo.push` receipt with its PR URL and provider, and a repeated push
   under the same key appears **once**, which is AC4 observed from the owner's side.

## What this makes deletable

Nothing structural yet — this is the first read half. It retires one workaround: inspecting
`context.rs`'s assembled prompt (or the database) to find out what a company actually did. If
`add_artifact_ref` gains its first real reader here, it stops being the orphaned write path flagged in
the sprint-02 purge.

---
Sprint spec: [`../sprint-04.md`](../sprint-04.md)
