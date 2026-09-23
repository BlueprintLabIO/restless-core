# Owner request closeout

Active objective: finish the owner's requested changes and resolve the reported issues.
This checklist records outstanding evidence; it is not a claim that the whole task is done.

## Outstanding implementation

- [x] Install Hermes, OpenClaw and arbitrary compatible ACP harnesses from Intelligence provider; make successful installations selectable for Exec and staff. Installation alone is insufficient. Preserve independent credentials, authenticate through each harness's supported flow, probe the real protocol, and exercise a selected harness through an actual Restless turn.
- [x] Repair the GitHub human-step lifecycle: shared preparation must be accessible across the agent/effect user boundary; a running sign-in prompt must reach a synced Attention/chat card before it expires; completion/expiry must be observed and reflected without requiring an owner chat message. Do not repeat the already recorded GitHub authority approval.
- [x] Finish and deploy the recorded internal-agent communications view in People, including owner-only authorization, attribution, history, pagination and usable narrow layouts.

## Prior requested behavior to audit against current runtime

- [x] Company creation opens ordinary company pages, provider first; no redundant setup wizard or name/model modal. Model-less backend and matching frontend are verified and deployed.
- [x] Intelligence provider combines direct connections and independent harness authentication, multiple simultaneous connections, preset/current models with custom input, agent assignment, clear colored status, and unavailable-provider chat CTA.
- [x] Local Infisical installation/boot, Vault metadata UI, startup Doctor, and native Codex/Claude installation and OAuth paths remain functional.
- [x] No unnecessary harness policy/company outcome dropdown; team quality target is editable and independent of thinking effort.
- [x] Chat-header intelligence hover only, stable Exec top-right location, right-aligned New focus/History, compact header, no horizontal overflow, resizable panes, streaming markdown, streaming reply, sensible auto-scroll, and larger Exec chat when Attention is confirmed All clear.
- [x] Computer visibly requires Take control and has a sufficiently visible cursor.
- [x] People names follow ordered fictional lead initials and members inherit their lead's initial; sidebar has no horizontal scroll, cluttered counts or redundant current-focus card.
- [x] Needs-you requests are discoverable in People and Attention through the same synced cards; no duplicate/manual approval needed after a grant.
- [x] Native documents open from Attention, edit/save synchronously, preserve concurrent edits, maintain automatic version history, have top-level Done and simple controls, and no redundant Discuss/manual checkpoint UI. Review tray is dismissible.
- [x] Exec work completion produces a useful unsolicited owner update when appropriate; terminal turns do not stay Working. Installed proactive-result/restart evidence and current streaming success/failure/disconnect tests cover these separate requirements.
- [x] Transport startup, JSON framing and cooldown failures are handled honestly. Large Unicode JSON framing, model-less startup/admission, independent native/direct route transitions, and exact-route cooldown/classification regressions pass.
- [x] Identity is editable; Access & limits combines resource/authority settings with obvious editors; decision/external-action history has clear semantics.
- [x] Work toolbar is only title and view switch; documents/completed history belong in sidebar and map legend belongs on canvas. Assess goal linkage and document links requested in earlier discussion against existing implementation; do not silently equate toolbar cleanup with solving data relationships.
- [x] Company bottom-left Live sources footer, divider and tooltip removed; current frontend built and checked at desktop/mobile widths.

## Current investigations

2026-09-22: GitHub login initially failed because `/company/run/github-access-dd9296ab`
was mode 0700 owned by UID 2000 while governed effects run UID 2001/GID 2000.
The live worker changed it to 0770 and created a device prompt, but no prepared identity card
was surfaced. At 04:02 UTC the login log reported `context deadline exceeded` and no `gh auth login`
process remained. Approval is recorded; authentication is not complete. A finished process
must not be described as still preparing sign-in.

Internal communications use the existing canonical direct Rooms and latest nondeleted message
revisions. The new read is bounded and does not change read/delivery state. Human conversations
are excluded, including any direct Room containing a human participant. The PostgreSQL regression
covers pagination, read history, private human conversations, unrelated agents, revisions,
deletions and nonconsumption. Browser deployment and endpoint authorization checks remain pending.

## Verified communications result

The People view is deployed locally. `web/scripts/verify-agent-exchanges.mjs` read real
Exec/lead history and verified 1440/390/320px layouts, keyboard expansion, failure/retry,
empty state and older-page loading through browser-only fixtures. No company messages or
settings were written. Full bodies render only when expanded. The transcript and composer
retain their own space when exchanges are open.

- OrgIntel `agent_exchanges_are_private_bounded_revision_aware_and_do_not_consume_mail`: passed against an isolated PostgreSQL database, removed afterward.
- Daemon `non_owner_members_may_collaborate_but_not_call_owner_mutations`: passed, including the new exchanges endpoint.
- Svelte checks: zero errors/warnings; type ramp and production build passed.
- Reference pass: Beautiful UI compact disclosure hierarchy, Cult UI restrained reveal, shadcn-svelte Collapsible keyboard behavior; existing native details and Restless tokens, no imported component runtime.

## Additional confirmed defect

The live team could not schedule its consent expiry check: `restless schedule add --at`
sent `execution_requirement=local-mac` by default, and the daemon rejected every execution
requirement as recurring-only. The fix removes that field from new one-time CLI requests
and accepts the old CLI's harmless local-mac default on the daemon. Recurring fields and
an explicit always-on requirement still require a recurring schedule. CLI and daemon
verification passed. Both fixes were built and deployed with the handoff browser correction below.

Model assignment audit (live owner API, 2026-09-22): all seven current agents resolve to
`native-codex-oauth/gpt-6-astra`, with `medium` thinking effort. `has_connections` is true.
This verifies the requested current assignment, not all provider sign-in flows.

Custom harness implementation constraints verified against official docs:
Hermes supports native stdio ACP and its `[acp]` Python extra. OpenClaw's `openclaw acp`
is a bridge to its Gateway: per-session MCP and ACP model config options are unsupported.
It needs a real Gateway-backed adapter with model selection and isolated sessions; simply
adding its executable to the current ACP allowlist would fail. Sources:
https://hermes-agent.nousresearch.com/docs/user-guide/features/acp and
https://docs.openclaw.ai/cli/acp (checked 2026-09-22).

## GitHub handoff browser correction

The reported `f0037b04-f666-40cc-be9d-41e70b99ef99` handoff explicitly withdrew the
previous code and said no replacement session was verified. Its projection nevertheless
attached any running company computer and offered “Open prepared browser”. The launcher
also deliberately set `GH_BROWSER=true`, so the existing desktop could only show its old
Claude tab; it had never navigated to GitHub.

Human-step handoffs now use their published external URL and never fall back to a generic
runtime attachment. Existing computer URLs return to the current card. The server rejects
new desktop tickets for that handoff. Review targets retain their review action.

Verified and deployed: Svelte check (zero errors/warnings), frontend and daemon builds,
external-provider action test, live projection and ticket rejection, browser-only pending
and prepared-link fixtures, and stale-URL recovery at 1440/390px. The first ticket probe
lacked the required Origin header and hit CSRF rejection; the corrected request reached
the actual attachment check and returned the expected 409. Script:
`web/scripts/verify-handoff-browser.mjs`.

No GitHub login/effect process was running before the deployment restart. Authentication
remains unverified, the same handoff remains pending, and replacement preparation/automatic
completion still needs work. This UI correction is not authentication completion.

The reported in-app browser tab was refreshed and visibly returned to the current Attention
card without the computer parameter. Doctor confirmed owner API, OrgIntel, browser, runtime
supervisor and document service connectivity. Its overall status remains degraded because
another runtime image has been built and the running container has not yet been reconciled;
container ID `7564c9182474` differs from target image `0c6fdde4cc5c`. Do not report the entire
stack green or authentication complete on the basis of the routing fix.

## Human-step recovery implementation (verification in progress)

The same GitHub Work was marked active but could not be claimed because its pending
handoff excluded it from the scheduler. Added a `preparing` state to the existing handoff
(migrations 64/65, same outstanding-request uniqueness), exposed through
`work refresh-handoff --preparing`. Repair preserves the request ID and approval; publishing
a replacement returns it to pending and blocks Work at consent again. Plain Work resume
now reports the unresolved boundary instead of creating active-but-unclaimable Work.
A produced claim cannot complete Work with an outstanding human boundary.

Scheduled checks linked to such Work previously consumed their time fact without releasing
Work or delivering a conversation wake. When Work cannot be released, the schedule now
atomically delivers an addressed observation to its actor, including the Work ID. Time itself
does not resolve the handoff. Preparing cards remain synced but do not show a Needs-you badge.

Five escalation regressions passed in an isolated PostgreSQL database, including the new
repair/publish cycle, single request/attempt, unauthorized repair, judgement/payment boundary,
scheduled observer delivery, and abandonment. The first observer test attempted to redefine
the seeded daemon actor; removing that incorrect fixture setup allowed the real scenario to run.
The database was removed. Svelte check/build passed, and the new runtime CLI's --preparing
option was executed successfully in a temporary network-disabled container, which was removed.
Final daemon deployment, schedule appliance regression, live recovery and browser verification
are still pending at this checkpoint. Custom harness install/select support remains unfinished.

## Live recovery verified

Deployed the daemon and frontend, reconciled the company to runtime image `b83168b78f08`
with CLI support for preparation and schema 65. Doctor reports `live`, every connectivity
check available, and no repair actions. The five escalation regressions and schedule appliance
regression passed, with both isolated databases removed. Browser verification passed for
Preparing labels, stale desktop rejection/recovery and normal-browser provider links at
1440/390px.

Returned the original handoff to preparation as the owner. Work automatically claimed
Attempt `d1c97a71-78b7-4a95-97d1-706eafe50ad1` at 04:54:06Z. That attempt found an additional
mechanism error: a company-supervised job has no Staff session grant, so the ordinary CLI
bridge fallback identifies Exec and rejects `RESTLESS_ACTOR=cloud-engineering`. It did not
copy session grants or bypass the rejection, and disabled its failed service. This is the
ordinary-service/actor-session distinction, not missing GitHub party approval.

The company then resumed itself into Attempt `0e024699-7e42-4897-a685-f338380afa1d` at 04:59:15Z.
A fresh native GitHub login began at 04:59:53Z, with an effect-owned gh process observed live.
The SAME handoff was refreshed to pending with its actual verification URL and code, and
its owner brief now says GitHub sign-in is ready. No duplicate approval/request was created.
The worker retained a foreground observer (its yielded tool session 19340) polling every
20 seconds through consent/expiry; no Core schedule was created for this particular attempt.
Do not claim a scheduled observer exists. Do not restart the stack during the live consent.

Opened `https://github.com/login/device` in an in-app browser tab and verified GitHub's real
sign-in form. Kept that tab for the owner. The transient code is in the canonical handoff,
not this repository. Expected expiry is approximately 05:14:53Z; GitHub's expiry field was
not captured, so this is an estimate. Authentication and exact repository/Git access remain
unverified until owner consent and the live observer's checks complete. Remaining broad goal
scope, especially selectable custom harnesses, is unchanged.

## Consent expiry observed without owner chat

The GitHub process ended at 05:15:03Z with `context deadline exceeded`. Staff's existing foreground
observer reconciled at 05:15:40Z: no matching process, no hosts configuration, no authenticated account.
It updated the SAME handoff to “GitHub sign-in timed out”, withdrew the old code and removed the
external-browser action. This was observed in the live Attention API without sending an owner message.
Existing party approval is preserved. Successful consent/repository verification is still unproven;
a new consent window needs the owner present. Do not reuse the expired code or report access verified.

## Custom harness runtime foundation

Replaced the unwired installer/probe helper with bounded process handling and Linux flock. Added
separate legacy ACP model-selection adaptation and official Hermes/OpenClaw install/setup presets.
15 Node regressions pass; no daemon/owner UI integration has been claimed or deployed for custom
harnesses. See `tools/custom-harness/README.md` for the exact remaining connection/runner requirements.

Real isolated installation/protocol checks:
- Hermes 0.21.4: installed and `acp --check` passed. Corrected managed uv path. Diagnosed an AWS
  metadata lookup blocking ACP initialization and disabled it for the local profile. Initialize now
  returns the real terminal setup method; provider setup/model access remains unconfigured.
- OpenClaw 2026.9.5: installed. Real loopback Gateway accepted the private token, and ACP discovery
  succeeded after using `--token-file`; environment-only ACP auth failed. The session returned no model
  menu, confirming that a Gateway-specific adapter is required.

Evidence: `../work/custom-harness-tests.log`, `custom-hermes-acp-install.log`,
`custom-hermes-probe-final.log`, `custom-openclaw-install.log`, `custom-openclaw-probe-final.log`.
The isolated `restless-custom-harness_test` container is retained for the remaining adapter integration;
its only running long-lived process is the test container's sleep command. It has no real company
volume, credentials or owner content. Gateway/probe processes were stopped after each check.


## Generic custom harness integration checkpoint (not deployed)

Added owner-only settings/install/setup/key/probe endpoints and a private per-company registry.
Generic ACP installations now have explicit per-agent routes, independent Infisical references,
actual model-selection validation, scoped launch identity, legacy model negotiation and initial Core
context. The UI is integrated into Intelligence provider with preset/custom installers, native setup,
Vault keys and model checks; custom names/model IDs/effort are represented accurately in the picker
and hover. OpenClaw's dedicated adapter is still outstanding; its generic discovery intentionally fails
rather than claiming a working model route.

Verified in the current worktree: 3 Rust registry/routing tests; owner/member endpoint boundary test;
16 Node transport/process/proxy tests; Svelte/type checks and frontend build; browser-only custom UI
fixtures at 1440/390/320px. Native Hermes setup opened a real visible xterm in the separate test
computer. The first UI run used the wrong Vite cwd (404); subsequent probes exposed and fixed a
missing exact select label and corrected design-token names. The mobile verifier closes the existing
Exec drawer before interacting with the underlying page, as a user would.

Evidence: `../work/custom-harness-rust-final.log`, `custom-harness-boundary-test.log`,
`custom-harness-tests.log`, `custom-harness-web-check-final.log`, `custom-harness-web-build-final.log`,
`custom-harness-ui.log`, `custom-harness-ui/`, and `custom-hermes-setup.log`.
Live owner endpoint still returns 404: these changes have NOT been deployed. The user company and
its saved agent assignments were not changed by the fixture tests. No commit/push was performed.
Full OpenClaw routing, productive end-to-end validation, model-refresh behavior and broad closeout
remain required. The test container retains its installed packages for those remaining checks.


## OpenClaw adapter checkpoint (not deployed)

Implemented OpenClaw's dedicated Gateway path and connected it to custom discovery and agent routing.
Each launch has its own Gateway, token file, session and Core context; native account state and OAuth
refresh locks remain shared only within that harness profile. Actor histories use separate native
databases. Exact model selection uses native models.list/sessions.patch and refuses mismatched model
or session acknowledgements. Core instructions enter the native system message. An explicit tool
allowlist prevents OpenClaw's own subagents, conversations and secrets tools from silently replacing
Restless coordination. Generic custom ACP still needs its system-context contract audited; its older
first-user-prompt injection must not be treated as equivalent to a native system instruction.

Observed against installed OpenClaw 2026.9.5 in the isolated test computer: discovery returned the
configured fixture model; selection confirmed the exact route; a native shell operation received the
correct test actor/capability; two provider requests produced two streamed ACP text chunks; Core context
was present in the captured system message. The deterministic provider was test-only, so this does not
prove a real account login or a full host-managed Restless turn. The first inventory check exposed
native coordination/secret tools; the final allowlist run proved they were excluded. Updated native
agent configuration from retired list/default fields to current entries after observing migration warnings.

Evidence: ../work/openclaw-acp-tool-smoke.log, ../work/custom-harness-gateway-tests.log (17 Node tests),
../work/custom-harness-gateway-rust.log (cargo check), ../work/custom-harness-gateway-rust-tests.log
(3 registry/routing tests). The test configuration and all Gateway/ACP processes
were gone after the run. The test container is removed at this checkpoint. No company assignments were
changed, and these custom-harness changes are not deployed. Remaining: full host-driven harness turns,
independent real-account setup validation, model refresh, generic system-context alignment, deployment,
and the original requirement-by-requirement audit above.

OpenClaw onboarding preset now skips a separate daemon, bootstrap persona, channels, hooks, skills
and UI using documented flags. Launches retain its configured provider plugins and extension directory;
these are needed for some native OAuth routes. The deterministic smoke did not exercise an external
provider plugin or its OAuth login; verify that path before closing the independent-auth requirement.


## Automatic custom model refresh checkpoint (not deployed)

Custom model discovery now refreshes from ordinary intelligence reads as well as Intelligence provider.
A healthy catalogue is reused for ten minutes; incomplete setup retries after thirty seconds. The host
allows one in-flight check per company/harness, with two discovery processes globally at a time. Page
reads return the current result immediately; the UI polls active work, preserves an open model selection,
and invalidates its intelligence query when new discovery arrives. Native setup invalidates the cached
catalogue. Helper deployment uses a source fingerprint so routine reads do not rewrite every helper.
A custom-runtime status error no longer rejects the entire intelligence connection list.

The real disposable-runtime regression first exposed missing test-home initialization, then a product
bug: discovery redacted ordinary environment values as if they were credential values, corrupting model
IDs. The host now supplies the actual credential names; real secrets remain redacted and ordinary model
settings survive. After those corrections, the host installed a fixture ACP process, ran exactly one
background discovery for repeated reads, changed the configured model, and published its exact new ID.
The test container and temporary registry were removed on both failure and success paths.

Verification: ../work/custom-harness-refresh-runtime.log (real Docker/host integration passed),
../work/custom-harness-refresh-rust.log (four registry/routing/cache tests passed),
../work/custom-harness-refresh-node.log (18 Node tests passed),
../work/custom-harness-refresh-web-check.log (zero Svelte errors/warnings and type ramp passed),
../work/custom-harness-refresh-web-build.log (production build passed),
../work/custom-harness-refresh-final-check.log (Rust check passed),
../work/custom-harness-refresh-ui.log and custom-harness-refresh-ui/ (automatic discovery and a newly
arriving model while the current selection stayed unchanged; desktop 1440 and mobile 390/320 layouts).
Visual pass compared Beautiful UI status rows, Cult UI restrained state continuity, and shadcn-svelte
Field labels. No external component source was copied. The preview server was stopped.

Still not deployed. Full host-managed productive harness turns, independent account flows, the generic
ACP system-instruction contract, and the broad original owner audit remain open. Hermes source was
inspected: its SessionManager builds AIAgent from native provider settings and accepts a session manager;
AIAgent exposes ephemeral_system_prompt, skip_context_files, load_soul_identity, skip_memory and
skip_background_review. Those are possible native integration points, not yet an implemented or tested
Hermes adapter. One raw source request returned HTTP 429; subsequent inspection used the public GitHub
page. Sources: NousResearch/hermes-agent/acp_adapter/session.py and run_agent.py on GitHub.

## Hermes native system-context verification

Installed Hermes 0.21.4 using the full preset in an isolated runtime. The native adapter now receives
Core instructions as system context, instead of prepending them to a user message. Its exact runtime
toolset excludes Hermes vault/memory/delegation controls; private transcript storage and automatic
session-title calls are disabled for these reconstructed Restless turns. The real selected model
produced a scoped shell result and two ACP text chunks against the local deterministic provider.
Evidence: `../work/hermes-contract-smoke.log`. Generic ACP commands must explicitly accept the
system-instruction file. Node tests, Svelte checks and three focused Rust regressions pass.

Rechecked the deployed GitHub handoff projection/browser flow: no generic desktop attachment,
stale tickets rejected, stale URLs recover, ready links target an ordinary browser tab. Authentication
remains incomplete after the expired consent window. Full custom-harness host turns, deployment
and the broad requirement audit remain outstanding; this is not a completion claim.

## Current deployment boundary (host integration follow-up)

Read-only checks on the running owner API return 200 for Intelligence, Vault and startup Doctor.
Intelligence reports seven agents and available connections; Vault reports connected. The startup
record has `setup_failed=false` and timestamp `2026-09-22T04:53:23.831651217Z`. Its image/schema report
is a historical startup observation, not current runtime health: Docker inspection confirms the
running company now uses image `b83168b78f08`. The custom-harnesses endpoint still returns 404, so
custom installation/assignment controls must not be called deployed yet.

The new real host integration scenario exposed slow terminal cleanup: the generic custom profile
root scanned all installed harnesses and forked grep for every file. Cleanup now scopes the selected
harness and uses a streaming Python scan. Boundary regressions cover a credential crossing a read
chunk, unrelated data and linked-profile preservation, and refusal to silently truncate oversized
captured state. The full host-driven native turn remains under verification; it must retain actual
coordination readiness, rather than bypassing that check.

## Current UI closeout audit (2026-09-22)

Read-only browser inspection of the live owner UI at 1440, 390 and 320px found no horizontal
overflow or browser errors on Attention, People, Work and Documents. Desktop exposes stable
top-right Exec and labeled desktop resizers for Attention/Exec, People, and Work/Exec; narrow
layouts intentionally hide those seams. The live Exec header has its identity at the left and
New focus/History at the right. Source confirms the intelligence details are hover/focus-only,
the streaming reply renders Markdown, and auto-follow stops when the reader scrolls up and
resumes at the end. No active live turn was available, so streaming and auto-scroll are not
runtime-proven.

People live data shows the requested fictional-initial convention (Alice/Aragorn/Aang,
Daria/Dobby, Elsa), grouped team rows, and the GitHub/draft Needs-you cards also visible in
Attention. Source uses the shared Attention projection rather than a manual duplicate. The
populated narrow People list was not conclusively observed because two narrow loads were
transient/incomplete; do not close that responsive evidence yet.

Work's header contains only its title and Map/Board switch; Documents and Completed are in the
goal sidebar and the map key is on the canvas. Work groups/filter rows and edges by actual
`goal_id`, but this live company currently has zero goals, so nonempty goal selection remains
unverified. Documents link directly to rooms and offer “Discuss alongside”; a document review
dependency can reference Work, but ordinary document rows and the document list do not expose a
direct goal or Work link. This records the present relation/evidence gap without assuming a
required schema design.

## Current deployment and GitHub recheck (2026-09-22)

The full ignored Hermes host integration passed (1 passed, 270.52 seconds), including a real
native install, host assignment, coordination readiness, scoped capability digest through a
shell tool, Core system context and streamed reply deltas. Its model provider is a deterministic
HTTP fixture; real account OAuth and the OpenClaw full host path remain unverified. The isolated
test database, private database URL file, container and temporary root were removed.

Deployment exposed a stale executable in the shared Cargo target. A forced targeted rebuild
was verified for the new route markers and copied atomically to `work/service/restlessd-live`.
The startup script now executes that dedicated binary. After restart, intelligence, Vault,
Attention and custom-harnesses all returned HTTP 200, and the live presets include Hermes and
OpenClaw. This supersedes the earlier 404/not-deployed observations above.

The reported GitHub handoff still reads “GitHub sign-in timed out”; authentication and repository
access remain unverified. The browser regression passed again against the deployed service:
no generic desktop attachment, stale desktop tickets rejected, old computer URLs recover to
the card, and a prepared external URL opens a browser tab (ready-link states use browser-only
fixtures). The earlier Claude view was an unrelated restored tab, not a GitHub sign-in failure
inside the browser. The expired code was not reused and no new login was started in this check.

## Bounded Company/People closeout audit (2026-09-22)

The prior intercepted browser test exposed the noncompliant pre-create name/model modal. The current
built browser fixture now passes direct model-less creation: one delayed double-click produces one
POST with the model omitted, no dialog appears, and the new unconfigured company opens
`/{id}/company/provider`. The component retains the returned company ID before navigation, so a
navigation retry cannot create a duplicate company.

Live inspection at 1440/390/320px found no horizontal overflow; Identity exposed its Add/Edit form
at each width, while the desktop Access & limits page exposed the spend-limit editor and recorded
resource/authority context. The observer-only Computer entry and Company-page selector were also
visible. `verify-company-settings.mjs` still targets
the current Identity and spend-limit controls and records an isolated `_test` company save/reload,
stale-write rejection, and the current settings routes at all three widths. Decision history and
External activity retain their explicit source semantics; this live company did not supply a
nonempty record to recheck. Computer's live entry says it is viewing-only and requires Take control;
the current same-origin desktop viewport injects the high-contrast ring cursor into noVNC. No
control request was made.

The current roster generator assigns fictional names serially and selects every team member from
the lead's initial pool; the live roster matched Alice/Aragorn/Aang and Daria/Dobby. The People
conversation index has no focus card or roster-count display. Its scroll container now hides the
horizontal axis while retaining vertical scrolling; this fixes the remaining long-label scroll
escape. This is current-source plus desktop live evidence; no owner data was changed.

## Shared attention, team target and Work closeout (2026-09-22)

`work/shared-attention-closeout.log` now exits 0. Its browser-only fixture intercepts every write:
the same scoped decision appears in People and Attention, resolving it removes both cards, a failed
grant stays visible, and the successful retry removes both without a duplicate/manual action. It
also passes People/Work/Doctor bounds at 1440, 390 and 320px, confirms Work's header has only two
view buttons and no links, and confirms Documents remains in the narrow Work resources navigation.
The Company outcome selector is absent; the current People team-quality selector remains owner-only,
saved atomically and described as separate from thinking effort in the current source and isolated
People-history evidence.

The toolbar result does not add a data relation: Work `goal_id` is optional, this company has zero
goals, and nonempty goal selection is still not observed. Ordinary documents have `linked_room_id`,
not a goal or Work link; a Work reference exists only on a document-review dependency. The requested
assessment is complete, while a future direct document-to-goal/Work design remains an explicit
product decision rather than an implied implementation task.

## Provider, Vault and Doctor audit (2026-09-22)

The live read-only Intelligence endpoint has one loaded native Codex connection and seven agents
resolving to native Codex OAuth; Vault is `connected` and exposes only the native Codex/Claude
profile references, never values. Current provider source supports separate direct and native
harness connections, saved-connection status badges, per-agent assignment, model suggestions with
models.dev/bundled fallback messaging, custom IDs, and the no-connection CTA. The isolated settings
browser test covers native sign-in failure/retry UI, Vault/Doctor read failure recovery and all
settings widths; startup code runs native-harness Doctor on creation and recovery, and the Doctor
page identifies that automatic start behavior.

These two broad rows remain open. This owner company does not demonstrate multiple simultaneously
saved direct connections, assignment changes, a populated unavailable-provider chat CTA, or a fresh
native Codex/Claude consent completion. The latter is intentionally not inferred from an existing
OAuth profile; support code and failure handling are proven, while human vendor consent was not
replayed during this read-only audit.

The browser-only custom-harness provider fixture was rerun cleanly (`work/custom-harness-provider-audit.log`,
exit 0): it exercises isolated install/discovery, an independent Vault-key save, exact model catalogue
refresh without replacing an open assignment, per-agent editor selection, retry handling, and
1440/390/320px bounds. It strengthens native/assignment support evidence but does not substitute
for a two-direct-provider fixture, so the Intelligence row remains unchecked.

## Streaming follow and document closeout verification (2026-09-22)

The expanded live-activity scroller still used unconditional scroll-to-bottom on each reply
update. It now uses the same follow action as the main transcript: follow new content, pause
when the reader scrolls up, and resume when they return to the end. The new browser regression
failed on the old deployed panel and passed after the fix. It exercises real EventSource
chunks, main transcript and nested panel scrolling, incomplete tables/code fences, stable DOM,
terminal success/failure durations, disconnect after completion, one durable final reply,
pointer resizing and narrow-page bounds. `work/streaming-follow-after.log` passes; checks have
zero errors/warnings, production build passed and the frontend was deployed without restarting
the daemon. This is browser fixture streaming, not a newly submitted owner model turn.

Document row closed based on inspected current code plus installed-stack evidence:
`work/document-direct-edit.log` proves an ordinary Exec owner-chat edit before its reply,
zero delegated Work, 22 concurrent human edits retained in a new client and after reload;
`work/auto-history-rich-smoke.log` proves actual Core/sidecar/Postgres automatic history,
continued co-editing/reload and review dismissal remaining closed through polling. Both
verify disposable teardown. Current Core still runs `local_documents::maintain_history`,
which snapshots committed bodies through OrgIntel, and the guarded editing path remains.
A fresh read-only check of the owner's actual Attention document confirmed top-level Done,
no Discuss or Save version control, usable History/Dismiss and 1440/390/320px page bounds.
No owner document edits or decisions were submitted.

Current Codex runner regression also passes its large JSON/Unicode chunk-framing test
(`work/codex-runner-closeout-tests.log`); its optional live-provider test is explicitly skipped.
Earlier proactive-result evidence remains independently recorded in collaboration-closeout.md;
this framing test does not prove every transport/cooldown or notification condition.

The All-clear expansion requirement is now browser-verified too:
`web/scripts/verify-clear-chat-layout.mjs` reads existing projections and changes Attention only
inside its browser fixture. At 1440px, pending requests retain a 380px Exec rail, a confirmed empty
queue expands it to 883px, and an unavailable source retains 380px rather than falsely claiming
All clear. No company mutations occur. Evidence: `work/clear-chat-layout.log`.

The creation audit then found a substantive mismatch: a model/name modal still preceded the
provider page, despite the owner's explicit request for immediate cockpit entry. Redirecting after
that modal is not completion. The row was reopened while genuine model-less company creation and
direct plus-to-provider navigation are implemented and browser-verified.


## Final engineering verification and deployment (2026-09-22)

The checked rows above describe implemented behavior. Older checkpoints below their headings are
historical; their pending statements are superseded by the evidence in this section. External account
consent is separate: GitHub is not authenticated, its last code expired, and no repository access is
claimed. The same handoff and recorded authority approval remain; a fresh consent window requires
the owner. This is no longer an unobserved preparing session or a route to the wrong desktop.

Creation now posts no model and immediately opens the ordinary Intelligence provider page.
Focused backend tests pass for omission parsing, persisted empty configuration, actual disposable
company/standing Exec creation, launch rejection before lease acquisition, and subsequent explicit
direct/native assignments. Native routes do not depend on unrelated direct-provider credentials.
The deployed frontend browser fixture passes one POST on repeated pending clicks, no modal, no
invented model and provider-first navigation (`work/company-creation-live-closeout.log`). Its POST is
intercepted; real provisioning/persistence is independently covered by the disposable database test.

The strengthened provider fixture (`work/provider-connections-audit.log`) passes two saved direct
connections, independent Codex/Claude authentication states, changing Exec without changing Daria,
reload persistence, and the empty-provider CTA. Unknown and failed intelligence reads wait for
hydration and do not pretend no provider exists. Model catalogue alias/filter/order/cache/fallback
validation passes (`work/model-catalog-closeout.log`). Existing custom-harness fixtures cover independent
keys, model refresh, assignment and narrow layouts. Initial local Infisical provisioning and repeated
boot are recorded in `work/infisical-provision.log` and `work/infisical-idempotence.log`; the live Vault
remains connected. Startup Doctor and native Codex/Claude sign-in support are preserved without
replaying owner consent merely for test evidence.

Hermes and OpenClaw now both pass actual native host turns with exact assigned models, Core system
context, scoped shell capability and streaming against deterministic HTTP model fixtures. The
OpenClaw test exposed a real probe-to-turn lifecycle defect: force-killing discovery's Gateway left
native startup state behind. Graceful SIGTERM with bounded SIGKILL fallback fixes it. OpenClaw host
test passes 1/1 and relevant Node tests 14/14 (`work/openclaw-host-turn.log`,
`work/openclaw-adapter-regressions.log`). Fixtures used no owner credentials. Disposable databases,
containers and temporary roots were removed. Native account setup remains the account holder's step;
protocol compatibility alone is not represented as authenticated access.

Seven focused health/gateway regressions also pass: provider prose cannot create false cooldowns;
framing errors containing status-like transcript prose remain transport errors; cooldowns apply only
to exact configured candidate routes; conversations wait only while their entire route policy cools;
explicit staff routes do not inherit Exec fallbacks. Combined with the startup/direct/native transition
and streaming terminal-state tests, these verify the reported failure boundaries without claiming
that external providers can never fail.

Deployed daemon SHA-256:
`4f32391d7d1e03e14c18fe8eeb6d5e741b000489e199b69fb9ca0d56192e4a0c`.
Its embedded OpenClaw adapter exactly matches the repaired source. The matching production frontend
was deployed while retaining prior hashed assets for open tabs. No owner agent or GitHub login was
running before restart. The service is active; Intelligence, Vault, Attention and custom-harness APIs
return 200, Vault reports connected, and the expired GitHub handoff has no desktop attachment.
No owner model assignments, documents or messages were changed by these verification fixtures.

Final post-deployment handoff browser regression also passes
(`work/handoff-browser-final-deployment.log`): current unprepared/expired projection has no desktop,
stale tickets are rejected, stale URLs recover, and a prepared verification URL opens a new tab.
An independent final source/evidence review found no remaining implementation blocker. Generic
custom setup deliberately reports only that its terminal opened; real discovery gates model
selectability. Later setup exit diagnostics could be improved, but terminal launch is never claimed
as authentication success.

## Follow-up: actionable GitHub preparation card (2026-09-22)

A new owner-requested consent session revealed another presentation gap: the stale timeout brief
remained the preparing title, and a ready URL published in requested_action did not create a button
because the daemon only inspected prepared_state. The shared frontend now clearly says there is
nothing for the owner to do while preparation runs, suppresses stale brief guidance/deadline, and
links the current human-step instructions when an older daemon omits that action. It never extracts
a link from historical evidence. The ready chat card also displays its deadline outside Details.

Frontend deployed without interrupting the active sign-in/observer. Manual inspection of the actual
owner Daria conversation confirms the current sign-in instructions, code, Open github.com button,
and deadline. Svelte check/build pass. A polling fixture had already passed before the owner asked
to prefer manual smoke verification; no further fixture expansion was performed. Backend source
action selection is being aligned separately; do not restart during active consent solely to load
that redundant server action. No GitHub authorization was submitted by this agent.

## Follow-up: company-browser link default (2026-09-22)

The owner explicitly clarified that external web links should default to Restless's company browser,
not the host/Codex browser. Shared company-layout link handling now sends ordinary external HTTP(S)
clicks to a browser-focused company-computer view. Internal links, downloads, other protocols and
modified clicks preserve native behavior; Open externally bypasses the default explicitly.
Destinations travel in POST/session state, not the app route query. Reload obtains a new observer
attachment to the existing browser without opening another tab. Take control remains explicit.

The new owner-only browser/open route validates the destination/client, opens and activates a new
Chromium tab through the private lease-aware broker, and returns a one-use observer ticket. A live
owner lease yields a conflict instead of navigating beneath that owner. Manual broker smoke preserved
a URL with multiple query parameters, activated the correct target, and cleaned up the smoke tabs.
Frontend check/build and daemon compile/build passed; no new mocked browser suite was added for
this change. Deployed daemon SHA-256:
936f23252d706cb69618af11154eb10feb0edf91292bc2c628b8166e1051e7d5.
The earlier normal-browser privacy assumption is superseded by the owner's explicit company-browser
preference; generic link navigation does not itself authorize a provider or complete a handoff.

Manual post-deployment UI smoke: clicked the real Coolify storage link in Daria's existing chat.
Restless navigated to its company-browser view, Chromium opened the exact Coolify page in a new tab,
and the embedded screen visibly showed the requested content. The header offered Open externally
and Take control, with viewing-only input. GitHub sign-in and repository read access were reported
verified in the live handoff; no login or permission approval was performed by this verification.

## Follow-up: People sidebar status icons (2026-09-22)

Conversation and People directory rows now show a Working spinner from live session state, a Reply
icon for an unanswered conversation request, or a Needs you bell for other attributed attention.
Working and attention can coexist; reply requests take precedence over generic attention. Direct
conversations resolve their actual actor, including team members. Stale data does not invent a live
working state. Idle rows remain quiet and animation respects reduced motion.

Svelte check, token lint, and production build passed. Manual inspection of the deployed Daria page
confirmed legible bell/Needs you labels for Exec, Daria, and Dobby with no sidebar overflow or pin
displacement. No agent was running during this smoke, so the working animation was not verified
using a fabricated paid conversation. The existing completed GitHub source still needs canonical
closure and is being handled by the separate completion fix.

## Follow-up: machine-interface preference and Coolify CLI (2026-09-22)

The canonical Company Operating Rules, injected into Exec and Staff prompts, now prefer exposed
MCP tools, installed CLIs, and documented APIs before UI interaction. The owner specifically requested
Coolify CLI, so Coolify has an explicit CLI-first rule with command discovery, selected-context checks,
and no invented credentials. Browser interaction remains available for authentication or unsupported
operations.

Installed official Coolify CLI 1.8.0 in the company volume at /company/home/.local/bin/coolify.
Version, command help, context help and MCP help passed as the company user. Shared agent launch
PATH now includes that durable directory so future runtime replacements do not depend on the
current container's convenience symlink. No Coolify token, deployment, or remote resources were
created. Official sources: https://coolify.io/docs/cli/installation and
https://coolify.io/docs/cli/authentication. The policy and PATH changes require the shared daemon
deployment recorded below.

## Follow-up: completed human steps and live deployment (2026-09-22)

The verified GitHub Identity handoff was owner-owned (assigned_to NULL), so staff could not resolve it,
and the shared card lacked the owner's canonical completion control. Ready human-step cards now
provide Done through an owner-only source endpoint. It validates readiness/category/ownership, records
the owner's completion and Work feedback, and resumes the linked Work. It does not grant payment,
review, or decision authority and does not infer completion from prose. Preparing steps remain gated.

Deployed daemon SHA-256: 889d96b0f3469d226e40df25eb0f29800fa407e426510b24758050c79561136b.
This binary includes the final CLI-first Coolify wording and durable agent PATH. Backend build,
frontend Svelte/type checks and production build passed, and git diff --check was clean.

Manual live smoke clicked Done on the actual GitHub card in Daria's conversation. The canonical
Attention endpoint removed handoff f0037b04-f666-40cc-be9d-41e70b99ef99, leaving only the document
collaboration. The UI count changed from 2 to 1, the shared card disappeared, and Daria/Dobby's stale
Needs you badges cleared. Dobby resumed work and its real Working spinner/label appeared, verifying
the live status indicator without a fabricated conversation. Existing conversation history remained.

## Follow-up: stable hold-to-grant hit area (2026-09-23)

HoldApprove replaced its long label with short progress text while cancelling on pointerleave.
This could shrink the button beneath the held pointer and cancel approval immediately. Labels
now share one grid cell and remain in layout while hidden, preserving the hit area. Interrupted
pointer input cancels, only the primary pointer button starts a hold, disabled state cancels an
active hold, and the release click cannot submit a form again after completion.

Svelte check and production build passed after fixing three optional-string diagnostics in
existing document routes. Deployed frontend assets only; no daemon restart or schedule changes.
Live DOM smoke confirmed the pending Coolify button retains its 180.203px width, grid layout,
and hidden progress/completion labels. No approval was granted by verification; an actual timed
hold completion was not exercised through the available browser-control interface.
