# Sprint 17 — Signal-ready supervised operation

**Status:** Implementation complete; the founder explicitly deferred the real signed-provider
validation gate. Controlled faults, real-model production and native owner review are green.

**Date:** 25 August 2026

**Depends on:** Sprint 15's trusted Runtime boundary and the landed Sprint 16 outcome-review path.
Sprint 12's connected desktop/mobile review and Sprint 16's remaining provider handoff stay explicit
open gates; this sprint does not silently close either one.

**Evidence:**
[`EXP-04 final results`](../../experiment/coordination/experiments/EXP-04/t02-final-results.md) ·
[`EXP-04 harness conformance`](../../experiment/coordination/experiments/EXP-04/t01-wave0-conformance.md) ·
[`coordination canon`](../../experiment/coordination/CANON.md)

**Spec refs:** `ARCHITECTURE.md` §4–§7, §9 and §16; `orgintel` §6–§8 and §10;
`company-runtime` §4–§5, §9 and §11; `authority-plane` §7–§9 and §12;
`cross-layer-contract` §3–§4 and §8; `evaluation-dogfood` real-provider and `_test` rules;
ADR 0005.

**Salvage:** No legacy control-plane machinery. Reuse and revalidate the current signed Resend
ingress, Authority inbound records, OrgIntel messages and LISTEN/NOTIFY wakes, sparse Work/Attempt
path, Runtime Bridge/ACP client, governed effect runner and outcome-review handoff. Do not lift a
provider catalogue, workflow engine, universal event type or legacy custody lifecycle.

---

## Observed product gap

The latest coordination experiments produced a stable organisational prior but exposed a split
between that prior, the live product and the experimental harness.

1. The current canon says Exec always dispatches executable owner work and returns to portfolio
   availability; an accountable lead remains a non-producing supervisor and commissions at least one
   worker for an executable outcome. Earlier sections of `ARCHITECTURE.md`, `orgintel` acceptance text
   and the live lead prompt still permit direct lead production. One product currently carries two
   incompatible organisations.
2. EXP-04 resumed one isolated ACP model session across three replaceable processes, retained exact
   context, made forced backing loss explicit and charged resumed turns by usage delta. Production
   still treats every ACP session as disposable and creates a new session on every wake.
3. EXP-04 Q4 allowed useful work to begin before proving that every actor session had its coordination
   MCP tools. One actor created the exact artifact but could not report it. The direct probe then left
   the exited Attempt as `running` instead of `unknown`.
4. The lab emits an empty event for every reasoning chunk, coalesces local completions against all
   active Work rather than one causal outcome and simulates external events by polling a file every
   50 ms. These are useful diagnostic shortcuts, not production evidence.
5. Restless already accepts signed Resend webhooks, records them in Authority and projects inbound
   mail to OrgIntel. The projection is best-effort after Authority returns custody: a crash can leave
   a real email durably recorded but never delivered to the company. Dedupe prefers `email_id` over
   the webhook delivery/event id, every reply is routed to Exec, thread/Work correlation is absent and
   signed email content is not clearly bounded as untrusted evidence.

The next experiment should not run on top of these contradictions. The product first needs one real,
recoverable signal loop and one production actor/session contract.

## Founders' decision

> **A world event is evidence, not an instruction. Record it once at the authoritative boundary,
> correlate it with existing responsibility, wake the nearest accountable owner only when judgement
> or action is owed, and reconcile any external effect against provider reality.**

This sprint starts with incoming email because a partial live implementation and a real business use
already exist. It does not generalise email into a universal signal platform. A second materially
different source may reuse the observed seam later; it does not shape this sprint in advance.

The company organisation remains:

```text
owner
  → continuously available Exec
      → one accountable, non-producing lead per executable outcome
          → at least one worker owns production

outside-world event
  → existing Work/account/thread owner when correlation is exact
  → otherwise the standing accountable departmental lead
  → Exec only when ownership is absent, cross-departmental or portfolio-level
```

Exec is not a universal inbox. A lead may interpret, commission, guide, redirect and judge, but does
not draft the reply, repair the artifact or perform the planned external action itself.

## Outcome

In a dedicated real-provider `_test` company, an authenticated inbound email is durably recorded,
projected exactly once, correlated with the nearest existing account/Work responsibility and delivered
without a model poll. The responsible non-producing lead supervises a worker that prepares a grounded,
unsent response or bounded operational action. The lead inspects the native email thread and prepared
result, then presents the exact judgement or authority boundary to the owner.

A duplicate delivery, daemon restart, ACP process replacement or missing semantic callback does not
lose the email, duplicate Work, invent completion or repeat an external effect. A changed policy or
harm signal can interrupt and redirect the same in-flight Work. Routine provider events and ordinary
successful units do not create an immediate model wake merely to announce activity.

The same run proves that actor context can stay hot through a resumable, responsibility-scoped ACP
session while each OS process and signed capability remains replaceable and short-lived.

## Success contract

Sprint 17 passes only when all of the following are observed.

### One organisation

1. **One canonical responsibility boundary.** `ARCHITECTURE.md`, `AGENTS.md`, ADR 0005, OrgIntel
   contracts, Exec/lead prompts and behavioural tests all say the same thing: Exec dispatches and
   quiesces; the lead supervises and judges; a worker produces. Stale direct-lead/player-coach paths
   and tests are deleted rather than retained as an alternative.
2. **No topology router.** Lead intelligence still chooses one end-to-end worker by default and adds
   workers only for differentiated value or independently closing queue units. No rule maps task size,
   email type or queue length to team size.

### Actor and Runtime continuity

3. **Session-hot, process-cold continuity.** A model session may resume only for the same company,
   actor and bounded responsibility/conversation. A new process and new short-lived Runtime/model
   capability are issued for every wake; resumption never reuses authority credentials.
4. **Explicit reconstruction.** If the provider/session backing is missing, Restless creates a fresh
   session, marks the wake reconstructed and supplies the durable factual context. It never claims a
   hot continuation or silently changes actor identity.
5. **No historical replay as current work.** Notifications replayed during ACP `session/load` may
   reconstruct the model but cannot become new owner text, tool activity, usage, messages or
   organisational events.
6. **Honest usage.** Persist session-cumulative provider usage where supplied, charge and display
   per-turn deltas, and preserve missing cache, reasoning and pre-cancellation usage as `unknown`.
   Raw chain-of-thought is neither stored nor requested.
7. **Session-specific capability readiness.** Before the first production prompt, the exact ACP
   session proves its required native and coordination tool contract through an observed Runtime/MCP
   readiness event. Missing tools block that launch as infrastructure-invalid before productive spend;
   another actor's successful probe is not evidence for this session.
8. **Terminal truth.** A process that exits without a semantic result leaves its Attempt `unknown`,
   preserves observable files/commit and wakes accountable recovery. It never remains `running`, and
   process exit alone never means success or failure.

### Durable external signal ingress

9. **Provider fact before acknowledgement.** A signed provider event is authenticated over the exact
   bytes, bounded in size and durably recorded by Authority before the provider receives success.
   Signature validity proves delivery through the provider, not sender trust or semantic truth.
10. **Correct identity and idempotency.** Redelivery dedupes on the provider's true webhook event or
    delivery identifier. Distinct events about the same email—reply, bounce, complaint, unsubscribe or
    another lifecycle transition—are not collapsed merely because they share an email id.
11. **Recoverable projection.** Authority-to-OrgIntel projection is idempotent and durably
    reconciliable. Killing the daemon after Authority commit but before message creation eventually
    creates exactly one OrgIntel projection and one owed wake without asking the provider to resend.
    No cross-layer database write or foreign key is introduced.
12. **Exact source reference.** The organisational message retains a stable reference to the
    authoritative inbound record and enough provider/thread metadata to reopen the native source.
    OrgIntel does not become a second owner of email delivery reality or copy an unlimited raw mailbox.
13. **Untrusted-content boundary.** Sender text, HTML, links and attachments enter context as clearly
    delimited untrusted evidence. They cannot change system instructions, grant authority, choose a
    recipient, execute an attachment or weaken approval. Unsupported/oversized attachments are
    quarantined or represented by an honest inaccessible reference.

### Correlation, routing and action

14. **Nearest accountable owner.** Exact reply/thread/account metadata routes a signal to its existing
    Work owner or accountable lead. A known departmental address routes to its standing lead. Only
    unowned, conflicting or portfolio-level signals wake Exec, which appoints a lead and quiesces.
15. **Materiality without a rules engine.** Authentication, dedupe, exact correlation, obvious
    delivery-only suppression and authority boundaries are deterministic. Meaning, urgency, ambiguity,
    the best response and whether new Work is warranted remain model judgement at the nearest
    accountable level.
16. **Causal wakes.** A policy change, harmful condition, blocker, contradiction or owner/provider
    reply that can change active Work wakes the responsible actor promptly and can redirect or cancel
    the same Work revision. Routine delivery receipts, duplicate notifications and ordinary local-unit
    success may accumulate without immediate model attention.
17. **Supervised production.** When an inbound signal requires an executable response, the lead
    commissions a worker. The worker produces the draft/action package; the lead samples or inspects,
    redirects through Work if needed and makes the final judgement. No email becomes a pretext for the
    lead to produce privately.
18. **Prepared last mile.** Review opens the native thread or exact provider reference beside the
    proposed response/action, evidence, uncertainty and exact judgement requested. The owner is not
    given setup instructions or an inbox-management dashboard.
19. **Governed effects remain separate.** Drafting and internal actions are Runtime work. Sending an
    externally attributable reply crosses the existing Authority effect path with exact recipient,
    idempotency, approval/grant and receipt. Unknown send outcome is reconciled from provider state and
    is never blindly repeated.

### Real proof and deletion

20. **One real-provider `_test` run.** A real signed inbound event—not a fabricated live-company
    capability—travels through Authority, OrgIntel, one lead and one worker to a prepared result.
    A test-domain outbound reply may run only with explicit founder authorisation; otherwise the sprint
    closes at an inspectable unsent draft and proves the existing effect gate refuses the send.
21. **Fault matrix passes.** Duplicate/out-of-order delivery, restart between custody and projection,
    ACP load failure, missing MCP tools, missing semantic callback, an adversarial prompt-injection
    email, an email arriving during active Work and provider degradation each retain their declared
    truth without duplicate work or effects.
22. **Experiment scaffolding is purged or quarantined.** Production-worthy session, usage,
    cancellation and readiness behaviour moves into the owning Runtime/OrgIntel paths. The EXP-04
    harness keeps only comparison and regression support; file polling, global coalescing and empty
    reasoning-event floods cannot be mistaken for the product implementation.

## Product path

```text
provider webhook
  → verify exact signature, size and event identity
  → Authority records one external fact
  → durable idempotent projection
  → correlate provider/thread/account/Work reference
      ├─ existing responsibility → addressed actor/lead wake
      ├─ standing department → accountable lead wake
      └─ genuinely unowned/cross-company → Exec dispatch wake, then Exec quiesces
  → lead interprets and commissions/redirects Work
  → responsibility-scoped worker session resumes or reconstructs honestly
  → worker prepares exact response/action artifact
  → lead inspects native source + candidate
  → owner judgement only where needed
  → optional governed effect + provider reconciliation
```

There is no generic event workflow between these steps. Existing ownership determines the route;
provider and Runtime observations determine what actually happened.

## Layer slices and ownership

| Concern | Authoritative owner | Sprint 17 responsibility |
| --- | --- | --- |
| Provider event authenticity, custody, dedupe, effect approval and receipts | Authority Plane | Correct event identity; durable inbound custody; existing governed outbound reply and reconciliation |
| External-message projection, correlation, Work linkage, actor inbox and wakes | OrgIntel | Idempotent projection; nearest-owner delivery; causal wake and restart recovery without a signal database |
| ACP process/session lifecycle, MCP readiness, cancellation and usage | Runtime Bridge | Resume same scoped model session across replaceable processes; prove tools before prompt; preserve exact terminal observations |
| Email body, attachments, draft and native review preparation | Company Runtime | Treat inbound content as untrusted ordinary files/references; produce the prepared response/action artifact |
| Staffing, materiality, response and acceptance judgement | Exec / accountable lead intelligence | Exec appoints only where required; lead supervises workers and judges the exact outcome |
| Outcome and authority presentation | Owner cockpit | Native thread + prepared result + exact decision; no signal/agent administration dashboard |
| Harness and product evidence | Evaluation | Repair invalid probes, run failure matrix and preserve one real-provider `_test` evidence package |

## Problem classification

**Deterministic and enumerable:** signature verification, body bounds, provider event identity,
idempotent Authority custody and OrgIntel projection, exact thread/Work references, actor/session
isolation, process/MCP readiness, cumulative-to-delta usage, cancellation, terminal observation,
effect idempotency and provider reconciliation.

**Judgement and open-ended:** whether an email matters, its urgency and credibility, whether it creates
new Work, the best accountable department, how to respond, whether the response is good and when an
ambiguous signal should reach Exec or the owner.

The sprint must not use model prose to repair missing delivery/process facts or static classification
rules to replace business judgement.

## Acceptance scenarios

### A. Canon and launch contract agree

Run the focused prompt/behaviour checks after purging stale direct-lead wording. An executable owner
request produces an accountable lead plus worker-owned Work; Exec and the lead produce no artifact or
content-changing repair. A pure coordination/judgement wake may close without creating production Work.

### B. Same actor, same responsibility, replaceable process

Wake one worker three times on the same `_test` Work. Observe one ACP model-session identity across
three different process launches, exact private commitment retention without transcript injection,
new per-launch capabilities and per-turn usage deltas. Delete the session backing and observe one
explicit cold reconstruction. Attempt to load the session as another actor and another Work; both are
refused.

### C. Missing tool contract fails before production

Launch four sessions concurrently and withhold or break the coordination MCP attachment for one. The
three valid sessions may proceed; the broken session creates no productive artifact, consumes no
unattributed Work and becomes infrastructure-invalid with an exact readiness failure. Re-enable its
tool contract and resume the same responsibility without duplicate ownership.

### D. Real inbound email reaches one accountable outcome

Using a real provider and dedicated `_test` company/domain, receive one signed reply associated with a
known account or prior outbound receipt. Observe one Authority fact, one correlated OrgIntel message,
one appropriate lead wake, one worker-owned response package and one native prepared review. Exec does
not wake and no external reply is sent.

### E. Custody/projection crash and redelivery

Kill the daemon immediately after Authority commit and before OrgIntel projection. Restart and redeliver
the same provider event. Observe one Authority fact, one OrgIntel message, one owned Work unit and no
duplicate wake/effect. Deliver a distinct event for the same email id and observe that it remains
distinct.

### F. Untrusted and material mid-work email

While the worker prepares a response, deliver a signed email containing prompt-injection text and one
real policy change that invalidates the draft. The content gains no authority. The policy fact wakes
the responsible lead once, the same Work is redirected to a new revision, stale output is not sent and
the repaired package preserves attribution.

### G. Routine burst without attention spam

Deliver a bounded burst containing duplicate delivery receipts, ordinary successes, one bounce and one
customer reply. Preserve all authoritative provider facts. Suppress or coalesce non-actionable events,
while the bounce/reply reaches the responsible outcome. Measure actual lead/Exec wakes rather than
assuming every callback deserves a model turn.

### H. Optional governed test reply

Only after separate founder authorisation, send one response inside the dedicated test domain. Observe
the exact effect request, authority decision, idempotency reference, provider receipt and reconciliation.
Interrupt the result path and prove the effect becomes `unknown` rather than repeating. If external send
is not authorised, record the prepared command and the real refusal boundary instead.

## Measurements

- webhook receipt → Authority custody → OrgIntel projection → accountable ownership latency;
- time to first useful response/action package and accepted result;
- Exec occupied time and number of Exec wakes per inbound event;
- lead wakes split by material, ordinary, duplicate and recovery cause;
- duplicate Authority records, OrgIntel messages, Work units, Attempts and external effects;
- hot resumptions, cold reconstructions and session-isolation refusals;
- per-turn model input/output/cost, with unknown cache/reasoning fields preserved;
- worker throughput, backlog age and lead review time for the bounded burst;
- provider/tool/runtime failures separated from organisational failures;
- harmful-tail defects: wrong recipient, stale reply, ignored opt-out, cross-account contamination,
  external-instruction obedience and unsupported claims;
- owner interventions and prepared-last-mile quality.

Measurements establish the next experiment baseline. They do not become a universal routing score.

## Risks and dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| A real inbound event is acknowledged but never reaches the company | **Invariant** | Authority-first custody plus durable idempotent projection reconciliation |
| A duplicate callback creates duplicate Work or an external reply | **Invariant** | Provider event identity, projection idempotency and existing effect idempotency |
| Signed email content is treated as trusted instruction | **Invariant** | Signature authenticates transport only; untrusted-content boundary cannot grant authority or rewrite policy |
| One actor resumes another actor/Work session | **Invariant** | Session locator bound to company + actor + responsibility; new capability each process |
| Lead or Exec becomes the reply producer | **Invariant** | Product prompt, Work attribution and artifact checks enforce non-producing supervision |
| A projection-retry mechanism becomes a workflow engine | **Invariant** | Reconcile only Authority record → OrgIntel source reference; no stage language, general task lifecycle or provider DSL |
| Email creates a universal signal/provider ontology | **Invariant** | One Resend vertical slice; a second source must earn any extracted interface through observed reuse |
| Routine notifications create model-spend storms | **Guarded** | Exact correlation, actionability/materiality judgement and bounded causal coalescing; measure wake count |
| Routing judgement is occasionally wrong | **Accepted with recovery** | Internal routing is reversible; preserve source, allow reassignment and escalate ambiguity rather than hard-code taxonomy |
| Webhooks are delayed or missed outside Restless | **Guarded** | Provider redelivery plus a bounded reconciliation/cursor check; honest degraded state, never model polling |
| Email/attachments expose sensitive or malicious content | **Guarded** | Minimise copied payload, preserve provider references, bound/quarantine attachments and never execute them automatically |
| Provider session/cache semantics change | **Accepted** | Live capability/session probe and explicit cold reconstruction; no cache-hit claim without provider evidence |
| Q4 remains slower or unavailable | **Accepted** | This sprint proves launch correctness, not a production Q4 performance promise; topology waits for later demand evidence |

## Non-goals

- a universal Event/Signal entity, event bus, provider registry, webhook SDK or connector marketplace;
- deterministic email intent, urgency, department or team-size classification;
- a CRM, ticketing system, shared mailbox product, campaign manager or generic inbox UI;
- automatic replies to every message, autonomous public sending or weakened approval;
- executing email instructions, links or attachments as authority-bearing commands;
- storing an unlimited raw mailbox, full reasoning traces or replaying complete histories into every wake;
- a fixed effort router, universal closure reserve, batch state machine or global completion-coalescing rule;
- exactly-once internal delivery, a bespoke durable workflow engine or a second Authority writer;
- Q2/Q4/Q8 economic conclusions, cheaper-worker routing or the next organisational experiment; and
- closing unrelated Sprint 12/Sprint 16 owner/provider gates by assertion.

## Proposed ticket decomposition

Ticket files are created only after founder alignment on this sprint. Status will then live only in
this checklist.

| Status | Proposed ticket | Slice | Observed friction served | Prior machinery made deletable |
| --- | --- | --- | --- | --- |
| [x] | [**S17-T0 · Purge the lead/worker split brain**](sprint-17/S17-T0.md) | Architecture + OrgIntel + prompts | Current docs and live tests permit both non-producing and player-coach leads | Direct-lead production wording, tests and compatibility branches |
| [x] | [**S17-T1 · Resume scoped ACP sessions and prove launch readiness**](sprint-17/S17-T1.md) | Runtime Bridge + model gateway | Disposable wakes duplicate orientation; Q4 lacked coordination tools after production began | Per-wake cold-session assumption and run-level capability inference |
| [x] | [**S17-T2 · Repair experiment/runtime terminal and telemetry truth**](sprint-17/S17-T2.md) | Harness + Runtime | Direct probes leave exited Attempts running; reasoning chunks flood traces; file polling impersonates callbacks | Probe-specific terminal shortcuts, empty thought events and polled event injection |
| [x] | [**S17-T3 · Make inbound custody project exactly once**](sprint-17/S17-T3.md) | Authority + OrgIntel | Authority can accept an email that OrgIntel never receives; email id can collapse distinct events | Best-effort projection and incorrect event-identity fallback |
| [x] | [**S17-T4 · Correlate and route untrusted inbound mail**](sprint-17/S17-T4.md) | OrgIntel + Runtime | Every reply wakes Exec and loses thread/Work/account context | Exec-as-inbox projection and context-free `REPLY` messages |
| [x] | [**S17-T5 · Produce one supervised prepared response**](sprint-17/S17-T5.md) | OrgIntel + Runtime + cockpit | The inbound rail stops at a message rather than a prepared company outcome | Manual inbox handling and lead-authored response work |
| [ ] | [**S17-T6 · Run the real-provider fault matrix, purge and report**](sprint-17/S17-T6.md) — real-provider arm deferred by founder; controlled matrix and purge complete | Full slice + evaluation | Static checks cannot prove the company notices and safely acts on the world | Losing adapters, duplicate signal paths and unscoped lab mechanisms |

## Verification and evidence package

The sprint exits with:

1. an exact diff/prompt/test audit proving the Exec/lead/worker contract has one meaning;
2. a session continuity report with process ids, ACP session id, reconstruction state, capability ids,
   per-turn/cumulative usage and cross-actor/Work negative controls;
3. a session-specific MCP readiness failure before production and a successful recovery;
4. a real signed inbound `_test` email with inspectable Authority record, OrgIntel source reference,
   correlated owner, Work/Attempt and prepared native response target;
5. duplicate, distinct-same-email-id, restart-window, out-of-order, injection, attachment, mid-work
   policy-change and provider-degraded controls;
6. either one separately authorised test-domain reply with receipt/reconciliation or a proved refusal
   and prepared unsent result;
7. wake, latency, backlog, cost, duplicate and harmful-tail measurements suitable for freezing the next
   experiment; and
8. focused tests while iterating, followed by the repository checkpoint verifier, web checks, a real
   `restless doctor -c <test company>` probe and a deletion record.

## Entry, stop and exit gates

**Entry:** founders accept the non-producing lead as the one product invariant, Resend as the first
vertical ingress, and a dedicated real-provider `_test` company/domain. The current EXP-04 evidence
and existing inbound implementation remain preserved as the before-state.

**Stop:** pause for founder direction if implementation requires a universal signal/provider schema,
new workflow engine, cross-layer database writes, autonomous live-company sending, raw mailbox
retention, weaker approval, a provider-root credential in Runtime, or changing the lead back into a
producer. A real test-domain send also stops for explicit effect authorisation.

**Exit:** one real inbound signal becomes one safely owned and prepared company outcome; actor context
survives process replacement without crossing responsibility boundaries; duplicate/restart/failure
cases preserve truth; Exec remains available; and the evidence is sufficient to design—not silently
begin—the next demand-backed experiment.
