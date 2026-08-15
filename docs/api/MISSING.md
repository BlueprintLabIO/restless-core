# Endpoints the cockpit needs and the daemon does not have

Every route here is **registered and stubbed** in `crates/restlessd/src/api.rs`. They answer:

```json
{ "ok": true, "data": null, "stub": { "implemented": false, "what": "...", "see": "docs/api/MISSING.md" } }
```

`data` is `null` and never `[]`. An empty list is a claim — "there are no settings" — and a UI
renders it as a finished, empty surface. `null` cannot be mistaken for content, so the frontend
shows "not built yet" instead of quietly lying about the company.

They are stubs rather than absent routes so the SPA can be written against the agreed shape now,
and so the gap is visible in `openapi.yaml` and at `/v1/docs` rather than living in someone's head.

---

## 1. `GET /v1/companies/{company}/authority`

**Surface:** Authority — the entire page. Nothing on it is backed today.

**Use case.** The owner wants to read, top to bottom, everything the company may do without
asking them. Today that answer is spread across a TOML file (`spend_ceiling_usd`, `providers`,
`credentials`, `approved_parties`), a hardcoded `OWNER_ONLY` list in the daemon, and the
approval rule in `approval.rs`. There is no way to ask for it.

**Suggested shape.** One flat list, grouped, each row carrying who it applies to, what they may
do, its standing, and who set it and when:

```json
{ "groups": [ { "id": "money", "name": "Money", "icon": "wallet",
  "rows": [ { "id": "...", "subject": null, "setting": "Monthly ceiling",
              "standing": { "label": "$4,000 all in", "tone": "ok" },
              "set_by": "you · 2 Jun", "invariant": false } ] } ] }
```

**Note.** `invariant: true` rows — "sign a contract", "stop everything" — must come from the
daemon, not be hardcoded in the SPA. A limit the frontend invents is not a limit.

**Estimated work.** Small. `CompanyConfig` already holds most of it; this is a serializer plus a
decision about where the invariants are declared.

---

## 2. `GET /v1/companies/{company}/attention`

**Surface:** Inbox — the stack, and the count on the nav.

**Use case.** The Inbox shows one merged stack of everything waiting on the owner, ordered by
what blocks the most work. Three separate reads exist (`inbox` for messages, `commitments` for
blocked work, and the approval decision inside `effect`), but nothing unions them, and nothing
answers "how many are waiting" — which the top nav badge needs on every page.

**Suggested shape.** The attention envelope from `owner-cockpit` §5.2, one array, each item
carrying its kind, who raised it, what it is blocking, and the standing setting that made it a
question rather than an action:

```json
{ "waiting": 3, "items": [ { "id": "...", "kind": "approval|message|blocked",
  "title": "...", "raised_by": "sage", "why": "Sage may draft outside mail but not send it",
  "blocking": ["commitment-uuid"], "at": "..." } ] }
```

**Note.** The `why` field is the load-bearing one. An approval request that cannot say which
standing setting produced it trains the owner to click through.

**Estimated work.** Medium. The union is easy; the priority model (§5.4) is the real work.

---

## 3. `GET /v1/companies/{company}/people/{actor}`

**Surface:** People — the detail pane, which is most of the page.

**Use case.** `GET /people` returns a list row: id, role, model, spend, whether a session is
running. The page needs one person in depth — what they are doing right now and under which
run, the work on their plate, what they may do alone versus what needs your word, their spend
against their own ceiling, and what they have produced lately.

**Suggested shape.** Composable from what exists (`commitments` filtered by `owner_id`, the
per-actor slice of `spend`) plus items 1 and 5. Worth one endpoint anyway: the SPA should not
have to make five calls and join them to render one pane.

**Estimated work.** Small once 1 and 5 exist. Mostly composition.

---

## 4. `GET /v1/companies/{company}/org`

**Surface:** People — the reporting-tree view (the toggle in the directory header).

**Use case.** Show who answers to whom, rooted at the owner. Used when deciding where a new
instruction should enter the company rather than who is busy.

**The gap is in the data, not the API.** `ActorRow` is `{id, kind, display, model, created_at}` —
there is no parent. The edge exists in the event stream, because `spawn` records who spawned
whom, but it is not on the actor and cannot be queried.

**Suggested shape.**

```json
{ "root": "owner", "nodes": [ { "actor_id": "exec", "reports_to": null, "reports": 4 },
                              { "actor_id": "sage", "reports_to": "exec", "reports": 0 } ] }
```

**Note.** This needs an OrgIntel column (`actors.reports_to`) before the endpoint means anything.
Until then the tree view in the SPA is a drawing — it currently renders from fixtures.

**Estimated work.** Medium — a schema change, and a decision about whether spawn-parent *is*
reporting line or merely correlates with it.

---

## 5. `GET /v1/companies/{company}/artifacts?actor=`

**Surface:** People — "made lately" on the person page.

**Use case.** Show what an actor actually produced — file paths, repo commits, URLs — so the
owner can check the work rather than read a claim that it happened.

**Partly there.** `OrgIntel::add_artifact_ref` writes these; nothing lists them. Per
ARCHITECTURE §5.3 these are ordinary references — path, repo+commit, worktree+branch, or URL —
and must not become a custody state machine.

**Estimated work.** Small. A list query plus a command.

---

## 6. A non-consuming read of the owner's inbox — `GET /inbox` mutates

**Surface:** Inbox. **This is a defect, not a missing feature, and it is the most
consequential thing found while wiring the SPA.**

`GET /v1/companies/{c}/inbox` marks every message it returns as read. Verified against a live
daemon on 15 Aug: unread went from 2 to 1 on a single request, with no write anywhere.

That breaks HTTP's contract — a GET must be safe — and in a browser it is worse than untidy.
A refresh, a prefetch, a double render in dev, a background tab waking, or a crawler on
`localhost` all silently destroy the owner's unread state. The Inbox is this product's attention
queue; consuming it by looking at it is the one failure mode this surface cannot have.

**There is no workaround.** `?as=<actor>` inspects without marking, but the query is
`to_actor IS NOT DISTINCT FROM $1`, and the owner's own mail is stored with `to_actor IS NULL`.
So `?as=owner` matches nothing and returns an empty list — which reads as "no mail" rather than
"wrong question". The SPA currently renders through the consuming read and says so in a banner.

**Suggested fix.**

- `GET /inbox` never marks. Add `?unread_only=true|false` if the filter is wanted.
- `POST /inbox/seen` with an explicit id list, or `{"all": true}`, marks read.
- Optionally let `?as=owner` resolve to `to_actor IS NULL`, so "the owner" is addressable by
  name rather than by absence.

**Estimated work.** Small — the marking loop already sits in the `inbox` command handler in
`main.rs`, separate from the query. It is a move, not a rewrite.

**Related.** Once §2 (`/attention`) exists it must not inherit this behaviour: reading the
attention stack must never clear it.

---

## 7. ~~`POST /v1/companies` — create a company~~ — **done**

Implemented. A company can now be created over HTTP or with `restless new`, instead of by
hand-writing a TOML. `runtime::create_config` writes the config and stops; `up` still starts it,
so a failed launch does not lose the company the owner just described.

Two things were fixed on the way in:

- **The name is validated before anything is created.** `restless_orgintel::valid_schema_name` is
  now `pub` and `runtime::up` calls it too. Previously an invalid name got a Docker volume and a
  container and *then* failed at the schema step, leaving orphans named after a company that
  could never exist.
- **A new company starts with no reach into the world** — no providers, no credentials, no
  standing approvals, no sender address — matching what `clone_config` already stripped, and for
  the same reasons.

Still true, and still blocking the designed flow: screens 06–08 show goals, staff and ceilings
being *derived* from the owner's first sentence. That is the Exec's first turn, so it needs a
model key before any of it can be demonstrated.

---

## Not missing, but unused

`GET /v1/companies/{company}/receipts` is **implemented and backed**, and nothing in the current
design renders it. Receipts are the strongest evidence the system holds about what it actually
did to the world — capability, provider, party, outcome, idempotency key. That is a gap in the
design, not in the API, and it is worth a surface.

---

## Deliberately not proposed

- **A four-level goal tree.** The Board draws objective → goal → step → task; OrgIntel has goal →
  commitment. Two levels. Adding two more to satisfy a drawing would be modelling ahead of a real
  workload (`ARCHITECTURE.md` §16.1). The drawing should shrink first.
- **Per-message threads for the dock.** The dock is `tell` plus the event stream. A durable
  conversation model is a first-class entity nobody has needed twice yet.

---

## Two more things found while running it

Neither is a missing endpoint; both are worth a decision.

**A read creates the company.** `GET /v1/companies/anything/goals` returns `{"ok":true,"data":[]}`
and, as a side effect, provisions a full OrgIntel schema named `anything` — `OrgIntelRegistry::get`
calls `OrgIntel::ensure`. Over a socket that was reachable only by someone who could already run
the CLI. Over HTTP it is reachable from a URL bar, so a typo silently creates a schema. Either
make reads fail closed on an unknown company, or keep the behaviour deliberately and say so.

**A subscription plan poisons the spend ledger on every turn.** `spend.rs:55` — a turn whose
`cost_usd` is `None` poisons the company fail-closed, on the reasoning that "unaccounted spend and
unbounded spend are indistinguishable". That is right for a metered key and wrong for a
subscription: Kimi-For-Coding reports tokens (24,862 on the first real turn) and **no dollar
cost**, because the plan is flat-rate. So every wake succeeds, produces work, and then blocks the
next one — `restless clear-poison` is needed between every single turn.

Verified live on 15 Aug against `moonshot/kimi-for-coding`. Claude Pro/Max and ChatGPT plans will
behave the same way whenever they are reachable, so this blocks subscription-based agent auth
generally, not just this one key.

The fix is a decision, not a patch: either the provider entry declares itself flat-rate and the
fuse switches to a token budget, or unpriced turns are recorded at zero with the ceiling
explicitly not enforced for that provider. Silently treating unknown cost as zero would be the
wrong one — that is the failure the poison exists to prevent.

**`message` leaks a database constraint.** `restless message --from exec ...` on a fresh company
fails with `insert or update on table "messages" violates foreign key constraint
"messages_from_actor_fkey"`. `tell` already guards this — it seeds `owner` and `exec` first,
with a comment saying the owner's first interaction must not fail on a machinery detail — but
`message` does not, and the raw Postgres error reaches the surface with `kind: "error"`. A
typed refusal naming the unknown actor would be honest; the constraint name is not.
