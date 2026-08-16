# Endpoints the cockpit needs and the daemon does not have

Two of the seven are now done — §2 (the attention queue) and §7 (creating a company). Both are
struck through below with what actually shipped, because a list that only grows is a list nobody
reads twice.

The rest are **registered and stubbed** in `crates/restlessd/src/api.rs`. They answer:

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

## 2. ~~`GET /v1/companies/{company}/attention`~~ — **done, and it lives at `/api`**

Implemented by S05-T1 as `GET /api/companies/{company}/attention`, on the owner gateway rather
than in this shim. The `/v1` stub was **deleted** rather than pointed at it: two paths to one
projection is the accumulation the working agreement warns off, and the gateway's is the one
with a credential in front of it.

Wired into the Inbox on the frontend branch. The nav badge is real, from
`$lib/model/attention.svelte`, read once per window so the count and the stack cannot disagree.

What shipped is better than what this section asked for, in two ways worth recording:

- Each item answers five questions rather than one: `what_happened`, `why_it_matters`,
  `recommendation`, `requested_action`, and **`if_no_action`**. The last is the one this document
  did not think to ask for and is the most valuable — an owner who cannot see the cost of ignoring
  a request learns to clear the queue rather than read it.
- `source_health` reports whether each plane could answer. The projection degrades rather than
  failing, so a partial queue is normal, and without this field a degraded source renders as
  "nothing needs you" — the one lie this surface cannot tell.

Still open, and deliberately: **there is no priority model.** Items are sorted by `created_at`.
`owner-cockpit` §5.4 wants "ordered by what blocks the most work", which needs a notion of how
much a blocked commitment is holding up, and nothing measures that yet. Oldest-first is honest
and the surface says so.

Also still open: **messages are not in the queue.** That is a decision, not a gap. Mail is read
separately and shown separately, because a count that mixed "a decision is waiting" with "you have
unread mail" would mean neither. Revisit only if real use shows the owner wanting one pile.

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

**Related.** §2 (`/attention`) landed **without** this behaviour — reading the queue does not
resolve anything in it, and `attention::project` cannot: it is a projection with no write path.
That is now the contract, recorded in `openapi.yaml`. The defect below is confined to mail.

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

## Things found while running it

None is a missing endpoint; all are worth a decision.

**The daemon will not start without `omp` on the host.** `model_gateway::start` shells out to
`omp` (`@oh-my-pi/pi-coding-agent`) to run the credential broker, and a missing binary is a fatal
start — deliberately, per the comment at `main.rs:361`: "no configured Exec can think without it".
But the requirement is written down nowhere. `.env.example` does not mention it, neither README
does, and the container's Dockerfile is the only place the install line exists
(`bun install -g @oh-my-pi/pi-coding-agent@17.2.15`, needing bun ≥ 1.3.14). Anyone cloning this
repo hits `Error: create OMP auth-broker bearer / No such file or directory` with nothing to
search for. Worth a line in `.env.example` and a check with a useful message.

**The agent cannot select a subscription-plan model.** Creating `harbourline` on
`moonshot/kimi-for-coding` produced a wake that failed with `No model selected`. The model is not
missing — the gateway offers it, and `/v1/models` through the gateway lists 21 models — but omp's
own catalogue inside the container knows only 17. The four it does not know are exactly the
Kimi-For-Coding plan's: `k3`, `k3-256k`, `kimi-for-coding`, `kimi-for-coding-highspeed`. So
`omp acp --model moonshot/kimi-for-coding` cannot resolve the name it was given.

Probing the provider directly on 16 Aug sharpens this, and corrects the framing above. `GET
{MOONSHOT_BASE_URL}/models` with the configured key returns **exactly those four ids and nothing
else** — `api.kimi.com/coding/v1` is not a subset of the Moonshot catalogue, it is a different
catalogue. So the disagreement is total: every model this key can serve is one omp does not know,
and every model omp knows is one this key cannot serve. There is no id that satisfies both, which
is why repointing the company at `moonshot/kimi-k2-0905-preview` did not help either — that id is
absent from the key's catalogue too.

This is the second failure caused by the same plan (see the spend poison below), and together they
say something worth acting on: **subscription plans are not a supported way to run a company.**
The metered path works end to end; the subscription path fails at model selection, and if it got
past that it would poison the ledger. Either omp's catalogue must be extendable from the gateway's
`/v1/models`, or a company on a plan omp cannot enumerate should be refused at `company-create`
with that reason, rather than accepted and failing at the first wake.

**The Inbox offers an action the runtime cannot honour, on a page that says so.** Screenshotting
every surface on 16 Aug put both facts in one frame. The banner across the top of the Inbox reads
"This queue is incomplete — browser is degraded", which is correct: `runtime::doctor` reports
`desktop`, `chromium`, `automation` and `web_transport` all `unavailable`, and the container is
running `tini` and `sleep` and nothing else. Four hundred pixels below it, the item's only action
is **"Open live browser"**. `attention::project` builds that action without consulting the browser
health it puts in the same response, so the projection contradicts itself within one payload. This
is worse than the errand misclassification below: an owner who clicks it learns the product's calls
to action are not load-bearing. The action should be withheld — with the reason — when the runtime
cannot serve it.

*Cause, found afterwards, and it is not the one this paragraph implies.* The browser stack is
built and works: `restless up -c harbourline --reconcile` rebuilt the image and the same probe
returned `desktop`, `chromium`, `automation` and `web_transport` all `available`, with Xtigervnc,
openbox, ten Chromium processes, websockify and the broker running in the container. Harbourline
had an image built before the sprint-5 browser stack landed, because **the cockpit never
reconciles**: `client.ts:159` sends `reconcile: false`, deliberately (a rebuild is minutes and a UI
should not hang), so a company created through `/start` runs whatever image is on disk, silently.
Two separate items, then: the create path needs a way to reach a current image — or to say which
one it got — and `attention::project` still builds `open-browser` unconditionally at
`attention.rs:262`, so the contradiction recurs on any company whose runtime is degraded.

**A refused wake never honours its schedule, so it re-fires every five seconds forever.** Observed
live on 16 Aug: `aura` had emitted **10,669** `wake_end` events since 15 Aug against 3 `wake`
events, one every five seconds for eighteen hours, each writing an identical message row. The
mechanism: `schedule.rs:247` treats a scheduled wake as honoured only when a `wake` event is newer
than the schedule, and `exec.rs:95` refuses on the budget preflight through `blocked_wake` —
returning *before* any `wake` event is written. aura's last schedule is `10:50:24` and its last
`wake` is `10:48:52`, so `honored` is false and cannot become true. Every refusal in that preflight
block (stopped container, full disk, budget) returns by the same path, so any of them puts a
company into this loop permanently. The refusal must still record that the schedule was seen.

**The fail-closed poison sentinel is printed to the owner as money, with a remedy that cannot
work.** The message above reads `aura has spent $18446744073709.55 of its $20.00 ceiling; the owner
must raise it before work continues`. That figure is `u64::MAX` micro-USD — the poison sentinel.
`main.rs:186` already defines `POISON_SENTINEL_USD` for exactly this, and the `spend` path guards
against printing it, with a comment calling it "a fabricated figure where the honest answer is
poisoned". `exec.rs:95` does not use the guard. Raising the ceiling does nothing to a poison;
`restless clear-poison` is the fix and the message never mentions it.

**Eleven cockpit controls have no handler.** Enumerated from source, not sampled by clicking:
`New run` (the primary CTA), the ⌘K search (a `<span>`, not an input), Board's `+ New task`,
`Active` filter and cards, People's `Hire someone`, `Pause` and `Revise role`, and `+ new goal`
(an `<a href="/board">` to the page you are already on). The read path is fully wired; the write
path is wired on the Inbox, the chat dock and `/start`, and nowhere else. `POST /staff` exists in
the API with no caller anywhere in the SPA — `Hire someone` is where it would go.

**The create-company form defaults to a model that does not exist.** `start/+page.svelte:28` is
`let model = $state('moonshot/kimi-k2-0905')` — a value, not a placeholder, so it is what an owner
gets by accepting the form. That id is in no catalogue: the published Moonshot one is
`kimi-k2-0905-preview`, and the configured key serves neither. The default path through the
product's own onboarding therefore creates a company that cannot think, which is the exact failure
the rest of this section documents. Placeholder, or a value read from the gateway's `/v1/models`.

**An infrastructure failure is presented to the owner as an errand.** The failed wake above became
an attention item whose `what_happened` is a raw ACP error — `acp session/prompt: Internal error:
{ "details": "No model selected.\n\nUse /login, ...` — and whose `requested_action` is "Take the
prepared last mile in the live company browser", with a single action labelled "Open live browser".
No browser can fix a missing model. `attention::project` classifies every blocked commitment as
`review` or `blocker` by keyword and gives blockers the browser action unconditionally, so a
company that cannot think is indistinguishable from one waiting on a human step. A blocked
commitment whose resolution came from a transport or configuration failure is a third thing, and
it belongs to the operator, not to the CEO.

**Changing a company's model does not reach the People surface.** `restless company set -c
harbourline model moonshot/kimi-k2-0905-preview` rewrites the config, and the next wake uses it,
but `harbourline.actors.model` still reads `moonshot/kimi-for-coding` — the value seeded when the
actor was created. `GET /people` reads the actor row, so the surface confidently shows a model that
is not the one in use. Either the actor's model is a projection that `company-set` refreshes, or
`/people` should read it from the config and the column should go.

**One company with an unset key stops every company.** `provider_keys` walks *all* configured
companies at boot and fails the whole start if any one of them has no credential for its
configured model. A scratch company left on `zai/glm-5.2` therefore takes the daemon down for a
live company whose key is present. Fail-closed is right; the granularity is not. A company whose
key is missing should refuse to *wake*, and say so on its own surface, rather than preventing the
daemon from serving the others.

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
