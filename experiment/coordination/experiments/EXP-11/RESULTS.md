# EXP-11 results — autonomy improved the artifact, not enough to make it independently playable

**Status:** Complete  
**Disposition:** `product-judgement-failure`  
**Founder decision:** `revise`; no acceptance target is being presented  
**Final candidate:** `41f4fa53a2cd05ab17aea473f3d1be28979b2dcf`

## Decision

The hypothesis is not supported at the tested frontier.

One standing non-producing Game Product lead and one end-to-end gameplay worker made substantial,
attributable improvements without an owner-managed backlog. They repaired view-relative movement,
real collision, recoverable cargo handling, interaction feedback, the exit-to-unload path, shortcut
prevention, and route-end recovery. The exact final candidate passed five deterministic gates and an
independent production-side native review.

It still did not pass the frozen independent-usability contract. In the final strict clean-room run,
an exact GPT-5.6 Sol player:

1. proved that the route-zero on-foot shortcut is explicitly rejected;
2. picked up the parcel, deliberately dropped it, and recovered it;
3. entered the truck carrying the parcel and drove from `ROUTE 0 / 40 m` to the visible
   `ROUTE END REACHED — EXIT TO DELIVER` state;
4. exited, re-entered, moved again at the endpoint, exited a second time, and attempted to unload in
   the visible destination structure; and
5. never reached visible delivery completion.

The resulting native sequence is a reproducible product blocker under the stated public controls.
It establishes an experience failure, not its internal implementation cause. Source and logs were
withheld from the referee, as required.

The required two consecutive fresh independent completions were therefore not achieved, and founder
taste review was correctly withheld.

## What changed in the game

The production organisation started from baseline commit
`84ff1745b29267708599e94036ec6f7a2a7e0457` and produced final experimental candidate
`41f4fa53a2cd05ab17aea473f3d1be28979b2dcf`.

Material product changes included:

- a real `CharacterBody3D` player and physical world/truck geometry instead of hand-written position
  movement and boundary clamps;
- camera-relative on-foot movement and first-person mouse look;
- host validation of remote movement and authoritative cargo, seat, route, and delivery state;
- collision-correct truck ramp geometry;
- visible success and rejection feedback for pickup, drop, seating, exit, shortcut, and unload paths;
- recoverable parcel drops and natural seat re-entry after an early exit;
- an explicit journey state that rejects carrying the parcel directly to the destination; and
- endpoint presentation intended to keep the displayed route and host-owned journey stage aligned.

This is a large improvement over the starting walking skeleton. It is still a compact prototype, not
a finished game: fun, audio, art quality, internet multiplayer, performance, reconnects, accessibility,
controllers, and content breadth remain outside the evidence.

## Strongest candidate evidence

The frozen candidate has these recorded hashes:

| File | SHA-256 |
| --- | --- |
| `main.gd` | `2bb5f472009754b05185bf5faee4512b7704e45fdba046dc36cbac4a34b1d3a0` |
| `main.gd.uid` | `85f0a82ca47bc98af847267b87cdcde5a8245752bf63cf42179dc47f6d77f5ed` |
| `main.tscn` | `ebc471afff5b3b8a8e20f3c3dfd455276fdb968084478c653d5ee9a7624a41d1` |
| `project.godot` | `bcd630f68188c625e564d5499170ca8d112a96ff2d347e24b13ed183e14f264e` |

The final production Attempt passed all five declared gates from the same candidate:

- script validation;
- visible endpoint loaded journey completion;
- route-zero shortcut rejection;
- seat re-entry and journey recovery; and
- movement, collision, and cargo recovery.

Those gates prove deterministic mechanics. They do not overturn the clean-room usability failure.

## Independent evidence chronology

The chronology matters because several apparently passing states were later falsified by stronger
evidence.

| Arm | Candidate | Result | Decision value |
| --- | --- | --- | --- |
| R8 | `a1d96fe4` | Revise | First valid exact-Sol clean-room run; endpoint interaction did not complete. |
| R9 | `09fe09d3` | Fail | Directly completed delivery on foot at route zero, exposing a journey bypass. |
| R10 | `09fe09d3` | Full journey completed with friction | Demonstrated mechanical completion, but did not close the shortcut question. |
| R11 | `7e76d1e0` | Pass recommendation | Full journey completed and no shortcut completion observed. |
| R12 | `7e76d1e0` | Infrastructure-invalid | Numeric CLIENT title repeatedly rendered HOST pixels; no gameplay claim. |
| R13 | `7e76d1e0` | Evidence fail | Full journey completed; shortcut evidence was outside the actual unload state. |
| R14 | `7e76d1e0` | Pass | Both paths appeared to pass after a capability-expiry continuation. |
| R15 | `7e76d1e0` | Fail | Reproduced route-40 exit/unload blocker and unavailable natural re-entry. |
| R16 | `41f4fa53` | Inconclusive | Later shortcut state was correct, but the immediate action pair was ambiguous. |
| R17 | `41f4fa53` | Inconclusive | Action and capture were separated by 11 seconds, longer than the rejection banner. |
| R18 | `41f4fa53` | Shortcut pass; run invalidated | Atomic capture proved rejection; a supersession/attempt-limit bug prevented the second Work. |
| R19 | `41f4fa53` | Final fail | Atomic shortcut rejection passed; fresh full journey reproduced the post-route-40 blocker. |

R19 is the terminal decision evidence because it uses the strongest protocol: read-only candidate,
repo-less Work, fresh process per responsibility, exact numeric CLIENT capture, native image ingestion,
current-Attempt artifact links, an atomic transient-state capture, and independent lead audit.
Its compact terminal runtime projection is retained at `results/referee-r19/runtime-summary.json`.

## What the final run directly proves

### Passed

- Exact model route: `litellm/gpt-5.6-sol`.
- Fresh HOST and CLIENT launch with direct CLIENT pixel identity.
- Ordinary keyboard/pointer control, capture, and model-native PNG reads.
- Pickup, deliberate drop, and recovery.
- Loaded driving from route start to the rendered endpoint.
- Leaving the seat and later re-entering it.
- Explicit immediate route-zero shortcut rejection.
- Durable evidence linked to the exact current Work and Attempt.
- Correct separation between Staff observation, referee judgement, and Exec disposition.

### Failed or unproven

- Visible delivery completion in the final fresh journey.
- Two consecutive fresh independent completions.
- Founder `accept`, `revise`, or `reject` from hands-on native play; only the prepared recommendation
  is `revise`.
- GLM-5.3 Flash as requested in the original hypothesis. The exact route was unavailable and the
  founder authorised GPT-5.6 Sol instead.
- Fun, polish, production readiness, internet networking, broad multiplayer, audio, accessibility,
  controller support, or sustained performance.

## Organisational result

The supervision architecture was useful but insufficient by itself.

- Exec delegated to a lead and generally returned to availability.
- The standing lead kept production with one end-to-end worker; no evidence justified a larger team.
- Material work arose from artifact and referee evidence rather than owner-written tasks.
- The lead correctly revised earlier candidates after outside evidence contradicted its acceptance.
- The same lead nevertheless prepared founder-ready states that stricter clean-room evidence later
  disproved.

The enduring conclusion is not “teams do not work.” It is that supervision cannot compensate for a
weak or contaminated acceptance boundary. A compact coupled implementation still benefits from one
end-to-end producer, while independent use must remain outside the production context and must be
allowed to fail the candidate.

## Cost and validity

Recorded model spend across all EXP-11-labelled companies was **USD 170.887523**:

- production company: **USD 48.606969**;
- admission, referee, and harness-repair companies: **USD 122.280554**;
- final R19 company alone: **USD 24.835997**.

The production company stayed below its USD 50 ceiling and R19 stayed below its USD 25 ceiling. The
sprint, however, lacked an aggregate envelope across replacement companies. Owner-authorised
continuation made the evidence permissible, but the original USD 50 “total” wording was not enforced
as a program invariant. This is a real experimental-harness failure and the largest efficiency
warning in the result.

The high evaluation cost does not invalidate the final native observation, but it does make this
protocol unsuitable as a default product loop. Future use should admit the route once, run one
calibration, use one strict two-part evaluator company, and stop on the first conclusive blocker.

## Decision for Restless

Keep:

- Exec-to-lead delegation;
- a non-producing accountable lead;
- one end-to-end worker for a compact coupled artifact;
- event-driven candidate-to-referee and report-to-lead flow;
- fresh independent native use with withheld producer context; and
- deterministic gates as mechanics evidence, never as a proxy for usability.

Repair before repeating:

1. enforce an aggregate experiment budget across companies;
2. make repo-less evidence outputs writable while keeping the candidate read-only;
3. stop dependency edges from automatically attaching upstream artifact locators when the downstream
   role is explicitly blind;
4. treat superseded Attempts as replaced snapshots, not consumed execution allowance;
5. let an explicit repaired resume grant exactly one attributable successor Attempt;
6. eliminate duplicate terminal lead-to-Exec wakes;
7. replace long foreground GUI commands with durable process handles and event completion; and
8. provide a native interaction trace primitive so a vision player does not spend most of its budget
   rediscovering window and capture mechanics.

## Next product move

Do not add content. Run one bounded product repair focused only on the route-end exit/unload experience,
then one fresh strict replication. The repair should begin from the R19 visual sequence and must not
assume the internal cause. It should make the permitted next action visually unambiguous and make the
rendered endpoint, authoritative journey stage, exit, destination placement, and unload result agree.

If that one repair cannot produce two fresh completions within a predeclared aggregate budget, stop
Swift Arrival autonomous playability work and return the interaction design to founder judgement.
