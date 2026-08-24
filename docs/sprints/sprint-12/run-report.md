# Sprint 12 — implementation evidence (in progress)

**Run date:** 23–24 August 2026
**Status:** implementation evidence only — the sprint checklist remains open until the natural-team
scenarios and a real Cosmon outcome are complete.

## Scope and controls

The recovery runs used four exact, disposable companies: `sprint12_test`,
`sprint12_relay_test`, `sprint12_runtime_test`, and `sprint12_orphan_test`. Each used a clone of the
Cosmon repository with its test-company authority configuration stripped. The fake `omp` binaries
were installed only inside those test Runtime containers to create bounded ACP transport failure or
orphan conditions. They did not contact a model provider and did not create an external effect.

The runs therefore prove process, artifact, attribution, recovery, and direct-message behaviour;
they do **not** prove semantic model work, natural staffing judgement, or the real-company success
criterion. The temporary fixture and fake binaries are deleted after this report is recorded.

## T1 — authoritative commission inputs

`actors_and_teams` behavioural coverage verifies the claimed Attempt records the assigned actor,
revision, input fingerprint, upstream artifacts, feedback, workspace/repository coordinates, skills,
and truthful capability locations before a Staff process starts. The context path accounts for the
facts bound automatically rather than requiring a lead to retype them.

For a lead-owned productive Attempt, the same local membrane now includes that lead's active team
charter and small roster. It presents capacity for judgement—not a Staff quota—and preserves the
rule that a real cross-actor contribution starts with bounded Work rather than a message.

## T2 — direct accountable delivery

In `sprint12_relay_test`, a recovery capsule for Work
`75563529-1a3a-4186-be35-a0e0ea3313af` and Attempt
`72d7abd7-71e7-4823-8312-156d00c2964c` was addressed to `product-direction`. The intentional ACP
transport failure then made that lead's conversation unavailable. The result was:

- the original recovery message remained in the accountable lead's inbox;
- `restless inbox -c sprint12_relay_test --as exec` returned `[]`;
- the graph remained one Work, one Attempt, and one linked artifact; and
- the only recorded model observation was the attempted lead conversation.

An earlier `sprint12_test` run exposed a defect: a failed lead turn emitted a generic message to
Exec. That is counterevidence, not a passing run. The generic relay was removed from the blocked and
error conversation paths; an explicit pending judgement may still fall through to Exec under the
pre-existing escalation contract.

## T3 — missing semantic completion and restart reconciliation

### Terminal-process recovery

`sprint12_runtime_test` exercised a real Staff Attempt through the normal Runtime bridge. While
Attempt `b8e2de01-8b25-4527-b7de-7f74b6a39490` was running, it linked the exact candidate artifact
`f1f49dcb-5de6-4b2a-abc2-0112b2940798` at
`/company/outputs/sprint12-recovery-candidate.html` (SHA-256
`e178162f...752e41a`). The controlled ACP process then exited without semantic completion.

The original Attempt became `failed`; its Work
`4abebf4d-f080-4ce8-bff1-975eb2d4ffb4` became `blocked` with the explicit reason
`productive outcome unknown`. The daemon retained the artifact, wrote one bounded terminal
observation and one recovery capsule to `product-direction`, did not message Exec, and did not start
a replacement Attempt. A normal Runtime `down`/`up` preserved the same candidate digest and the same
one Work / one Attempt / one artifact / one recovery message state.

### Supervisor restart while live

`sprint12_orphan_test` started the genuine Staff Runtime process for Attempt
`89e1b06-e52d-4d07-920e-10acc996c070` on Work
`afb49738-99e3-44b4-9850-641597dda431`; the process was observed live before daemon restart. Its
candidate artifact `ade19ade-ffee-4968-a89c-977deb577388` had the same bounded candidate digest.
On restart, supervision reaped the old process, retained the original Attempt and workspace, and
recorded:

- the original Attempt as failed, with no second Attempt;
- the Work as blocked with `productive outcome unknown` and the supervisor-loss reason;
- one terminal `semantic unreported` observation and one recovery capsule; and
- an empty Exec inbox.

No outbound effect began. `restless doctor -c sprint12_orphan_test` reported the runtime and owner
path live after reconciliation.

## Real-model probes — isolated only

The following runs used `anthropic/claude-haiku-4-5` through the already configured host broker,
within disposable `_test` companies only. No credential was copied into a company, and no external
effect was requested. Subscription billing charged `$0`; the listed dollars below are provider-list
estimates, not charges.

### B — attributed solo result

`sprint12_link_test` gave a single lead the deliberately tightly-coupled quality-direction outcome.
The lead worked alone, which is a valid natural-team decision. Its first Attempt linked the exact
declared output before reporting completion:

- Work `29412409-fd94-4db8-a023-d470c3d0f972`, owned by `quality-direction`;
- Attempt `dab6b649-ba53-49ea-bcec-3cae8daa8168`;
- artifact `1b415ece-dc77-4662-a117-8f70908f2e42` at the exact expected URI; and
- source commit `614bde5509c22b75befcc65b9e51b31030cb6dfd`.

The Exec dispatch used 29,294 tokens (estimated `$0.1342095`) and the lead used 56,066
(estimated `$0.3265441`), from `11:55:42.392646Z` to `12:01:22.860828Z`. This establishes an
attributed real-model solo result with one concrete artifact; it does not establish collaboration,
material-message delivery, or the real-Cosmon gate.

### A/C/D — useful counterevidence, not passing runs

In `sprint12_team_test`, Exec dispatched two distinct leads while it remained available, but all
four real model Attempts (the initial pair and their retries) reported completion without linking
their expected artifacts. The completion boundary correctly failed rather than accepting narration.
Those turns used 180,186 tokens (estimated `$0.9864482`, charged `$0`). A later fresh exact-path
solo Attempt above demonstrated that the added bound-context completion instruction can be followed;
it does not turn the four failures into passes.

An earlier broad `sprint12_link_test` lead chose solo work rather than inventing a specialist, which
is valid judgement. Its candidate exposed a real event-name mismatch during independent headless
inspection. A subsequent owner-to-lead conversation repaired that mismatch directly in the company
base checkout, outside any Work/Attempt boundary. The resulting 12/12, 7/7 and 29/29 harness results
are therefore **unattributed** and cannot count as Sprint 12 outcome evidence. The full series used
204,887 tokens (estimated `$1.13680875`, charged `$0`).

Before the immediate Exec execution boundary existed, `sprint12_collab_test` provided the sharper
failure: Exec directly committed `af345e578b49764d7997f5e1d4aefd806389e970` and wrote a review
file, with no Work, Attempt, or artifact reference. It used 61,338 tokens (estimated `$0.4058818`,
charged `$0`). The direct source change and prose report are invalid as an attributable company
outcome.

### Execution-boundary rerun

`sprint12_collab_v2_test` reran the broad collaboration request after moving the boundary into the
immediate Exec user prompt. Exec did not change the base checkout: it created Work
`c3a83987-868e-48a4-bfb7-e9c35eeae259` and its bound Attempt
`d99a1403-3872-49bd-a1ec-5612a32faf48`, then quiesced. That is evidence that the execution boundary
held for the dispatch.

It also surfaced two remaining failures. Despite a prepared `onboarding-curator` team lead and
`world-builder` specialist, Exec created an unassigned redundant `cosmon-curator`; no natural
lead–Staff commission or direct material message could honestly be induced from that graph. The
curator produced worktree commit `9e2e3588e17aea7e306153d401c09999fd457e02` and wrote
`/company/outputs/collab-native-review.md` (SHA-256
`84569d994dd0137440c92349c1ab8d5fbb6a5f6a10aab4374e5ee68f0becdb87`), but did not link an artifact
before declaring completion. The original Attempt consequently failed and the Work remained blocked;
Exec's later statement that it was awaiting publication conflicts with that factual state.

Independent execution of the candidate's new direct Playwright harness loaded the page but failed to
encounter a creature. That does not by itself prove the UI change wrong—the model-authored movement
harness may itself be inadequate—but it means the claimed native result is unproved. The test server
was stopped and the candidate worktree was left clean. The run used 144,300 tokens: 32,190 for
initial Exec dispatch (estimated `$0.1252606`), 69,240 for the curator (`$0.5471702`), and 42,870 for
the callback (`$0.2371747`); charged `$0` subscription.

The follow-up source change is deliberately narrow: the immediate Exec boundary now says to inspect
and reuse a standing lead whose charter covers the outcome before commissioning capacity, and to
preserve an owner-specified path or URL as the exact expected proof. It remains model judgement, not
a deterministic staffing router. Lead-owned productive context now also carries its current team
charter and roster so that this choice is made from authoritative local state rather than a role-name
guess; the new focused test verifies it excludes unrelated colleagues and sets no headcount target.

### v3 — standing-lead reuse, direct fact delivery, and a failed terminal outcome

`sprint12_collab_v3_test` was a fresh, disposable real-model run against a clean Cosmon baseline
(`dc0611c6d5de2f757ff986b4d43ef31458a6187b`). It used the pre-follow-up-fix Runtime binary and
`anthropic/claude-haiku-4-5` through the existing host broker. No external effect was requested.

There is real positive evidence at the start of the path:

- Exec reused the prepared `onboarding-curator` lead rather than creating another role, created one
  parent Work `86081b66-eefe-4a93-b864-4453718b07b8`, and ended its dispatch wake while the Work
  Attempts ran.
- The lead's bound Attempt recorded its small current team capacity. The lead naturally created one
  distinct child Work `ae4bc6a2-fc30-4761-8dca-b9f7f4d1117f` for `world-builder`; the child acquired
  its own Worktree and Attempt before it started. This is a real lead-selected Staff commission,
  rather than a fixed team-size rule.
- A linked material fact (message `3`) said that the landmark cue could not take the spotter card's
  lower guidance region during first approach. Once the lead's productive Attempt ended, its direct
  coordination wake explicitly repeated that constraint and repaired/resumed the child Work. It was
  therefore delivered to the accountable lead, not lost in an owner or daemon transcript.

The rest of the run is counterevidence, not a pass. The parent Attempt
`102f2406-e077-4ed8-b612-ce4a295038d5` made commit
`ed65637c4f1b5a3f7448c3cd779cfc63fd9eaaeb`, but reported `outcome_met` without linking its exact
expected review artifact and was blocked. The child made two source commits (`4fb2a1e…` and
`7e1f301…`) and both reported success, but both failed deterministic Work gates. The declared output
files did exist—`sprint12-collab-v3-native-review.md` (SHA-256 `88dc3b80…`),
`sprint12-collab-v3-discovery-review.md` (`786bb0e7…`), and
`sprint12-collab-v3-landmarks-review.md` (`ef9fbe82…`)—yet none was linked to the relevant Attempt.
Their existence is evidence for review, not evidence of completion.

Independent execution of the candidate's own headless harness in a read-only Runtime image started a
local server and exited `1`: after entering play, `window.__game.landmarks` was absent and every
landmark-count assertion was zero. The harness itself was model-authored, so this is not a complete
quality verdict; it is still concrete counterevidence against accepting the model's claim that the
landmark system was working.

The direct-message path is consequently only **partial** evidence for D. The fact was deferred until
the lead was free rather than producing a concurrent mid-work response. More importantly, the lead
then sent routine status to Exec (causing an Exec wake), and its coordination turns claimed source,
test, review-file, and Git changes outside a claimed productive Attempt. The preserved Runtime
snapshot contained those un-attributable changes on the base `main` checkout (`f22d31c…`,
`5039394…`, plus a dirty `js/game.js`). They are invalid as company outcome evidence. The test was
non-destructively stopped at `13:40:53Z` while a further self-triggered lead turn was still live, so
that loop could not contaminate later observations.

This run recorded 409,260 tokens and estimated `$3.2560204` across its seven completed model turns;
subscription charge remained `$0`. The final stopped turn has no trustworthy final usage snapshot, so
it is deliberately excluded from that total. The immediately following source change adds the same
explicit execution boundary to every team-lead conversation wake: it permits factual graph and
direct-message decisions, but forbids hidden repair, build/test, candidate artifact, Git commit, and
routine Exec-status work. A focused Rust test exercises that immediate prompt.

The first fresh post-v3 probe, `sprint12_boundary_v4_test`, established that prompt wording alone was
not enough. Its lead preserved an empty Git sentinel and did not wake Exec, but it attempted the
nonexistent commands `restless message list` and `restless message history`. Because an omitted
recipient means owner mail, those literal bodies created two unsolicited owner messages. The lead's
otherwise plausible final prose cannot turn that into a passing coordination turn. It used 14,636
tokens (estimated `$0.0344205`, subscription charge `$0`) and was deleted after capture.

The follow-up is deliberately narrow. A team-lead coordination session now carries
`RESTLESS_COORDINATION_WAKE=1`; the local CLI rejects `restless message` without `--to` in that
session, and the immediate prompt names both the exact owner-send hazard and the real inspection
command. This is an accidental-command guard, not a claim of per-actor filesystem isolation. An
in-image negative control invoked the guarded command directly: it exited `1` before OrgIntel, and
the test-company message count remained zero.

`sprint12_boundary_v5_test` then repeated one bounded real-model direct fact against a clean Git
sentinel with `anthropic/claude-haiku-4-5`. The lead completed after 16,650 tokens (estimated
`$0.0420422`, charged `$0` subscription), read the one addressed message, and sent one direct factual
acknowledgement to `world-builder`. Exact read-only counts were `0` Work, `0` Attempts, `0` artifact
references, `0` messages to owner, `0` messages to Exec, `0` Exec terminal wakes, and `1` lead terminal
wake; the sentinel stayed clean with no commits, receipts remained `[]`, and `restless doctor` reported
the current source/image digest `4c619246…bc3b24`. This is a narrow real-model pass for the corrected
coordination boundary. It does not substitute for D's concurrent mid-work exchange, a credited Staff
artifact, or the real Cosmon outcome.

The v3 cost record also exposed a deterministic Staff-runtime gap: unlike Exec, a live metered Staff
turn passed a permanently-false budget predicate to ACP, so one turn could continue past the company
envelope before its final usage was recorded. The bridge now gives each metered Staff provider session
the remaining company amount and stops it when ACP's cumulative charged cost reaches that amount; the
focused regression test covers charged, uncharged, and subscription cases. This is intentionally only
single-session parity with the existing Exec fuse. It does not claim a reservation system or resolve
concurrent-session oversubscription, and no metered-provider dogfood run has exercised it yet.

### Mid-work controls v6–v8 — timing and linkage counterevidence

The first direct mid-work control, `sprint12_midwork_v6_test`, kept Staff Attempt
`d5afef84-6204-4246-b48d-89bd38aa8883` live while a fixture injected one Work-linked landing-interface
fact. `product-direction` made one real direct decision to `world-builder` (16,028 tokens; estimated
`$0.0494528`, subscription charge `$0`) without owner or Exec interaction. The Staff process then
terminally reported its stale blocker because it had no discoverable current-inbox capability. That is
counterevidence against treating delivered ordinary mail as observed Work input.

`sprint12_midwork_v7_test` added a narrowly worded one-time inbox cue. Its real lead response used
`landing_zone_id` (14,426 tokens; estimated `$0.0353056`, charge `$0`) and stayed off the owner/Exec
paths, but the Staff check occurred before the reply. Attempt
`c982e653-780c-442d-8788-cece0f600ce8` then completed at `14:23:13Z` while the lead's direct reply,
recorded at `14:23:06Z`, remained unread. A single prompt-directed read is therefore not a delivery
guarantee, and the cue was removed rather than retained as a fake solution.

The following source repair preserves the immutable input boundary: direct Work-linked feedback to an
Attempt owner that lands after its input cursor supersedes a stale terminal report and leaves the Work
active for one sequential successor, where the same existing feedback becomes initial input. The
focused live-Postgres behavioural test proves the first Attempt stays immutable, the direct decision
is bound to Attempt 2, and no second process runs concurrently.

`sprint12_midwork_v8_test` then showed the remaining linkage gap in the actual lead turn. The lead
sent the useful `landing_zone_id` decision and an irrelevant second `list` message directly to
`world-builder`, with no owner or Exec mail/wake. Both were ordinary messages, not Work-linked
feedback rows; the Staff process could read them dynamically and completed Attempt
`754c7470-6fbe-4dee-888a-0d4ba479181e` (26,030 tokens; estimated `$0.0815628`, charge `$0`) without
an attributable late input or successor. The lead used 14,667 tokens (estimated `$0.03666655`,
charge `$0`). This is not a pass for the new safeguard: the exact Work identifier was absent from the
lead's message context, so the current reply command had no reliable link to preserve. The next
minimal repair renders that existing Work id and exact `restless message --work <id> --to <sender>`
form in a Work-linked coordination wake; it adds no message kind, queue, or lifecycle.

`sprint12_midwork_v9_test` is the clean real-model confirmation of that repair on Runtime/source
digest `a4c2bb5a…f2127`. Staff Attempt `4dd5b157-7287-4fa6-bd0e-9c85cfdecd38` was live before the
fixture injected Work message `1`. The lead received the exact Work id in its bounded context and
sent exactly one Work-linked direct reply (message `2`): `landing_zone_id` is canonical. The only two
messages in the company were the Staff question to its lead and that lead reply to Staff—none to the
owner or Exec. Staff's self-inbox read at `14:46:37Z` marked message `2` read and attached it to that
same live Attempt, while its immutable initial `feedback_cursor` stayed `0`. The original Attempt then
produced at `14:47:03Z` with the applied `landing_zone_id` contract; no successor Attempt, artifact,
Git/file change, receipt or external effect occurred. The lead used 12,388 tokens (estimated
`$0.0223732`, subscription charge `$0`); Staff used 16,654 (estimated `$0.0491612`, charge `$0`).
This proves the preferred live-observation branch. The focused live-Postgres behavioural test also
proves the other branch: an unconsumed late linked reply supersedes the stale Attempt and binds to one
sequential successor, rather than completing stale work or launching a concurrent process.

### Accepted Staff contribution control v10–v11

The deliberately bounded acceptance control keeps two facts separate: v3 shows a lead freely choosing and commissioning a Staff seam, while this control asks whether an attributable Staff artifact can become a later lead's immutable input and accepted contribution. It is still an isolated `_test` company—not a natural end-to-end product outcome, and not a substitute for the Cosmon gate.

`sprint12_accepted_v10_test` is counterevidence caused by a harness error, not a model miss. Real Staff Attempt `af30cddb-be2b-4909-ba52-d42c16819611` wrote and linked artifact `87caf27f-1114-4691-b858-863a27aa79b2` at the declared path. Its file and title gates passed, but the three literal Markdown-bullet gates supplied dash-prefixed patterns to `grep` without `--`. Each returned exit 2 (`grep: invalid option -- ' '`), even though the file contained the exact text. That invalid false failure woke the lead's normal recovery conversation; the lead correctly inspected the artifact and abandoned the broken Work. A read-only probe against the retained file confirmed `grep -Fxq -- <literal-bullet> <path>` succeeds. V10 was captured and destroyed rather than resumed or reclassified as a pass.

`sprint12_accepted_v11_test` repeated only that corrected deterministic check on current Runtime/source digest `4580085ab40eaf2649413ecf5e524e7fe403ae1dbf13fce82eb7c7b3c329deed`. The host fixture created the two actors, team, two Work nodes, and their `requires` edge; it did not write either artifact, link either artifact, or choose the review disposition. Real Staff Attempt `6210dc07-43d4-410d-b878-f06cdd837441` wrote and linked `/company/outputs/sprint12-v11-staff-evidence.md` (artifact `6ae0eea4-4269-4e72-81fc-3d76cbc56cb1`) and completed all five exact-file gates. Its model use was 11,708 tokens (estimated list `$0.03112`, subscription charge `$0`).

Only after that completion, lead Work `d9f420a6-defd-4287-bbd5-f6e6ce7a7da7` claimed. Its one real lead Attempt `bb4b4304-cbd9-4690-8198-5b404380a6ce` received the Staff artifact as immutable `attempt_input`, wrote and linked artifact `7c4ee46c-b46e-404c-b9e3-8ca37001e44d`, and completed all three review gates. The review says `Staff contribution: accepted` and gives a factual rationale based on the bound artifact. The lead used 12,741 tokens (estimated list `$0.03559405`, subscription charge `$0`). The two process observations recorded no repository commit or dirty workspace entries.

The final read-only checks found exactly those two produced Attempts, two linked artifacts, zero `messages`, zero owner handoffs, no Exec model Attempt, and `[]` receipts. Thus the control supplies an honest accepted-Staff-artifact observation without claiming that the host-created dependency chain was a natural commission or a real-company outcome.

## Owner-surface probe

The recovery Work route returned HTTP 200 from the local cockpit and the owner API exposed its Work,
Attempt, and artifact graph. `restless doctor -c sprint12_runtime_test` reported the owner gateway,
cockpit shell, OrgIntel, Runtime browser path, and storage available.

The Runtime advertises an unclaimed browser-automation path, but this Codex session has no attached
Browser surface. Source-level type checking and headless route/API probes therefore support this
change, but desktop/mobile visual sign-off remains open; it must be done in a connected browser before
T4 closes.

## Current implementation verification

On 24 August, after the direct Work-message receipt repair, the full local workspace command
`RESTLESS_TEST_DATABASE_URL='postgresql:///restless' cargo test --workspace --no-fail-fast --
--nocapture` passed all 145 automated tests, including the 112-test daemon suite and live-Postgres
OrgIntel scenarios. `cargo fmt --check`, `git diff --check`, `npm run check --prefix web`, and
`npm run build --prefix web` also passed.

That full run initially exposed a smoke scenario that resolved an owner handoff without modelling
delivery to its still-running Attempt. The scenario now uses the ordinary self-inbox delivery path
and asserts the reply is attached as Attempt feedback before the process finishes; it passes within
the full workspace run rather than weakening the stale-result guard.

`restless-dev sprint10b_office_test --reconcile` then restarted the host daemon and reconciled the
running company Runtime at source/image digest
`4580085ab40eaf2649413ecf5e524e7fe403ae1dbf13fce82eb7c7b3c329deed`.
`restless doctor -c sprint10b_office_test` reported the cockpit API and shell, owner gateway,
OrgIntel, browser/desktop Runtime path, and storage as available. This verifies the current local
integration path; it neither re-runs the isolated model evidence nor closes the outstanding product
gates below.

## Measurements and limitations

| Run | Model/runtime path | Outcome | Owner intervention | Limitation |
| --- | --- | --- | --- | --- |
| `sprint12_relay_test` | real bridge, fake ACP crash | direct recovery delivery, no Exec relay | none | no semantic model work |
| `sprint12_runtime_test` | real bridge, fake delayed ACP crash | linked candidate preserved across Runtime restart | none | no semantic model work |
| `sprint12_orphan_test` | real Staff process, fake hanging ACP | daemon restart reconciled the original Attempt | none | no semantic model work |
| `sprint12_link_test` | real model and Runtime | one attributed solo artifact link | none | not a collaboration or real-Cosmon result |
| `sprint12_collab_v2_test` | real model and Runtime | Exec dispatch boundary held; candidate remained failed/unattributed | none | no valid lead reuse, Staff commission, material message, or native outcome |
| `sprint12_collab_v3_test` | real model and Runtime | standing lead reuse, one Staff commission, and deferred direct-fact delivery | none | no linked artifact, failed candidate harness, Exec-status wake, and un-attributable coordination edits |
| `sprint12_boundary_v4_test` | real model and Runtime | clean sentinel and no Exec wake, but two unsolicited owner messages | none | failed coordination-boundary negative control |
| `sprint12_boundary_v5_test` | real model and Runtime | one direct factual coordination response; no Work/Attempt/artifact, owner/Exec mail, Exec wake, commit, or receipt | none | narrow boundary probe, not a mid-work or native-outcome pass |
| `sprint12_midwork_v9_test` | real model and Runtime | one Work-linked lead decision consumed by the same live Staff Attempt | none | internal delivery control; no accepted Staff artifact or real-Cosmon outcome |
| `sprint12_accepted_v11_test` | real model and Runtime | Staff artifact linked, received as lead Attempt input, and accepted in a linked lead review | none | host-created bounded dependency control, not a natural commission or real-company outcome |

Recovery overhead was one bounded terminal observation and one direct recovery capsule per Attempt.
No duplicate process, Attempt, message, or external effect was observed in the controlled cases.
Elapsed model usage is deliberately not reported for these runs because the fake ACP command did not
perform a provider inference.

## Real Cosmon outcome attempt — invalid, blocked before dispatch

Cosmon was reconciled onto the current Runtime image and live-probed successfully before the owner
directive was sent. The directive asked Exec to appoint one accountable lead for a bounded local
improvement to the existing playable `cosmon-game` repository, preserve the game loop, use real
Runtime tools, prepare a native review target, and make no external publication.

The configured real model, `moonshot/kimi-k3`, refused the first Exec wake with a provider `402`
membership-verification/quota response. As a result, this is an **invalid pre-dispatch run**:

- no lead, team, Work, Attempt, artifact, file change, spend, or external effect was created;
- the Runtime and owner API remained live; and
- it cannot be substituted with a fake ACP command or counted as the Sprint 12 real outcome.

The only required external repair is for the owner to restore the configured Moonshot membership or
configure an authorised, funded real model. After that, reissue the same bounded directive and record
the native result before closing T5.

On 24 August, the persistent Cosmon Runtime was reconciled to source/image digest
`4580085ab40eaf2649413ecf5e524e7fe403ae1dbf13fce82eb7c7b3c329deed`; its volume was retained and
`restless doctor -c cosmon` then reported every local boundary live. This was a Runtime-only repair:
no model wake was issued, so the recorded 402 remains the latest provider evidence rather than an
inference from the healthy Runtime.

## Real Cosmon outcome — passing run

The earlier Moonshot `402` is retained above as an invalid pre-dispatch counterexample. It was not
reclassified. After the configured default was changed to `zai/glm-5.3`, Cosmon ran the bounded
local Prism Caverns outcome through the real Runtime without publication, purchase, credential change,
or other consequential external effect.

Two independent departments overlapped while Exec remained available:

- Gameplay Work `4de43f07-51a1-4ecf-b4ee-8a6cdde1847a`, accountable lead
  `cosmon-gameplay`, began at `23:51:53Z`. Its first Attempt
  `ba62a515-b5c3-45dd-91c3-e29f2286d8ce` produced a clean, linked candidate after 48m 59s, but
  direct Work-linked feedback `9` arrived after its frozen input cursor. It was correctly marked
  `superseded`, not accepted or duplicated. The one sequential successor
  `9d94343e-7953-465f-bb0d-1b4b7ce700df` bound that feedback, ran from `00:40:55Z` to
  `01:26:03Z`, and completed the same Work.
- Independent read-only QA Work `6ec5d2a3-20f3-4e9e-8300-f17a1dd07b2f`, lead `cosmon-qa`, ran
  from `23:59:43Z` to `00:15:35Z` against a separate clean worktree. Its review target, baseline
  snapshot, and harness evidence are all linked to Attempt
  `3dc5e540-dc3d-441f-b8df-dcf63285555e`.
- While both Attempts were live, the third owner availability question was answered by Exec without
  turning Exec into a producer. The run therefore has two distinct leads, two isolated worktrees,
  and no Exec production Attempt.

The final gameplay successor made clean checkpoints `8e5ed5c` and `f69645e`; terminal Runtime
observation recorded zero dirty entries at commit `f69645e`. Its linked native ReviewTarget is
`/company/outputs/sprint12-cosmon-caverns-review.html`, with the playable candidate at
`/company/outputs/cosmon-game/`. The review is a standalone 1.45 MB rendered HTML evidence package.
It records these actual Chromium results, all with zero console/page/request errors:

| Gate | Observed result |
| --- | --- |
| exploration/capture/battle | 12/12 |
| two-sided combat | 7/7 |
| roster and evolution | 29/29 |
| first Prism Caverns journey | 27/27 |
| total | 75/75 |

The final review also measured the borrowed-eyes cave-wild legibility concern instead of accepting
the critic's prose: the attempt-local paired screenshot probe improved worst-case regional contrast
from `1.7` to `10.0` and mean contrast from `18.7` to `22.7`, with no measured wild worse. A separate
post-run inspection of the rendered Lumen Array and wild-encounter screenshots found the target
readable and distinct, but is not substituted for owner judgement.

Attributable metered usage through the two completed Works and the dispatch/availability wakes was
`$13.94858696`: initial gameplay `$8.36585540` / 204,519 tokens, successor `$3.77512972` /
144,565 tokens, QA `$1.02349340` / 81,418 tokens, and three Exec turns `$0.78410844` / 133,270
tokens. Owner interventions were two bounded outcome directives and one availability question; the
single material feedback fact produced exactly one successor rather than a parallel retry. No Staff
contribution is claimed for the real gameplay result: the lead correctly worked solo. The separate
`v3` commission and `v11` accepted-artifact controls remain the evidence for the natural Staff path.

One post-completion factual lead wake cost an additional `$0.143423` / 28,653 tokens. It correctly
verified that the feedback was already applied and created no Work, Attempt, effect, or owner action.
It did, however, regenerate four tracked screenshots in the already-completed worktree after the
terminal observation had recorded it clean. The committed candidate, linked output, and base checkout
remain clean; this is counterevidence against claiming that ordinary coordination inspection is
write-free. The next sprint should either keep verification output wholly outside a completed
worktree or provide an explicitly governed disposable review copy. It also found that `master` remains
four commits behind `sprint12-caverns`; future work must branch from the integrated checkpoint (or
advance master) rather than silently reintroduce the stale harness URL.

`restless doctor -c cosmon` after completion reported the current Runtime, OrgIntel, owner gateway,
cockpit, storage, embedded desktop, Chromium, automation, and web transport all available. The
embedded browser controller was unclaimed. This Codex session had no connected in-app Browser surface,
so it could not perform the ticket's connected desktop/mobile owner-cockpit review. The prepared
native target explicitly keeps real GPU, audio, mobile/touch, save persistence, and human pacing/taste
judgement as unresolved risks rather than reporting them green.

## Deletion record

The temporary `sprint12_fixture` test, fake `omp` scripts, synthetic candidate file, and the four
exact `_test` companies were removed after their observations were captured here. They were fault
harnesses rather than product machinery. The persisted implementation is limited to the OrgIntel
recovery reference/migration, Runtime observation and reconciliation path, direct accountable
delivery, and calm outcome projection.

The isolated real-model companies `sprint12_team_test`, `sprint12_link_test`,
`sprint12_collab_test`, and `sprint12_collab_v2_test` are likewise disposable evidence runs. Their
candidate repositories, browser state, local test servers, authority projections, OrgIntel state,
and spend spools are removed after the facts above are recorded; no Cosmon state is removed.

`sprint12_collab_v3_test` was stopped after its evidence snapshot while its final self-triggered
coordination turn was still live, then removed with the same exact `_test` cleanup. Its stopped
container, volume, source Worktrees, output files, and no-effect authority configuration were all
test-only; no real Cosmon repository or company state was touched.

The post-v3 `sprint12_boundary_v4_test` and `sprint12_boundary_v5_test` companies are likewise
disposable controls. Each contains only its fixture team, empty Git sentinel, bounded direct message,
and provider-usage observation; both are destroyed after this record. The v4 owner-mail leak is
preserved above as counterevidence, not retained as company state.

The later `sprint12_midwork_v6_test`, `sprint12_midwork_v7_test`, `sprint12_midwork_v8_test`, and
`sprint12_midwork_v9_test` companies are the same kind of disposable direct-message controls. Their
only state was the fixture actors/team, bounded Work/messages, empty Runtime, and subscription usage
observations recorded above. They are destroyed after capture; no Cosmon company, repository, browser
state, credential, or effect evidence is removed.

`sprint12_accepted_v10_test` and `sprint12_accepted_v11_test` were also destroyed after their exact
graph, file, message, handoff, receipt, and provider-usage observations were captured. V10's temporary
config and Runtime were removed after its gate-parser counterevidence; V11's were removed after the
accepted-contribution control. Neither company contained credentials, repository state, browser work,
external effects, or Cosmon state.

## Remaining gate

- S12-T4 remains unchecked solely for its specified connected-browser desktop and mobile owner-surface
  visual review. The real outcome and its native target are complete; this is a prepared owner
  judgement, not a simulated pass or a claim that a screenshot replaced mobile review.
