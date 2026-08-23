# Sprint 05 run report — browser review substrate and held Aris launch

**Observed:** 15–16 August 2026
**Branch:** `dev`
**Commercial status:** **not complete** — the implementation and `_test` substrate ran, but the
owner merge, production 200, four grants/sends and five-business-day response window have not
happened.

This report separates machine-observed evidence from the remaining owner and external gates. The four
real tutoring-centre emails are present in Attention and remain unsent.

## Outcome at handoff

- The SPA reads a live source-owned attention projection rather than its old company fixture.
- Approvals, effect receipts and replay evidence now live in a narrow Authority-owned schema rather
  than OrgIntel's compactable operational event stream. Existing company rows migrate once at boot.
- The authenticated owner gateway serves the SPA, approval operations and a one-time remote-desktop
  attachment. It does not expose a shell, file API, CDP endpoint or provider-specific browser API.
- The company computer now runs supervised Openbox, TigerVNC, noVNC/websockify, visible Chromium, a
  persistent browser profile/download directory and a loopback controller broker.
- Caddy carried the full owner path over HTTPS/WSS while the owner gateway and Runtime ports remained
  private.
- `restless` now covers the deterministic company/config/credential/revocation operations named by
  T2; open-ended computer work remains behind `restless attach`.
- A pinned, loopback-only self-hosted Infisical instance now holds API-key provider material behind a
  dedicated project-scoped Universal Auth identity. The daemon resolves `infisical:` references and
  launches OMP's host auth broker/gateway. Runtime ACP processes receive a gateway bearer, not raw
  provider credentials. The active broker contains Moonshot for Aris and the explicit `_test` policy,
  plus host-held Anthropic OAuth for that `_test` policy; Aris itself remains Kimi-only.
- Aris exposes one explicit owner-review item for the rendered tutoring-centre landing page. Legacy
  approval rows whose prepared command is null remain Authority history but are not actionable inbox
  items. The four proposed centre emails remain unsent: no centre has authority and no matching send
  receipt exists.

## Desktop branch, run and purge

The detailed comparison is in [`desktop-stack-decision.md`](./desktop-stack-decision.md).

| Evidence | Selected TigerVNC/noVNC | Discarded KasmVNC |
|---|---|---|
| Runnable stack | TigerVNC 1.12.0, noVNC 1.3.0, websockify 0.10.0, Openbox 3.6.1, Chromium 151 | KasmVNC 1.5.0, Openbox 3.6.1, Chromium 151 |
| Browser available | `3.713s` from `restless up` | Built-in HTTPS answered in `0.520s` after manual server start |
| Integration observation | Existing gateway proxies private RFB and the noVNC client without a Runtime credential in the owner page | Openbox was rejected by the packaged desktop selector; manual `xstartup` ran, but the client added Basic auth and a different non-RFB proxy contract |
| Cleanup | Retained as the one canon | Exact candidate container, image and probe files deleted; it had no mount or published port |

An automated second browser client exercised keyboard, pointer, scroll and clipboard through the
actual built SPA and noVNC canvas. That is system-level input proof, not a human owner usability
verdict; the owner's own visual acceptance remains open because the in-app browser controller was
unavailable in this environment.

## `_test` runtime and persistence evidence

The main proof used `handover_fresh_test`; no simulated capability was exercised in Aris.

1. `restless up -c handover_fresh_test` started the company without an owner-authored container
   command. `restless doctor` reported desktop, Chromium, automation and web transport `available`.
2. Killing desktop, Chromium and web transport separately made the corresponding health probe
   degraded/unavailable. Supervisor restored each process without restarting the company.
3. A test tab and downloaded marker survived `restless down` / `restless up`. A server-set persistent
   cookie also survived after Chromium's on-disk flush interval. Shutdown now gives Chromium a
   30-second graceful window before container removal.
4. Older persistent volumes are reconciled safely: current browser/download directories are created
   on every boot, not only behind the original `.seeded` marker. This repaired the real Aris volume
   without deleting it.
5. Neither `handover_fresh_test` nor Aris publishes a container port. Direct host connections to
   5901, 6080, 9222 and 9223 were refused. The gateway streams via `docker exec`/`socat` into the
   container's loopback web transport.
6. In `browser_input_test`, the live SPA focus surface measured 1420×753 and the noVNC canvas
   1201.6×751 for the 1280×800 remote bitmap. Through that canvas, the client typed `OWNER-OK`,
   scrolled the page to `scrollY=120`, clicked the prepared submit button, then separately pasted
   `CLIPBOARD-OK` through noVNC clipboard. After clean hand-back, agent-side CDP read the same values
   and submitted state from the shared visible Chromium.

## Owner gateway, ticket and controller evidence

Observed against source-owned `_test` attention unless noted:

| Probe | Result |
|---|---|
| Unauthenticated attention | 401 |
| Forged `X-Principal` without owner cookie | 401 |
| Same source row in CLI and gateway | `{plane: authority, kind: approval_required, reference: 1}` |
| Valid attach ticket | 303 to a relative noVNC URL; attach cookie is HttpOnly/SameSite=Strict |
| Reused / wrong-company ticket | 401 / 401 |
| Unknown attention source | 404 |
| Stale Runtime generation | 409 |
| Ticket used after 31 seconds | 401 |
| Nested protected noVNC asset | `/core/rfb.js` returned 200 only with the owner and attach sessions |
| WebSocket transport | RFB banner `RFB 003.008` |
| Owner takeover while agent controlled | Existing agent was disconnected; a new agent control request returned 423 |
| Second owner controller | 409 while the first owner held the lease |
| Owner return | 200; controller returned to `agent`; a later agent connection returned 200 |
| Closing/returning desktop | Did not resolve the source review/approval item |
| Full canvas input | Real noVNC keyboard, pointer, scroll and clipboard reached the shared page; requester observed the changed DOM after hand-back |
| Unclean owner disconnect | After the bounded 45-second lease expiry, health returned from `owner_paused` to `available` and control returned to the requesting agent session |

Owner hand-back appends an ordinary OrgIntel wake message rather than inventing a browser workflow
state. The attempted wake then failed honestly because `MOONSHOT_API_KEY` is absent. No model was
substituted: every company remains configured as `moonshot/kimi-k3`. This was the pre-`.env` daemon
and is retained as failure evidence; the later loaded-daemon acceptance run below closes that
configuration gap.

## HTTPS/WSS ingress evidence

Caddy 2.11.4 was installed under a temporary directory for the probe and removed afterward. With
`RESTLESS_OWNER_HOST=localhost:8443`:

- HTTP returned 308 to HTTPS;
- the SPA returned HTTPS 200 with HSTS, `nosniff`, same-origin referrer policy and the bounded
  permissions policy;
- sign-in returned 200 and a `Secure` owner cookie;
- Aris returned six live attention items: one review, four centre approvals and one older unrelated
  approval; browser source health was `available`;
- the one-time desktop open returned 303 with a `Secure` attach cookie and a relative location;
- the protected nested noVNC asset returned 200;
- `wss://localhost:8443/desktop/aris/websockify` reached the private Runtime and returned
  `RFB 003.008`.

This proves the edge locally with a generated certificate. Off-machine deployment still needs an
owner-chosen, trusted HTTPS origin and a durable Caddy service. That origin may be private/VPN DNS;
the system does not inherently require a public hostname.

## Generic browser effect proof

In `handover_fresh_test`, capability `browser.form.submit` used the existing generic effect path with
a self-reported provider and arbitrary JSON outcome. The first stable key produced one receipt with
`unknown` outcome; replaying the same key returned the prior receipt and did not repeat the event.

The run also exposed a real authority defect: revoking a party after a prior receipt did not re-arm
the first-contact gate. `approval::check` now treats the latest unresolved `approval_revoked` event as
a gate until a later grant. A focused unit test covers this. A second unique effect was produced while
isolating that defect, so the test company now has two `browser.form.submit` receipts; the same-key
idempotency claim above was observed before that second, differently keyed probe.

No browser site acquired a bespoke command or provider adapter.

## Authority ownership and degraded-plane proof

The final contract audit found that `approval_required`, grants/revocations, effects, receipts and
replay evidence were still written to OrgIntel's explicitly compactable event stream while the owner
projection labelled them Authority. That violated the cross-layer ownership table and made the whole
queue return 503 when OrgIntel failed.

The daemon now has one private `restless_authority` Postgres schema containing only governance
records and per-company migration markers. It is not a new service or public API. On the first new
daemon boot:

- Aris imported 154 governance rows: 138 outbound effects, two inbound provider effects, six
  approval requests, one grant, five replay suppressions and two repeat-party advisories;
- successive daemon restarts left the Aris count at exactly 154 and migration version 2;
- `restless receipts -c aris --limit 500` returned all 138 historical receipts;
- the owner queue reconstructed five outstanding Authority asks: four centres plus the older Claire
  request;
- legacy config approvals are migration input only and are purged after Authority commits them, so
  config cannot remain a second writer.

For the failure-boundary test, `browser_input_test`'s OrgIntel actor, message and commitment tables
were temporarily renamed. The live owner projection then reported `orgintel: unavailable`,
`authority: available`, kept the exact pending item and exposed its actions. An owner grant succeeded
despite OrgIntel failure. A generic `browser.form.submit` self-reported effect then produced receipt
`ef24e407-db42-43e8-aa89-9825e41be2d6`; replaying the same key returned that ID with `replayed: true`
and left one receipt. The OrgIntel tables were restored before the `_test` company was destroyed.

A separate `attention_action_test` drove the built SPA itself: sign in, select the source item, click
**Decline**, refetch to zero items, and observe the Authority `approval_declined` record. This closes
T1's browser-action path without granting or contacting any real party.

The inbound side was exercised separately in `inbound_custody_test`. With its OrgIntel actor,
commitment and message tables unavailable, two concurrent correctly signed deliveries carrying the
same provider event ID both received 202 while the Authority unique constraint produced exactly one
`inbound_effect`. The log then named the failed OrgIntel projection without losing Authority custody.
Ingress now waits for Authority before returning 202 and schedules OrgIntel projection afterward; a
custody failure returns 503 so the provider can redeliver.

## Administrative CLI evidence

The T2 round-trip used a throwaway `control_surface_test` company:

- create from TOML, show, list and field set were available through `restless`;
- `orgintel-init`, approval grant/revoke and all five commitment states were reachable;
- the show/create config round-trip was identical;
- credential checks reported status and reference only, never a value;
- revocation re-armed the next effect approval gate;
- the static completeness guard covered every daemon owner verb and writable config field, and its
  intentional missing-verb mutation failed.

The first acceptance attempt failed because the running daemon had not inherited `MOONSHOT_API_KEY`
or `RESEND_API_KEY`. Both names were present in the ignored repository `.env`; the diagnosis was a
daemon startup/configuration gap, not missing credentials and not a reason to change model/provider.

The corrected daemon loaded `.env` without overriding inherited service configuration. At this
earlier T2 checkpoint, before T8 removed capability-named bindings, Aris reported the then-current
Resend binding `present` and a deliberately unset Git probe binding `absent`; Resend ingress listened
on 7792. A real `moonshot/kimi-k3` wake ran through the
host OMP auth broker/gateway. The broker snapshot contained only `moonshot`, even though unrelated
model-key names existed in `.env`: production import follows configured company models, never every
available variable.

During that wake, the ACP process environment exposed only `PI_CODING_AGENT_DIR`, Restless actor/
company/coordinator identity and `RESTLESS_MODEL_GATEWAY_TOKEN`. Exact-value probes found the Kimi
provider key in neither that live process, the persistent container environment nor the complete
`/company` volume. The generated Runtime config contained only the `host.docker.internal:7790`
gateway route and `transport: pi-native`; provider and Infisical credentials stayed host-side.

The host broker profile is durable, so startup also reconciles it to current company configuration.
A fake `openai` probe row was deliberately inserted, observed alongside Moonshot, then the daemon was
restarted. Boot disabled that exact unconfigured row before gateway startup; the active broker and
gateway catalogues both returned only `moonshot`, while the disabled OpenAI tombstone remained as
diagnostic evidence. Stale provider state therefore cannot become a silent fallback after restart.

The observed successful wake is OrgIntel event 244 with usage event 245: 49,866 tokens,
`$0.3686112`, 4% of the 1,048,576-token context, model exactly `moonshot/kimi-k3`. It terminated
`blocked` on the existing owner/external gates. The immediately preceding malformed-config attempt
is event 242 and has no usage event; it failed before model inference. Aris spend settled at
`$20.7406 / $30`, leaving `$9.2594`. No grant, send, merge, deployment or new effect occurred, and
the exact centre receipt count remained zero.

## Imported Infisical credential evidence

The local bootstrap source was migrated into a real self-hosted Infisical v0.162.19 deployment:

- `infra/infisical/compose.yml` imports pinned Infisical, PostgreSQL and Redis containers. Only the
  Infisical HTTP endpoint is published, at `127.0.0.1:7793`; neither data service has a host port and
  no public hostname is required.
- The provisioner created the `Restless Authority` project, `/companies/aris` and
  `/providers/moonshot` folders, and a project-only `restless-authority` Universal Auth identity.
  Its access tokens expire after one hour. Bootstrap/recovery material lives outside the checkout at
  `~/.restless/infisical/` with directory mode `0700` and file mode `0600`.
- Infisical v0.162.19's automated bootstrap returned a legacy no-expiry instance token after that
  release's compiled legacy-token cutoff. The provisioner opens a fresh-instance-only compatibility
  window, creates the scoped identity, discards the instance token, removes the compatibility setting
  and recreates only the backend. Normal operation uses the scoped short-lived token.
- The Aris Resend, inbound-webhook and model credentials, plus the Cosmon and Thymelake model
  credentials, stored only `infisical:` references. T8 later consolidated the live Aris bindings to
  `resend.production`, `github.production`, `model.inference` and `model.inference.anthropic`; the
  ingress service reference continues to use Infisical.
  Kimi, Resend and webhook values were streamed into the backend without secret argv or output, then
  their raw bootstrap lines were removed from `.env`.
- An in-memory exact-value sweep found none of the three values in `.env`, any company config, the
  Aris OrgIntel/Authority rows, receipt JSON, the Aris container configuration, accessible live
  process environments, supervisor/container logs, gateway audit logs, live SPA Attention JSON, or
  the complete `/company` volume.
- With only the Infisical backend stopped, the three authenticated capabilities became `invalid`
  with an explicit connection-refused cause. The six-item Attention queue remained readable and the
  persistent browser remained `available`. After restoration, all three references returned to
  `present`.
- OMP retains its host credential vault across restarts and represents a changed API key as another
  row. Restless now retains exactly the key resolved from the current reference, disables every
  superseded active row for that provider, and verifies convergence before starting the gateway.
  The live broker therefore has exactly one Moonshot credential and no model fallback.
- After the daemon restarted without raw provider keys, wake 247 reached Kimi through the imported
  reference. Usage event 248 records exactly `moonshot/kimi-k3`, 60,763 tokens, `$1.0219914`, and 5%
  of the 1,048,576-token context. After 54 tool calls Kimi returned its billing-cycle usage-limit 403,
  so wake-end event 249 is honestly `blocked`. The credential path worked; further autonomous model
  work is now provider-quota blocked until the Kimi allowance refreshes or is increased. No other
  model was attempted. Aris spend is now `$21.7626 / $30`, leaving `$8.2374` inside Restless's own
  budget.

No centre grant, effect or send occurred during provisioning, migration, leakage or outage probes;
the exact centre receipt count remained zero. A post-wake Authority query returned no new records,
and the only new organisational message was the explicit quota block.

## Explicit provider continuity and requester conversation

The Claude login first proved the account through an isolated OMP profile inside the persistent Aris
Runtime. It was then migrated without printing credential material into Restless's host OMP broker.
After the host proof, `omp auth-broker logout anthropic` removed only that Runtime-profile copy;
`omp usage --provider anthropic --redact` there now reports no credential. The shared Chromium login
cookies were left intact. A subsequent daemon restart reconstructed a two-row host broker—Moonshot
API key plus Anthropic OAuth—and both references in `claude_oauth_test` probed `present`.

The live `_test` wake used the exact ordered policy
`moonshot/kimi-k3 → anthropic/claude-haiku-4-5` and allowed no external effects:

| Event | Observed result |
|---|---|
| Wake 6 | Candidates recorded in owner-configured order |
| Attempt/usage 7–8 | Kimi refused on billing-cycle quota after 15,625 tokens; the refusal was telemetry, not invented spend |
| Failover 9 | One transition, `quota`, Kimi → Claude |
| Attempt/usage 10–11 | Claude completed after 28,546 tokens; billing=`subscription`, charged `$0`, estimated list cost `$0.1026895` |
| Wake end 12 | `outcome_met`, eight local tool calls, no external effect; Exec projection names Claude |

The command report named Claude as the final model and included the Kimi-to-Claude transition. The
spend projection remained unpoisoned at `$0 / $5`. The first rehearsal exposed two defects rather
than being discarded as a failed test: a new company created its milestone before its Exec actor FK,
and an unpriced Kimi quota refusal was treated as unknown charged spend. The actor is now established
before milestone creation. Classified quota/auth/model refusals remain unpriced telemetry and may
advance; unpriced transport after token use still fails closed. The corrected second run above passed.

Provider selection remains explicit company configuration. No OpenAI/GPT fallback exists, no stale
broker credential can become routable by discovery, and Aris was not changed from Kimi. The host OAuth
credential is therefore available for an owner-approved Aris fallback later without making that
product decision silently.

The owner focus surface was also changed around the actual handover boundary. The three-state-looking
interaction applies only to the deterministic controller lease. Beside the desktop, the owner gets an
ordinary free-form OrgIntel conversation with whichever durable actor requested intervention—Exec or
Staff—with the Attention recommendation, requested action, evidence and current browser/controller
context already visible. Runtime session identity and conversational actor identity are carried
separately, so returning input goes to the exact browser session while the hand-back message goes to
the durable requester. Taking or returning control never resolves the item, grants an effect or waits
for a model. Rust tests, Svelte checks and the production SPA build pass; a human visual/message pass
through this new built surface remains open. Exec already consumes directed mail in its next assembled
context. Ordinary messages to Staff are durable, but live prompt injection into an already-running
Staff ACP session has not been proved; the UI must not imply synchronous presence, and that actor-resume
proof remains part of T7's open owner-surface acceptance.

## Visible owner-feedback propagation through Staff

A second `_test` company, `aris_feedback2_test`, exercised the organisational path the commercial
work actually needs: owner feedback → Exec judgement → visible copywriter → two visible critics →
writer correction. No provider, email or live-Aris effect was configured or permitted.

The first attempt looked convincing in transcript and failed in organisational state. Exec used OMP's
private `task` subagents, while `restless people` showed only owner and Exec and no Staff commitment
owned the claimed handoffs. OMP now retains its ordinary read/shell/edit tools but its private task
tool is absent; `restless spawn` is the only Staff canon. The corrected run produced named OrgIntel
actors and commitments for each writer and critic.

The run then exposed four separate Runtime/OrgIntel defects rather than treating a generated email as
proof:

1. Two concurrently started critics shared `models.yml.tmp`; one lost the rename race and crashed.
   Agent-runtime installation now uses a process-unique temporary path.
2. Staff reused the company-wide Exec termination question, so a critic that had completed its own
   review marked itself blocked because later company work remained. Staff now judge only their
   bounded assignment; a completed review reaches `completed` even while the milestone stays open.
3. The Exec failed over from exhausted Kimi to Claude, but inherited Staff tried Kimi only. The retry
   now keeps one actor/commitment/work directory while recording Kimi attempt → quota refusal → Claude
   attempt. Inherited Staff receive the ordered company policy; an explicit role model runs first and
   retains the remaining configured fallbacks.
4. Staff recorded Claude catalogue estimates as charged spend and treated Kimi's classified unpriced
   quota refusal as unknown spend, poisoning the company. Staff now use the Exec accounting semantics:
   subscription turns emit tokens and estimates with `$0` charged, while classified unpriced provider
   refusals remain telemetry and may fail over. The live retry stayed unpoisoned at the previously
   accounted `$1.0041 / $5`.

The owner deliberately rejected false-positive work during the run. Early drafts omitted parts of the
four-part offer, used awkward phrases, retained unsupported timing language and called a questions-only
PDF proof of included answers. The final visible chain was:

| Actor | Commitment result | Durable evidence |
|---|---|---|
| `staff-email-writer-v9` | completed | `/company/outputs/centre-emails-simple-v7.md` |
| `staff-plain-english-critic-v7` | completed, copy PASS | `/company/outputs/english-critic-v7-check.md` |
| `staff-commercial-evidence-critic-v2` | completed, copy PASS / product FAIL | `/company/outputs/commercial-evidence-critic-v2-check.md` |

Independent per-email checks found all five required statements in each body: affordable, quick PDF
delivery, new papers regularly, full answers and an explicitly free sample. The four bodies contain
zero em dashes and zero prohibited timing/AI-sales phrases. The copy remains labelled **DO NOT SEND**.
The evidence critic opened the sample PDF and found questions with QR-linked online answers rather
than full answers in the PDF, confirmed the centre page still returns 404, and found no published
delivery SLA or release schedule. Copy therefore passed as the intended offer while product
send-readiness failed. The review copy is preserved at
[`experiment/aris-tutoring-centre-emails-simple.md`](../../../experiment/aris-tutoring-centre-emails-simple.md).

The run also exposed and closed an Exec wake-custody gap. Twice, an owner rejection arrived while Exec
was already inside a long wake; the message persisted, but its notification lost the in-flight race and
an explicit `restless wake` was needed. Scheduler triggers that meet an active company now coalesce into
one pending continuation. This is scheduling mechanics only: the message and work remain ordinary
durable OrgIntel state, and no workflow entity or command API was added.

The corrected live proof used the same `aris_feedback2_test` company. While wake 158 was active, the
owner sent a no-effect instruction containing `QUEUE-PROOF-20260816`. Wake 164 closed at
`04:22:59.442Z`; wake 165 began automatically at `04:23:00.101Z` with reason
`event: mail from owner (queued while exec was active)`. The new context recorded the exact token in
`/company/org/exec/journal/0013.md`. No manual wake, Staff spawn, output edit or Authority effect was
used, and `restless receipts` remained `[]`. Focused tests also prove coalescing and the case where a
manual wake wins the newly released slot. Exec wake custody is therefore guarded; live injection or
queued resume for an already-running Staff actor remains an honest T7 owner-surface gap.

## Company outcome semantics exposed by the live wake

The earlier credential-path wake completed its bounded instruction and returned the old termination
word `done` even while its report named owner merge and production gates. The daemon incorrectly
translated that turn-level word into completion of the whole Aris milestone, temporarily removing the
review item from Attention. The source commitment was restored through `restless commitment blocked`,
not a database edit.

The termination wire now says `outcome_met` and the model prompt distinguishes it from finishing one
wake: `blocked` is mandatory while a human/external gate remains. Legacy `done` is rejected rather
than completing a company outcome, and a focused regression test encodes the distinction. Wake 0019
returned `blocked` correctly; the live Attention projection again contains the Aris review plus the
five Authority asks.

## Real Aris launch state

### Review gate

The legacy OrgIntel row `8f3e8058-9b9a-4549-a7dc-f9c6d3605e52` was migrated in place to Work and is
blocked rather than completed. Its source-owned Attention item carries:

- centre-offer compare:
  `https://github.com/BlueprintLabIO/study/compare/main...feat/tutoring-centre-offer?expand=1`,
  commit `4eb334570070c12664ea5ad810eadb4b289ca4f8`;
- pricing correction compare:
  `https://github.com/BlueprintLabIO/study/compare/main...fix/pricing-doc-rate-limits?expand=1`,
  commit `4e18414ddd6e9232fbe55591ad814a1d774f9f65`;
- the production centre page, static sample and interactive booklet URLs.

Unauthenticated GitHub requests returned 404 for both compare URLs. That is not merge evidence and
cannot distinguish a private repository from a missing ref; the owner must review them in their
signed-in browser.

The independent production probes were repeated on 16 August and remained:

| URL | Observed |
|---|---|
| `https://aris-academy.com/for-tutoring-centres` | 404, `text/html` |
| `https://aris-academy.com/booklets/26-02-1.pdf` | 200, `application/pdf` |
| `https://aris-academy.com/booklet/26-02-1` | 500, `text/html` |
| `https://aris-academy.com/api/health` | 503 |
| `https://aris-academy.com/sitemap.xml` | 500 |

The centre-page gate therefore remains red. The QR paragraph was removed from all four drafts because
the interactive path is broken; the useful static PDF remains.

The deployment healthcheck currently probes only `/`, so the container can report healthy while the
database-backed booklet, API health and sitemap paths are red. That is production evidence, not a
reason to treat the root-page 200 as launch readiness.

### Four held first contacts

Each exact party has one `approval_required` event, a persisted exact draft and a stable key:

| Centre | Canonical party | Stable idempotency key | Draft body SHA-256 |
|---|---|---|---|
| BrainTree | `hello@braintreecoaching.com.au` | `aris/s05/centre-first-contact/braintree-v1` | `aedf389667dd5e835e1f380ced1084ad352c049dd39be86782a2d9c629784801` |
| Global Education Academy | `enquiries@globaleducationacademy.com.au` | `aris/s05/centre-first-contact/global-education-v1` | `67305727570e10de3e2b1120e6ba93ae90851108301bf00f54f616dd9023eb84` |
| Pre-Uni New College | `info@newcollege.com.au` | `aris/s05/centre-first-contact/pre-uni-v1` | `550b8a0146ae3c2690a7414fc21aa98d983bd4a1e9d44e54ed21d0510b2b7979` |
| Matrix Education | `info@matrix.edu.au` | `aris/s05/centre-first-contact/matrix-v1` | `94ea4e12360e7c03d115968f2c2e04b1d07c5a9e516dff1f9f0bbc4e3369c65a` |

Authority/config evidence:

- Authority has no current grant for any of the four centres;
- `resend.production` and `github.production` both reference Infisical and probe `present`; presence
  is storage evidence only, not a live tool capability claim;
- `RESEND_WEBHOOK_SECRET` is loaded by the daemon and signed ingress listens on 7792;
- pre-rebuild self-reported/simulated rows remain preserved but are labelled `legacy_unverified` and
  excluded from governed effect and money totals;
- both the legacy class and new `customer-contact.email` class have **zero** receipts for the four centres.

The full drafts remain reviewable in
[`experiment/aris-tutoring-centre-emails-simple.md`](../../../experiment/aris-tutoring-centre-emails-simple.md).

## Remaining acceptance gates

These are not machine-doable without owner judgement, owner authority or elapsed time:

1. The owner completes the remaining visual SPA pass in an available browser surface. Do not rotate
   the owner credential merely to prove this; use the existing signed-in session or an intentional owner rotation.
2. For off-machine use, the owner chooses a trusted remotely reachable HTTPS origin. Same-machine use needs no new hostname; off-machine use
   supplies any trusted, remotely reachable HTTPS origin (private/VPN or public), runs Caddy durably,
   and performs the visual keyboard/pointer/scroll/clipboard pass from a second client.
3. The owner reviews and merges/rejects the two private GitHub changes. Aris independently re-probes
   production; `/for-tutoring-centres` must be 200 before its link can be sent.
4. The owner separately grants or declines each of the four canonical parties. A grant is authority,
   not an instruction to bypass the still-red production gate.
5. Only then may the stable effect keys run the installed Resend CLI through
   `customer-contact.email`; four distinct generic receipts are
   required if all four are granted.
6. Replies are observed for five business days. The first signal/objection advances the offer; zero
   replies becomes a dated non-response finding.

Aris now has explicit `moonshot/kimi-k3` → `anthropic/claude-haiku-4-5` failover, backed by the
owner's OMP OAuth reference. No GPT/OpenAI route was added. Until the six steps above occur, T3 and
the Sprint 05 commercial success contract remain open.

## S05-T8 Work graph and generic-effect acceptance

### Canon and deterministic handover

- Migration `0006_work_graph.sql` renames the old primitive in place. A fresh live Postgres schema
  contains `work`, `work_edges`, `work_attempts`, exact artifact/feedback input tables, gates,
  schedules and owner handoffs; it contains no `commitments` table.
- The live Postgres smoke creates author and independent verifier before execution, rejects a hard
  `requires` cycle, blocks the verifier until the exact producer artifact and gate exist, sends
  `changes_requested` through `revises`, supersedes revision 1, and proves verifier Attempt 2 consumes
  only the revision-2 digest.
- Reopening OrgIntel between producer and verifier proves readiness survives supervisor restart. A
  failed Attempt then reclaims the same Work, repo/base/worktree and exact revision as Attempt 2.
- Work-linked owner and accountable-lead messages form one exact-Work conversation and leave
  `owner_judgement` pending. Only an explicit Accept or Request changes decision resolves that handoff;
  Request changes records the owner's feedback and opens the next Work revision. Identity remains an
  explicitly observed handoff. `machine_work` was rejected as an unsupported handoff category.
- Live owner message 78 exposed a missing UI receipt: Postgres stored the exact Work-linked instruction
  and the scheduler woke Exec immediately, but the SPA looked inert while the actor worked. Send now
  confirms durable delivery and automatic reply polling, names itself as discussion, and the explicit
  Request changes action can reuse that unconsumed message without duplicating the next Attempt input.
- The first live Request changes then exposed a transaction-notification race: the feedback message
  notification and the newly ready revision each started Exec, producing two real ACP sessions against
  one worktree. The duplicate rail was terminated before it made a tool call. Active Work-linked
  feedback now routes only through the deterministic Attempt input; blocked owner-review discussion
  remains a free-form lead wake.
- Direct CLI/end-of-turn Staff spawn, arbitrary model-selected wake timers, generic Work-state setting,
  provider adapters and simulator persona cloning were removed. Staff launches only from an atomically
  claimed ready Attempt.

### Generic governed process

- The official open-source `resend-cli` 2.12.0 is installed in the company image. Receipt
  `fd30c7bc-c471-44e9-8e3e-2d86e810de4d` ran `resend emails send --dry-run --json` with no secret,
  returned `dryRun: true`, and observed the declared PDF attachment at 1,920 bytes. No network send occurred.
- Private `0600` agent artifacts are copied only when explicitly declared and present as exact argv,
  into a UUID-shaped directory owned by isolated effect UID 2001. The receipt retains original paths;
  staging paths never become company truth and are removed after the child exits.
- The first real governed Git push exposed that the same `0600` default also made the persistent
  repository unreadable to effect UID 2001. Four attempts failed locally before receipt
  `3d019b9e-71fe-4356-8aff-33f3c95970a7` received GitHub's successful ref-update response. Agent
  processes now use group-private `umask 007`; a one-time runtime migration grants the isolated effect
  UID access to existing productive repos/worktrees without giving the actor process the injected
  credential or introducing a Git-specific Restless command.
- Fake-tool receipt `24d1531b-d288-42a9-80eb-d410efec6a65` proved success and replay without
  re-execution. Two execution-numbered exit-7 receipts proved bounded retry after confirmed failure;
  same-key/different-argv was refused.
- A sleeping fake effect was interrupted by a real daemon SIGINT after its durable intent. On restart,
  boot reaped the dedicated effect UID before accepting new effects, retry remained `unknown`, status
  receipt `069627c0-980d-4435-ad8d-fdb9d0879491` observed `not_applied`, and reconciled receipt
  `b1f028d4-9aa6-4c7a-a1e0-a5165428acc2` closed execution 1 as failed. A subsequent governed `find`
  returned an empty staging directory.
- Effect children are serialised because they share the isolated UID. This prevents one actor-selected
  command reading a concurrent secret-bearing child's process environment without inventing a new
  per-provider isolation system.

### Owner projection and live-company boundary

- `restless work graph` and `restless attention` returned the same repeatable-read three-node
  author → critic → publisher graph, including the critic → author `revises` cycle. The Svelte SPA
  consumes that same generated `WorkGraphSnapshot`; `pnpm check` and `pnpm build` passed. No browser
  backend was available in the final tool environment, so no new visual-browser claim is made here.
- Aris config now names `resend.production` and `github.production`; old `email.send`, `repo.push` and
  unused `email.inbound` bindings were removed through `restless company unset`. The mission teaches
  Work/Attempt handover and generic argv receipts. Kimi remains primary and the already-proven Claude
  OAuth route is the only explicit fallback.
- Exact receipt queries at the end returned zero rows for all four canonical tutoring-centre parties
  under both the legacy class and `customer-contact.email`. The four drafts remain unsent.
- `effect_sweep_test` and `work_graph_test` were destroyed after evidence capture. Seventeen inactive
  legacy simulator files were moved from `~/.restless/simulators` to the recoverable macOS Trash path
  `~/.Trash/restless-legacy-simulators-20260816`; runtime code no longer reads or clones them.

## Final repository checks

Run after the implementation and report edits:

- `cargo fmt --check` — passed;
- `cargo clippy --workspace --all-targets -- -D warnings` — passed;
- `cargo test --workspace --no-fail-fast` — passed: 59 tests total, including 48 `restlessd` tests and the
  generated-binding drift guard;
- live Postgres `restless-orgintel` smoke — passed both graph/recovery scenarios and left zero
  `smoke%` schemas;
- `cargo build --workspace` — passed;
- `pnpm check` — zero errors and zero warnings;
- `pnpm build` — passed with the static adapter;
- `node --check` for the browser broker and `bash -n` for all desktop entry scripts — passed;
- `git diff --check` — passed.

The final image reconciliation was performed through
`restless up -c reconcile_t08_test --from aris --reconcile`, not a container command. The disposable
company stripped live authority, rebuilt the current image, and `restless doctor` reported matching
source/image digests, desktop/Chromium/automation/web transport `available`, and controller
`unclaimed`. It was then destroyed. Aris remains stopped on its older persistent container; it was
deliberately not started or replaced because doing so could schedule live Work before owner review.
Its next intentional start must use `restless up -c aris --reconcile`.

## Test-only cleanup and owner data

- `ticket_stale_test` was destroyed after its stale-generation proof, including its container,
  volume, OrgIntel schema, spend spool, personas and config. It was test-only and is not recoverable.
- `control_surface_test`, `handover_test` and `handover_fresh_test` were destroyed after the final
  checks. Their test-only configs/schemas/spend state and, where present, containers/volumes/personas
  are not recoverable.
- `browser_input_test` and `attention_action_test` were destroyed after the actual canvas,
  degraded-OrgIntel and SPA-action proofs. Their containers/volumes/configs/schemas/spend and
  Authority records are not recoverable.
- `inbound_custody_test` was destroyed after its concurrent signed-redelivery and degraded-OrgIntel
  proof; its schema/config/spend and Authority record are not recoverable.
- The Kasm candidate container/image and temporary Caddy directory were removed; neither contained a
  company volume.
- `reconcile_t08_test` was destroyed after proving the source image current; its stripped test
  container, volume, config, schema and spend state are not recoverable.
- All `/tmp/restless-*` scripts, screenshots and isolated Chrome profiles created by these final
  probes were deleted after their evidence was recorded.
- Aris data and its persistent browser volume were preserved.
- `web/package-lock.json` existed as an unrelated untracked user file and was not changed.
