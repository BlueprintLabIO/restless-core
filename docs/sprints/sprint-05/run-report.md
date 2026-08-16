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
- A pinned, loopback-only self-hosted Infisical instance now holds the provider material behind a
  dedicated project-scoped Universal Auth identity. The daemon resolves `infisical:` references and
  launches OMP's host auth broker/gateway. Aris's live ACP process receives a gateway bearer, not the
  Kimi provider key; only Moonshot is present in the broker.
- Aris exposes one blocked review item and four exact centre approval items. No centre has authority
  and no centre send receipt exists.

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

The corrected daemon loaded `.env` without overriding inherited service configuration. Against live
Aris, `credential check` then reported `email.send = present` and the deliberately unset
`repo.push = absent`; Resend ingress listened on 7792. A real `moonshot/kimi-k3` wake ran through the
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
- Aris `email.send`, `email.inbound` and `model.inference`, plus the Cosmon and Thymelake model
  credentials, now store only `infisical:` references. The ingress reference also uses Infisical.
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

The existing OrgIntel commitment `8f3e8058-9b9a-4549-a7dc-f9c6d3605e52` is blocked rather than
completed. Its source-owned Attention item carries:

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
- `RESEND_API_KEY` is loaded by the daemon and its Aris reference probes `present`; no live send was
  attempted;
- `GITHUB_TOKEN` is absent;
- `RESEND_WEBHOOK_SECRET` is loaded by the daemon and signed ingress listens on 7792;
- there are two historical provider=`resend` receipts, both to `yaillives@gmail.com`;
- there are **zero** provider=`resend` receipts for the four centres.

The full drafts remain reviewable in
[`scratch/aris-tutoring-centre-emails.md`](../../../scratch/aris-tutoring-centre-emails.md).

## Remaining acceptance gates

These are not machine-doable without owner judgement, owner authority or elapsed time:

1. The owner runs `restless owner-token --rotate` and signs into the SPA with the newly shown
   credential. Test rotations intentionally invalidated and discarded the previous token.
2. The owner chooses the deployment mode. Same-machine use needs no new hostname; off-machine use
   supplies any trusted, remotely reachable HTTPS origin (private/VPN or public), runs Caddy durably,
   and performs the visual keyboard/pointer/scroll/clipboard pass from a second client.
3. The owner reviews and merges/rejects the two private GitHub changes. Aris independently re-probes
   production; `/for-tutoring-centres` must be 200 before its link can be sent.
4. Kimi's billing-cycle allowance refreshes or is increased before another autonomous Aris wake is
   expected to run. The configured model remains `moonshot/kimi-k3`; no GPT/OpenAI fallback exists.
5. The owner separately grants or declines each of the four canonical parties. A grant is authority,
   not an instruction to bypass the still-red production gate.
6. Only then may the stable effect keys send through `email.send`; four distinct Resend receipts are
   required if all four are granted.
7. Replies are observed for five business days. The first signal/objection advances the offer; zero
   replies becomes a dated non-response finding.

Until those seven steps occur, T3 and the Sprint 05 commercial success contract remain open.

## Final repository checks

Run after the implementation and report edits:

- `cargo fmt --check` — passed;
- `cargo clippy --workspace --all-targets -- -D warnings` — passed;
- `cargo test --workspace` — passed: 65 tests total, including 54 `restlessd` tests and the
  generated-binding drift guard;
- `cargo build --workspace` — passed;
- `npm run check` — zero errors and zero warnings;
- `npm run build` — passed with the static adapter;
- `node --check` for the browser broker and `bash -n` for all desktop entry scripts — passed;
- `git diff --check` — passed.

The final runtime reconciliation was performed through `restless up -c aris --reconcile`, not a
container command. It rebuilt/replaced the disposable container, retained the Aris volume, and
`restless doctor` then reported image reconciliation `current`, desktop/Chromium/automation/web
transport `available`, and controller `unclaimed`.

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
- All `/tmp/restless-*` scripts, screenshots and isolated Chrome profiles created by these final
  probes were deleted after their evidence was recorded.
- Aris data and its persistent browser volume were preserved.
- `web/package-lock.json` existed as an unrelated untracked user file and was not changed.
