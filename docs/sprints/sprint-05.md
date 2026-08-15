# Sprint 05 — Review in the real browser, then make first centre contact

**Status:** In progress — T1, T2, T4 and T6 are complete; T5 implementation and full automated
`_test` proof are recorded. The owner visual/merge, four grants/sends and five-business-day gates
remain open. The four prepared emails are unsent.
**Date:** 16 August 2026
**Spec refs:** `owner-cockpit` §5 / §6.6 / §11.4 / §14.2,
`company-runtime` §3.1 / §4.3 / §12.1, `authority-plane` §2.2 / §7.1 / §9,
`cross-layer-contract` §3.1 / §9 / §13.1, `ARCHITECTURE.md` §5.6 / §9.6 / §16.2

---

## Outcome

> **The owner clears one real commercial launch queue from the SPA: Restless presents the prepared
> Git change and live production surface as independent evidence, hands over the same persistent
> company browser when a human must act, observes the hand-back, and—only after four explicit
> first-contact grants—Aris sends the prepared tutoring-centre samples and records the first reply,
> objection, or bounded non-response as the next revenue decision.**

### Success contract

The sprint passes when one observed run demonstrates all of the following:

1. The SPA Attention Inbox presents the Aris merge decision, production gate, and four exact
   first-contact approvals from their source-owned state; ordinary mail is not the launch queue.
2. From the selected review item, the owner can open a real, visible browser running inside the Aris
   company computer, take exclusive control, and return control without losing the profile, tabs,
   downloads, or prepared page.
3. The agent that requested the handover cannot race the owner's input. It resumes only after the
   controller lease ends and observes the same browser state; closing the browser surface is never
   treated as proof that the requested action succeeded.
4. The prepared centre-page change is reviewed and merged by the owner. Provider/external observation
   then shows the production centre page live; an agent statement or self-attested receipt alone does
   not satisfy this criterion.
5. The four emails remain unsent through build and `_test` validation. During the live run, each exact
   canonical party is separately granted or declined by the owner. Only granted parties may receive
   the prepared first-contact email.
6. At least four granted first-contact sends are accepted by the provider with distinct receipts, or
   the ticket remains honestly incomplete because the owner declined one or more sends.
7. Replies are observed for five business days. A reply or objection advances the offer; zero replies
   becomes dated channel evidence and a bounded next experiment, not narrated success.

The commercial result remains the sprint outcome. The browser and inbox are complete only when they
carry this run; a working VNC demo, an HTTP endpoint, or a rendered fixture does not complete Sprint 05.

## Why this is the next slice

Sprint 04 completed the machine-doable preparation but delivered its owner boundary as links and
instructions:

- two real compare URLs;
- four double-verified canonical tutoring-centre parties;
- four tailored drafts offering the real sample;
- a static sample PDF observed live;
- a production centre page still gated on merge;
- interactive booklet and health probes that were red at the end of the run.

That run answered the open question in the earlier Sprint 05 draft: **the SPA is now more urgent than
another CLI-only projection.** The owner needs to judge the work in the real source surface and, when
identity or a manual step is irreducible, receive the prepared browser state rather than a checklist.

The CLI remains the complete deterministic administrative control surface. `restless attach` remains
the generic terminal door into the company computer. The remote desktop is the visual form of that
same door, not a second command algebra and not a provider catalogue.

## Tickets

| ✓ | Ticket | Layer | Evidence (observed friction) | Depends |
|---|---|---|---|---|
| [x] | **S05-T1 · Unified attention projection and SPA review loop** | Owner surface over OrgIntel and Authority | The owner received merge links, a production blocker and effect grants through ordinary mail/prose; the SPA still renders fixtures and its four-kind union cannot express the observed envelope | S04-T10, S05-T4 |
| [ ] | **S05-T5 · Persistent remote browser and owner handover** | Runtime + Owner surface + Authority boundary | The company image has headless Chromium/Playwright but no visible desktop, persistent browser service, secure remote attach, controller handover or resume observation | S05-T1 |
| [x] | **S05-T6 · Imported Infisical credential backend** | Authority | The live keys existed in `.env` but the daemon did not load it, while the current `env:` resolver contradicts the spec's default backend and the workload has crossed its old deferral trigger | S05-T2 |
| [ ] | **S05-T3 · Browser-reviewed launch gate → first real centre outreach** | All | Sprint 04 ended with prepared work and no external contact; the owner still has to reconstruct merge, live health and four grants from mail | S04-T4, S05-T1, S05-T5, S05-T6 |
| [x] | **S05-T2 · The CLI is the complete administrative control surface** | Owner surface / Authority | Three companies were created by hand-writing TOML and authority withdrawal still requires an editor | S04-T10 |
| [x] | **S05-T4 · The OrgIntel → owner-surface type seam, generated** | OrgIntel | Rust and TypeScript wire rows could drift silently; the guard now fails on drift | — |

The outcome path is **T1 → T5 → T6 → T3**. T2 may proceed in parallel but supplies T6's owner
verb; T4 has already landed.

### 16 August implementation checkpoint

- **T1:** complete. The source-owned projection, authenticated gateway, SPA read/actions and
  focus-mode host passed an actual browser click-through. Authority truth was migrated out of the
  compactable OrgIntel stream; with OrgIntel tables unavailable, the queue remained readable,
  identified the failed source distinctly, accepted an owner grant and recorded/replayed a generic
  effect. Restarting the daemon imported the 154 Aris governance rows once without duplication.
- **T5:** both desktop candidates ran, TigerVNC/noVNC was selected and Kasm was purged. Persistence,
  supervisor recovery, ticket failures, controller exclusion and local HTTPS/WSS passed. An automated
  second client used the real SPA/noVNC canvas at 1280×800 to type, click, scroll and transfer
  clipboard text; the requester observed the same page after hand-back. The human owner's own visual
  acceptance pass and any chosen off-machine deployment origin are still open; a public hostname is
  not intrinsically required.
- **T2:** complete. The bounded control-surface verbs, config round-trip, revocation re-arm and
  completeness mutation guard ran. After the daemon loaded the checkout `.env`, Aris reported Resend
  `present`, the deliberately absent Git reference `absent`, ingress listened, and a real
  `moonshot/kimi-k3` wake ran without an editor, `psql`, another model or a printed credential.
- **T6:** complete. A pinned self-hosted Infisical instance now runs as loopback-only host
  infrastructure with a dedicated project and project-scoped Universal Auth identity. Kimi, Resend
  send and Resend ingress material were imported without secret argv or output; every company model
  reference and both Aris email references now resolve through `infisical:`. The raw provider values
  were removed from `.env`. Exact-value scans, a live Kimi-only credential-path wake, broker rotation
  reconciliation and a controlled Infisical outage all passed while the four centre receipt count
  stayed zero. That wake then exhausted the Kimi account's own billing-cycle quota; no other model was
  tried.
- **Live outcome correction:** a bounded wake returning the legacy word `done` incorrectly completed
  the company milestone despite named owner gates. The wire now uses `outcome_met`, rejects `done`,
  and requires `blocked` while any human/external gate remains. The source commitment was restored
  through the CLI and the next real Kimi wake returned `blocked` correctly.
- **T3:** four exact Aris approval items are queued with zero centre send receipts. The centre page is
  still 404, so merge/production/grants/sends/reply observation have not started.

Evidence and the exact remaining gates are in
[`sprint-05/run-report.md`](./sprint-05/run-report.md). Checkboxes stay open until their full ticket
acceptance holds; implementation alone does not close a ticket.

## The complete remote-browser system, bounded

“Complete” in Sprint 05 means complete for one owner, one company Runtime, one shared visible browser
profile, and one controller at a time:

```text
SPA Attention item
    │ context, recommendation, evidence, source reference
    ▼
HTTPS/WSS ingress (Caddy)
    │ TLS termination; no Runtime endpoint exposed
    ▼
Owner Gateway in restlessd
    │ authenticates owner; exchanges opaque attach ref for short-lived ticket
    ▼
remote-desktop WebSocket proxy
    │ pixels, keyboard, pointer, clipboard
    ▼
Company Runtime
    ├── supervised desktop + visible Chromium
    ├── persistent profile, tabs and downloads
    ├── loopback-only automation/CDP endpoint
    └── controller lease
          ├── agent/session controls
          └── owner controls (exclusive)
```

The owner gateway is a narrow transport boundary in the existing modular daemon. It serves the SPA's
owner projection and proxies attach traffic; it does not create a third source of truth or a generic
HTTP API for shell, Git, browser actions, files, or provider operations.

### Ownership

| Concept | Owner |
|---|---|
| Why the owner is needed, recommendation, and organisational resolution | OrgIntel |
| Approval, grant, consequential effect, and authoritative receipt | Authority Plane |
| Browser profile, page state, downloads, desktop and browser process | Company Runtime |
| Live process/session association and controller lease | Runtime Bridge; reconstructable after restart |
| Ordered owner-facing projection | SPA/BFF projection only; never authoritative |
| Short-lived attach ticket | Owner Gateway; ephemeral and not organisational state |

An attention item carries an opaque runtime attach reference containing the company, runtime
generation and requesting session identity. It never carries a VNC password, raw browser credential,
CDP endpoint, cookie, or provider secret.

### Three paths, kept distinct

| Need | Owner experience | Accounting |
|---|---|---|
| Inspect a public build, commit, PDF or URL | Open the evidence directly; no takeover required | Ordinary work; no effect receipt |
| Complete login, 2FA, CAPTCHA, identity or another prepared manual step | Open the same persistent runtime browser, take control, return control | Handover itself is not an effect |
| Cause a material external consequence in the browser | Authority is checked before execution; browser performs the prepared action | Existing generic effect receipt with arbitrary JSON outcome; no site-specific API |

The browser is not a semantic proxy for the internet. Research, reading and navigation stay ordinary
runtime work. A material consequence receives the same party, capability, idempotency and receipt
whether an HTTP adapter, browser process or owner click supplies the transport. If the external result
cannot be independently confirmed, its result remains attested or `unknown`; closing the desktop or
pressing “return control” never upgrades it to confirmed.

### Sensitive identity boundary

The shared persistent profile is suitable only for company accounts whose continued presence in the
runtime is acceptable. Provider root accounts, personal passkeys and high-impact owner identities stay
outside it. For those, the attention item opens a direct owner-browser link and preserves the runtime's
prepared context alongside it. Sprint 05 does not solve secret delegation by leaving a root login in a
profile every agent can later use.

## What the Sprint 04 run already decided

1. **Attention is genuinely a queue.** One run produced two merge links, four exact party approvals,
   a production-health blocker, an older copy verdict and five older send approvals. Ordinary mail is
   not a workable owner queue.
2. **The email-only projection is too narrow.** The first real queue must represent the merge review,
   production blocker, four exact grants and optional runtime attach without prebuilding all eight
   hypothetical categories in `owner-cockpit` §5.3.
3. **The owner needs independent evidence.** A real compare page, production URL, provider receipt and
   browser state outrank the actor's account of them.
4. **The generic runtime door held.** Shell and file inspection stay behind `restless attach`. The
   browser addition is one visual attach transport, not `restless browser-click`, `browser-type`, or a
   provider-specific RPC family.
5. **Durability means written or persistent state.** Sixty-seven ephemeral browser/tool calls vanished
   across continuations. Browser state belongs in the persistent profile; organisational conclusions
   still belong in OrgIntel or ordinary checkpoint files.
6. **The four owner questions were useful.** `people`, `spend` and `receipts` answered role, model,
   cost and output without `psql`. Keep those reads; do not redesign them here.

## Validation inside Sprint 05

Validation proceeds from checkable substrate to the real commercial run.

### 1. Component proof in a `_test` company

- Start the company through Restless, not a transcript of manual `docker` commands.
- From a second browser client, a visible Chromium session is reachable over HTTPS/WSS only through
  Caddy and the authenticated owner gateway.
- Cookies, one open tab and a downloaded marker survive Runtime restart.
- An invalid, expired or wrong-company attach ticket is rejected.
- A second owner tab is observer-only while the first holds control.

### 2. Complete handover proof in a `_test` company

An agent opens a stateful local test page, prepares everything except one human-only field, requests
attention and yields control. The owner enters the field in the SPA desktop and returns control. An
attempted agent browser action during owner control is refused or paused. After hand-back, the same
agent observes the same page state and completes the test. No person is asked to report “done”.

A separate `_test` browser-driven consequential action passes through the existing generic effect
path and produces a JSON receipt. This proves that browser transport does not require a provider API
without placing simulated evidence in Aris.

### 3. Aris review proof, still with sends held

- The live Aris inbox shows the merge review and production blocker.
- The owner opens the real compare page and production page from the item.
- Where the account is safe for the shared profile, the same remote browser may be used; where it is a
  personal/root identity, the direct owner-browser link is used instead.
- The production gate resolves only after an external 200/title probe, not when the owner closes the
  review surface.
- The four email items remain pending and no `email.send` receipt exists.

### 4. Live commercial run

Only after the owner explicitly proceeds:

- each exact party receives a separate grant or decline;
- approved bodies are rechecked against final production capabilities;
- each send uses a stable per-party idempotency key;
- provider receipts and inbound reply evidence close the loop.

## Risks and dispositions

| Risk | Disposition | Why |
|---|---|---|
| Agent and owner act in the same browser simultaneously | **Invariant** | One controller lease; owner acquisition pauses/refuses agent control until explicit release or bounded disconnect expiry |
| Remote desktop leaks a durable credential or endpoint | **Guarded** | TLS, owner authentication, opaque short-lived tickets, loopback-only desktop/CDP endpoints, no secrets in the view model |
| Model process receives the Kimi provider key | **Invariant** | Host OMP broker/gateway holds only configured providers; Runtime gets a narrow gateway bearer and a credential-free `pi-native` route. Exact-value process/volume scans are part of T6 |
| Shared profile accumulates a high-impact owner identity | **Guarded** | Restricted company accounts only; personal/root/passkey work stays in the owner's own browser. Revisit when a real handover requires profile segregation |
| Desktop disconnect is mistaken for task completion | **Invariant** | Resolution comes from the source-owned condition or effect receipt, never connection close or lease release |
| Browser stack choice becomes permanent before it works | **Guarded** | T5 runs KasmVNC and noVNC/TigerVNC candidates against the same acceptance probe, records evidence, then purges one |
| Browser system expands into per-site automation APIs | **Accepted and watched** | Browser/desktop, CDP and generic effects are sufficient for this run; a new site-specific operation needs repeated observed friction |
| Remote video is awkward on a small screen | **Accepted** | Desktop browser is the Sprint 05 target. Mobile-native interaction is explicitly deferred |
| First contact becomes spam | **Guarded** | Four hand-verified centres, one useful free sample, per-party owner authority, no sequence or automated follow-up |
| No centre replies | **Accepted** | Four sends are a channel probe. Bounded non-response is a valid commercial finding |

## Explicit non-goals

- browser fleets, parallel profiles or one browser per actor;
- provider-specific browser APIs or a remote RPC for every browser action;
- semantic inspection of every HTTP request;
- session recording, employee surveillance or token/command streaming;
- multiplayer owner presence or collaborative cursors;
- a universal artifact editor or file browser in the cockpit;
- a deploy adapter, forge API, CRM, bulk-email sequencer or custom workflow engine;
- leaving personal/root credentials in the company profile;
- replacing `restless attach` for terminal access;
- claiming all websites, passkeys or anti-bot systems work from one successful run.

## Carried constraints

- **Kimi only.** The Runtime continues to use the configured Kimi/OMP path. Sprint 05 does not add,
  substitute or silently fall back to an OpenAI model.
- **No simulated evidence in Aris.** Browser and effect simulations run only in `_test` companies.
- **One concept, one writer.** The SPA resolves back to OrgIntel or Authority; it stores no duplicate
  attention lifecycle, approval or receipt.
- **No direct database or filesystem writes from the SPA.** The owner gateway invokes the same
  application operations as the CLI and exposes runtime attach; it is not a privileged bypass.
- **The reply leg remains conditional.** Inbound email observation still requires the owner MX record
  for `reply.blueprintlab.io`; if absent, the run must report that limitation rather than invent replies.

## Founder decisions recorded by this revision

1. The SPA, not another CLI-only queue, is Sprint 05's primary review surface.
2. The inbox is the launch and return point; the desktop takes over the main canvas in focus mode.
3. The remote browser is a persistent company-computer primitive, not an Authority API.
4. Controller handover is deterministic runtime coordination; deciding why the owner is needed remains
   OrgIntel judgement.
5. Browser-driven material effects use generic effect receipts. They do not earn provider-specific
   APIs merely because the transport is a browser.
6. The four prepared emails remain unsent until the live-run grants.

## Exit evidence

The Sprint 05 run report must contain:

- the selected desktop stack and the discarded candidate, with the observed comparison;
- the `_test` handover transcript and browser-state persistence evidence;
- an SPA capture showing the real source-owned attention envelope and focus-mode desktop;
- controller-exclusion and expired-ticket failures;
- the Aris compare URL, merge commit and independent production probe;
- the four canonical party grants/declines and, when granted, four provider send receipts;
- the first reply/objection or the dated five-business-day non-response finding;
- every place the owner still left the cockpit, reached for raw logs, or had to narrate completion.
