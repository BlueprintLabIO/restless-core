# EXP-11 run log

This is the append-oriented experiment record. A model or tool being configured is not evidence that
it ran. A generated screenshot is not evidence that a vision model inspected it.

## 2026-08-28 admission

- Exact requested arm `zai/glm-5.3-flash`: infrastructure-invalid. The direct Z.ai ACP catalogue did
  not advertise that exact model value, so the new exact-model selector refused to silently run a
  different model.
- OpenRouter route `openrouter/z-ai/glm-5.3-flash`: infrastructure-invalid. The model is advertised
  there, but this machine has neither the governed OpenRouter binding nor a host bootstrap key.
- `zai/glm-5v-turbo`: exact selection succeeded, then live inference returned provider code 1311:
  the current subscription does not include the model.
- Recovery vision arm `zai/glm-4.6v`: admitted. A cold/hot ACP continuity probe completed in about
  22 seconds. It is never represented as GLM-5.3 Flash.
- Production model `zai/glm-5.3`: admitted with exact session configuration selection.
- Owner amendment: after the direct Flash, GLM-5V Turbo and OpenRouter routes failed admission, the
  founder selected GPT-5.6 Sol, exact routed selector `litellm/gpt-5.6-sol`, as the replacement
  producer and vision referee through OMP's generic adapter and the locally provisioned
  `GPT_BASE_URL` / `GPT_API_KEY` route. A direct models probe advertised both
  `gpt-5.6-sol` and `gpt-5.6-terra`; an exact Sol Responses request returned `EXP11_SOL_OK` with
  non-zero usage; and a separate image-input request correctly described a fresh Swift Arrival frame.
  These are provider and modality admission evidence, not native-playtest evidence. A first smoke
  request using bundled selector `openai/gpt-5.6-sol` exposed that OMP's broker gateway ignores local
  base-URL overrides by design and reached the official OpenAI endpoint instead. A clean isolated
  broker/gateway probe then admitted `litellm/gpt-5.6-sol` through OMP's built-in generic
  OpenAI-compatible discovery path for both text and the same real image input; this is the actual
  selector used below.

## Baseline and orchestration observations

- Frozen candidate: commit `84ff1745b29267708599e94036ec6f7a2a7e0457`, tree
  `25733ead6eb7c83221048d323838a5cadc2a235e`.
- Deterministic baseline probe passed. One fresh native run passed; a separate fresh native run failed
  after dropping the crate behind the truck and becoming unable to recover it. The contradiction is
  retained as intermittent robustness evidence.
- Exec delegated once to a non-producing Game Product lead and returned available. The lead created
  one end-to-end gameplay Staff worker and five explicit gates.
- Attempt 1 failed closed because the repository had been seeded at `/company/projects/swift-arrival`
  while isolated Work resolves `/company/repos/swift-arrival`. The lead repaired the canonical link
  without owner intervention and Attempt 2 launched from the exact clean baseline.

## Harness failures discovered during the live run

1. Exact ACP model selection was previously only a launch flag; ACP session configuration could still
   choose another model. Restless now selects the exact advertised session option and verifies the
   confirmation before prompting.
2. Successful model prose was passed through the broad provider-error classifier. Ordinary words such
   as `unauthorised`, `credit`, or status-like digits could create false credential/quota cooldowns.
   Observed effects: the production company received a false quota cooldown and the corrected referee
   company received a false 24-hour credential cooldown. The classifier now requires a provider-error
   content envelope when reading assistant text; regression tests cover the observed false positives.
3. The production worker's long response ended without a relay terminal `done` charge even though
   charged usage had been recorded during the turn. The fail-closed metering poison correctly stopped
   further charged work, but left the candidate as one dirty unverified file and the Work blocked.
   This requires audited poison clearance and further relay investigation; the edit is not accepted.

## Baseline vision referee R0

- The first all-vision hierarchy (`zai/glm-4.6v` as Exec) was invalid: it repeatedly guessed CLI
  syntax instead of commissioning the playtest. It was stopped with its volume retained.
- The corrected hierarchy used GLM-5.3 for Exec and the non-producing lead, and GLM-4.6V only for one
  Staff player. Delegation and native input execution completed.
- Referee evidence was **invalid** and never counted. The Staff created screenshots but made
  no image-read tool call. It inferred success from `xdotool` and `scrot` exits, then claimed pickup,
  driving, unloading and recovery. Its own host log instead says the crate was released outside the
  destination and `delivery not completed`. The observation record therefore contains ungrounded
  visual claims. This arm does not count as an independent vision pass.
- A terminal Work message was durably owed and delivered to the lead, but the false credential
  cooldown prevented the supervisor callback from running at that point. Later arms replaced this
  evidence rather than retroactively validating it.

## Production playability run R1

- Exec delegated once and remained available. A non-producing Game Product lead commissioned one
  end-to-end gameplay worker from the exact baseline. The hot Staff session and dirty Work tree
  survived failed Attempts rather than restarting the implementation.
- The producer replaced numerical player clamps with a real `CharacterBody3D`, physical truck and
  world collision, camera-relative movement and mouse look, host validation of reported movement,
  recoverable player-relative crate drops, visible requester feedback, and explicit positive,
  negative, mechanics and recovery probes.
- Positive delivery, negative unload rejection and mechanics probes passed before the recovery path.
  Recovery repeatedly stopped at the same coordinate while walking back into the truck after a real
  drop and re-grab.
- The decisive recovery diagnosis was a representation mismatch: the visible loading-ramp mesh was
  rotated but the `CollisionShape3D` created by the position-only `_solid()` helper was not. Physics
  therefore saw a flat plate ending at the bed lip. Copying both position and rotation to the ramp
  collision produced the first green recovery run: drop behind the truck at route start, host-observed
  re-grab, physical re-entry, drive, unload and authoritative completion on host and client.
- A later clean-tree gate batch caught an edit regression that the successful live run had not: the
  debug-removal edit also removed the recovery branch's post-grab waypoint reset and timeout, causing
  an immediate too-far seat request. Staff restored the waypoint and committed `1aadbb3`. One full
  batch from that tree then passed positive, negative, mechanics and recovery modes; the recovery host
  and client both reached authoritative delivery completion.
- Native X11 input then found a separate experience gap that the direct mechanics probe could not.
  Pickup, deliberate drop, re-grab, walking, seating and driving all worked through ordinary keyboard
  events, but the first run silently failed to turn and re-seated instead of unloading. A second run
  exposed an edit mistake in the playtest driver: its explicit seat-exit input had been removed. The
  corrected run still did not turn. Temporary `_input`-level instrumentation showed that synthetic
  X11 pointer button and motion events did not reach Godot in this Runtime, while the same XTest
  keyboard path repeatedly drove authoritative gameplay. The worker chose an honest keyboard-only
  native route (back down the ramp while facing the destination) and began removing the temporary
  product instrumentation, but hit the provider allowance before it could run or commit that final
  evidence. A controller validation during the cooldown found the removal incomplete: one debug print
  remained and called a nonexistent helper, so the preserved dirty tree currently fails parsing. No
  candidate is accepted from this mid-debug state; the successor Attempt must repair and re-run gates.
- Z.ai stopped Attempt 5 with code 1308 after 257,391 tokens in the hot context. The Work tree, prior
  commits and native failure bundles survived. This was a provider five-hour allowance reset, not the
  Restless USD ceiling: production spend stood at about USD 13.90 of USD 50.

## Additional harness findings from R1

1. A 45-minute model-session capability expired during the third long Attempt. The completed tool
   call returned after expiry, so both the next reasoning call and mandatory terminal-decision call
   were rejected. This is a hidden wall-clock work timeout. Long-lived ACP work needs transparent
   capability renewal or a process-lifetime grant with revocation; prompting the model to terminate
   earlier does not repair the mechanism.
2. A Work-linked owner message addressed to the accountable lead was delivered and correctly relayed
   to the Staff Attempt, but remained unread. The lead then woke repeatedly to answer the same message.
   The likely contract mismatch is that the reply path requires the replying actor to own the Work,
   while this architecture intentionally makes the lead supervise Work owned by Staff. The controller
   consumed the two exact inbox rows with the lead's live scoped capability to stop the wake loop.
3. Sending changed Work feedback while a Staff tool call was active correctly cancelled the stale
   call, preserved the dirty tree and hot session, and started a successor Attempt with the new fact.
   This is useful event-driven repair behavior; it should not be triggered repeatedly by an unread
   message lifecycle bug.
4. A failing client probe can exit while the host waits for its full semantic timeout. The probe
   runner should cancel the sibling process on the first terminal failure so deterministic evidence
   remains event-driven and fast.
5. The quota response supplied a timezone-less reset timestamp (`2026-08-28 16:46:09`). Treating it
   as local time caused a premature retry and another provider rejection, so that parser change was
   reverted. Restless correctly retains its conservative generic cooldown when a provider timestamp
   has no trustworthy timezone; the live contradiction is preserved rather than calling the earlier
   focused parser tests evidence.
6. While that company/model cooldown was active, the scheduler retried the same owed team-lead wake
   every five seconds and logged an identical refusal. It spent no model money but is genuine local
   churn. Team-lead conversation admission now preflights the actor's whole model policy and stays
   quiet only when every exact candidate is cooling; a configured live failover is not suppressed.
   The focused policy test passes and the rebuilt live daemon stopped the five-second retry stream.
7. Fresh Sol referee arms R1-R4 were invalid before gameplay because their contracts hard-coded
   `xd://inspect_image`. Adding `inspect_image` to OMP's CLI allow-list did not mount that device:
   OMP intentionally hides the delegated inspection tool in automatic mode when the active exact
   model already accepts image input. Both corrected R3 and R4 preserved initial captures, recorded
   the missing device and stopped with no gameplay claims. The valid exact-model primitive is now
   model-native image ingestion: `read` the fresh PNG so the image block enters the active Sol turn,
   then record the pixel observation from that same turn. The harness allow-list was returned to its
   smaller form rather than forcing a second, potentially different vision route.
8. Corrected arm R5 proved the intended primitive: its exact Sol transcript contains a `read` result
   with a real `image/png` image block for the fresh focused-client capture. Parallel arm R6 stopped
   invalid after one exact-title `xdotool` activation returned non-zero even though it had already
   enumerated the client window. This exposed an over-strict referee rule: a first setup command typo
   is not a native-input capability failure. The contract now permits one bounded launch/focus/capture
   repair, preserves the failed command in the trace, and still forbids evidence substitution.
9. Arm R7 exposed a separate X11 evidence bug. It mapped the numeric client window correctly, but
   `scrot -u` captured whichever overlapping surface remained active; the resulting pixels truthfully
   said `HOST`. The referee stopped invalid instead of sending blind input. A controller-only probe in
   the retired arm proved the deterministic replacement: raise the exact client ID, obtain that ID's
   geometry, capture that screen region, and reject the capture unless its pixels identify `CLIENT`.
   Future arms use this ID-bound path; focus or command success alone no longer establishes routing.
10. Corrected arm R8 became the first valid exact-Sol clean-room playtest. It used the numeric CLIENT
    ID, exact-region capture and model-native PNG reads for every cited state. The player recovered
    from an intentional crate drop, drove to visible truck z=40 and repeatedly attempted the endpoint,
    but each plausible `E` action re-seated it; mission state remained in progress. The durable verdict
    is revise, not pass. R5 independently reached the same overlap with a physically transported crate,
    but later lost its native window after a quota/transport interruption and is retained as invalid
    supporting evidence rather than counted as a second referee.
11. OMP retried the provider's rolling-cap 429 several times inside one ACP prompt before Restless
    received a terminal error. Stopping the company to halt those retries closed the incoming
    transport, converted an otherwise useful session into blocked Work and killed its native windows.
    Capacity was raised without resetting usage, but the forced restart could not preserve the exact
    live GUI state. The enduring repair is a cancellable provider cooldown/pause that preserves the
    actor session and child processes; container teardown is not a valid pause primitive.
12. The local gateway key reached both its five-hour and daily caps at USD 15, then USD 25, across the
    production and referee arms. The authorised limits were raised to USD 40 with usage and the USD 75
    seven-day cap unchanged. A fresh exact Sol image request and exact Terra text request admitted the
    two routes before reuse. Production kept Sol primary and used Terra only as a configured fallback
    while Restless's conservative Sol cooldown remained active.
13. The production organisation independently reproduced the R8 blocker on candidate `a1d96fe` and
    returned `REVISE BEFORE FOUNDER REVIEW` despite all deterministic probes passing. The non-producing
    lead then commissioned bounded Staff Work to repair the post-seat unload route and HUD legibility;
    this product cycle arose from external referee evidence rather than an owner-written backlog.

## Candidate progression and stricter replication

14. Candidate `09fe09d30c65b811c0904508525a6ab8af02145c` completed a full intended native journey
    in R10 but failed R9 because an on-foot player could carry the parcel directly to the destination
    and receive delivery completion at route zero. A full-journey pass did not erase the negative-path
    failure.
15. Production added a host-owned loaded-journey requirement in candidate
    `7e76d1e087cf4cde12aaf1fa6d5e9309f39bcddd`. R11 recommended pass. R12 stopped before gameplay
    because CLIENT titles and captured HOST pixels contradicted one another. R13 completed the full
    journey but failed its shortcut evidence condition. R14 passed both paths after a bounded
    capability-expiry continuation.
16. R15 falsified the apparently accepted endpoint. It rejected the route-zero shortcut, then reached
    visible route 40 with a loaded truck, exited, entered the destination structure and remained at
    `TRUCK JOURNEY REQUIRED`. Natural driver-seat recovery was not available. Production abandoned the
    prior founder target rather than defending it.
17. The lead commissioned one bounded endpoint/recovery repair. Staff produced exact candidate
    `41f4fa53a2cd05ab17aea473f3d1be28979b2dcf`. Its final source Attempt passed five Work gates:
    parsing, positive loaded journey, route-zero bypass rejection, seat recovery, and combined
    movement/collision/cargo recovery. A separate production-side exact-Sol native review also passed.
18. The product lead escalated the exact candidate for founder taste only after those checks. The
    experimental controller withheld founder review pending the frozen consecutive clean-room rule.

## Final-candidate referee sequence

19. R16 kept the candidate read-only and the player repo-less. It reached the correct later shortcut
    state but could not prove the exact immediate `E` result; Work 2 was correctly withheld.
20. R17 tightened the visible precondition but still placed `E` and capture in different model-tool
    turns. Eleven seconds elapsed, longer than the 3.5-second rejection banner, so the result remained
    inconclusive and Work 2 was withheld.
21. R18 made `E`, a 0.50-second wait and screenshot one atomic native action. The direct before/after
    pixels proved explicit shortcut rejection. Late supervisor feedback superseded the otherwise
    valid Attempt, and the one-Attempt allowance then prevented the promised successor. The run was
    retained as a harness failure, not counted as a product result.
22. Attempt accounting was repaired in two places. `superseded` Attempts no longer consume bounded
    execution allowance, and an explicit `work resume` after a concrete repair grants exactly one
    attributable successor when the previous allowance is exhausted. Focused regressions passed.
23. R19 used the repaired protocol. Its first shortcut Attempt failed only because the seeded output
    directory was root-owned; the candidate remained read-only while the exact evidence root was made
    writable. An explicit resume produced the promised successor. The atomic before/after pair then
    directly proved shortcut rejection, and the independent lead accepted only that narrow result.
24. R19 Work 2 launched a fresh exact-Sol model process and fresh HOST/CLIENT state. The player directly
    observed parcel pickup, a deliberate Q-drop mistake, E recovery, loaded driving from route 0 to
    `ROUTE END REACHED — EXIT TO DELIVER`, and an on-foot exit. It then re-entered, moved again at
    route 40, exited a second time, entered the visible destination structure and attempted to unload.
    Delivery never completed.
25. The player linked 15 critical current-Attempt artifacts plus the immutable evidence directory.
    The non-producing referee audited the direct pixels and returned final fail for a reproducible
    post-route-40 blocker. Exec recorded the same terminal disposition. No actor prescribed production
    work or claimed founder acceptance.

## Terminal disposition

- Final candidate: `41f4fa53a2cd05ab17aea473f3d1be28979b2dcf`.
- Deterministic final gates: 5/5 pass.
- Strict R19 shortcut path: pass, immediate explicit rejection.
- Strict R19 full journey: fail, reproducible post-route-40 completion blocker.
- Consecutive fresh independent completions: 0/2 required.
- Founder acceptance review: withheld; prepared recommendation `revise`.
- Disposition: `product-judgement-failure`.
- Aggregate EXP-11-labelled model spend: USD 170.887523. Individual company ceilings held, but the
  sprint had no aggregate enforcement across replacement companies. This is reported as a harness and
  experimental-efficiency failure.

The complete decision is in `RESULTS.md`; product, evaluator and harness frictions are separated in
`FRICTIONS.md`.
