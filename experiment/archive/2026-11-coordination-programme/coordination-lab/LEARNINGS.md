# Coordination lab learnings

This is the compact, durable synthesis. Per-run evidence stays in run reports; this file changes only
when evidence alters the working model.

## Historical v0-v2 evidence

1. The seven generic coordination commands covered every observed coordination action. No eighth
   command was required.
2. A command mutation is not a turn boundary. In v0 the Exec commissioned Work, kept its turn, polled,
   and then implemented Staff work itself.
3. Asynchronous dispatch materially improved real parallel production in v1, but prose rules alone did
   not make workers terminalise, preserve graph provenance, or converge candidates.
4. Real useful artifacts existed despite organisational failure. The bottleneck was connecting and
   integrating work, not simply the models' ability to create it.
5. Multiple integration writers produced two incompatible “merged” candidates. Integration needs one
   accountable owner and exact input artifacts, not a special universal merge protocol.
6. Shared SQLite WAL over a host/container bind mount and concurrent direct JSONL appends failed. V2's
   single writer fixed these substrate faults and now passes 30 deterministic probes.
7. V2 overcorrected into leases, a single-writer coordinator, and workflow-like machinery. The current
   architecture explicitly rejects promoting that shape without repeated production evidence. It is a
   comparison baseline, not the target.
8. Terminal callbacks cannot be merely aspirational. If a process exits without a callback, the
   substrate must record `unknown`, preserve files, and enable a bounded evidence-aware continuation.
9. Owner judgement was misused when coordination became confusing. Reversible operating decisions are
   Exec work; the owner is not a recovery mechanism.

## Current structural hypothesis

The highest-leverage move is not replacing ACP, MCP, or model providers wholesale. It is owning the
small execution layer that existing coding harnesses currently control:

- construct the exact actor identity and instruction hierarchy;
- expose an explicit per-session tool/skill set;
- stream ordered thought/text/tool lifecycle events;
- carry cancellation, stop reason, usage, and unknown outcomes faithfully;
- keep credentials and authority out of model-visible context;
- allow OrgIntel to wake a fresh bounded reaction to an event.

Pi supplies the model/tool loop. ACP remains the replaceable transport boundary. OrgIntel supplies
coordination. Thin MCP adapters may expose structured services, but MCP is not the scheduler or policy
engine.

## Open hypotheses to test

- A strong quiescence rule plus event wakes may be sufficient; a durable lease state machine may not
  be necessary in production OrgIntel.
- Explicit artifact-bound Attempts and a single accountable integrator may outperform broad project
  write restrictions while preserving the persistent shared-company-computer model.
- Role-specific model selection may improve output, but only if compared without changing the
  coordination mechanism simultaneously.
- A few workplace defaults—delegate substantial craft, end a coordination wake after dispatch, report
  terminally, integrate once, review the native artifact—may produce more value than additional
  commands or types.
- ACP v1 may be sufficient for the current bridge. ACP v2 becomes valuable if session resume/close,
  usage, and richer running states eliminate Restless-specific transport work; it should not absorb
  organisational semantics.

## New first-party harness evidence

10. A minimal ACP v1 server composed from Pi streamed thought chunks, a real tool start/completion,
    answer chunks, usage, and terminal state in chronological order using a live zero-price model.
11. Cancellation is not complete when only the model loop is aborted. Shell descendants and inherited
    pipes can keep a turn alive. Process-group termination plus an already-aborted-signal check reduced
    a `sleep 30` cancellation to 6 ms.
12. Usage must aggregate all assistant messages in the bounded prompt. Reading only the final synthetic
    aborted/error message falsely reports zero usage.
13. The same exact ACP/Pi contract worked with Laguna XS and North Mini Code. A third model failed with
    an upstream 429 before its first token; provider availability must remain a retryable runtime event,
    not be mislabelled as Staff refusal or Work failure.
14. ACP v1's stop-reason set has no `error`. Restless needs explicit outcome metadata while staying on
    v1; ACP v2's extensible stop reason may later remove this translation wart.
15. ACP-supplied stdio MCP tools can be validated, discovered, called, streamed, and closed without the
    harness owning their semantics. This supports retaining MCP as an adapter rather than replacing it.
16. A strong coding model given the full commercial mandate spent ten turns and twenty tool calls only
    orienting itself, consumed 83,959 input tokens, and changed nothing. More turn budget would reward
    the wrong behavior. Outcome framing is a coordination function, not a larger-context problem.
17. Pi's graceful `shouldStopAfterTurn` emits `agent_end` without changing the model's last stop reason.
    A harness that imposes a turn ceiling must record `max_turn_requests` itself; otherwise a bounded
    but unfinished tool loop is falsely labelled completed.
18. A loose Exec produced a detailed, testable brief that duplicated capabilities explicitly marked
    implemented in the README it had just read. Formal specificity is not groundedness. Context should
    lead with observed current capabilities/gaps, and Work needs a terminal challenge/redirect path.
19. Passing that stale brief as prose caused the worker to spend 14 turns re-reading rather than reject
    the premise. Explicit Work plus `report(blocked|outcome_met)` is valuable partly because it gives a
    bad assignment a truthful organisational response.
20. Logging Pi's full growing partial assistant message on every token turned one worker run into 112 MB
    of JSONL. The durable trace needs deltas and lifecycle summaries, not repeated transcript snapshots.
21. Durable telemetry and owner-facing streaming are different products. Coalescing only persisted
    text/thought deltas while keeping ACP token-live reduced an equivalent trace from 1,134,021 bytes
    to 37,111 bytes (30.56x) without obscuring tool or terminal chronology.
22. A read-only agent still needs perception. In the first mode-C run, Exec spent six of seven calls
    trying to use a file reader as a directory browser, created no Work, and ended at its truthful
    ceiling. Scoped listing and search are harness primitives; omitting them does not simplify the
    company, it makes its agents blind.
23. Better perception in the wrong role only made churn more successful. Once listing worked, Exec
    spent its whole wake reading implementation and still delegated nothing. Production reconnaissance
    is outcome-sized Staff Work when it displaces executive coordination.
24. A fresh ACP process has no implicit organisational memory. An event wake containing only its cause
    and current Work graph lost the owner directive/current capability evidence, so Exec commissioned
    combat and switching that the seed already verified. Context assembly must repeat stable Goal and
    current-state evidence on every bounded reaction.
25. Optional infrastructure detail invites plausible invention. Exposing `base_ref` led Exec to choose
    conventional `main` against a repository whose canonical branch is `candidate`, stopping Work
    claim. Producer Work should snapshot the current candidate by default; exact revisions enter via
    dependency artifacts, not Git folklore in an executive command.
26. Repeating durable Goal and exact candidate evidence while removing production inspection from
    Exec changed behaviour immediately: one coordination call created bounded reconnaissance and Exec
    quiesced. Role-specific tool posture can be a better control than exhorting an executive not to
    inspect implementation.
27. A model-authored “argv” containing one shell command string is neither structured nor executable.
    It failed literally with exit 127 and caused a valid 139-line artifact commit to be rejected. Gate
    shape must be validated when Work is commissioned, with shell interpretation explicit if allowed.
28. Automatic finalisation is unsafe. After an upstream timeout before any critic work, a clean
    unchanged checkout was interpreted as a completed review and the base commit was accepted even
    though the named review file did not exist. No callback is `unknown`; another model turn must not
    manufacture success from process cleanliness.
29. `outcome_met` needs positive production evidence. In this Git-backed comparison, HEAD must advance
    beyond the Work input; clean status alone says only that no uncommitted files exist.
30. Text/thought delta coalescing did not cover tool-argument deltas. Growing `toolcall_delta.partial`
    snapshots pushed v11 durable telemetry to 26 MB. Every streamed event family needs an explicit
    durable representation; unknown event shapes must be summarized, not copied wholesale.
31. Deleting the automatic finalisation turn made recovery both smaller and safer. In a live forced
    one-turn probe, the absent callback became `unknown`, no artifact or decision was invented, the
    exact workspace survived, and Exec received a normal continuation event.
32. The comparison fault suite itself had encoded the bad assumption that an unchanged seed commit
    could satisfy newly commissioned Work. Making the test create a real commit before reporting was
    part of fixing the product invariant, not mere fixture maintenance.
33. Free-model availability is operationally volatile even after a valid zero-price catalogue proof.
    Laguna XS completed three tool calls and then returned a shared-pool 429. The correct organisational
    result was `unknown` with the same Work/workspace preserved, not transparent model substitution or
    a new responsibility.
34. Provider substitution belongs at the Attempt boundary. An explicit repair of the same Work with
    North Mini Code advanced revision 2, preserved revision 1 as unknown, produced an exact commit,
    passed a structured gate, and terminally reported without controller inference.
35. The positive callback contract is viable with the thin ACP/Pi/MCP stack. The remaining friction in
    the focused case was efficiency—ten tool calls for one line—not missing protocol expressiveness.
36. Streaming and persistence must be separate for every event family, not only prose. Omitting
    redundant tool-argument deltas and hashing/bounding tool payloads cut an equivalent North trace
    from 331,076 to 69,618 bytes while ACP still delivered 1,143 live thought chunks.
37. Context projections are adapter-specific. Staff was told to use its current directory but also
    received Docker's `/workspace` from observed state; the contradiction consumed most of a bounded
    turn. OrgIntel should expose Git/work evidence, while the Runtime Bridge supplies the local cwd.
38. Removing Runtime-local locators was enough for the repaired North Attempt to reuse an uncommitted
    marker, make a real commit, pass its gate, and callback in nine calls. Persistent workspace state
    plus a small Git observation is more useful recovery context than path narration.
39. Actor identity and model identity are different. In v17, four provider errors caused Exec to
    reassign the same Work merely to reach another configured model, confusing accountability with
    inference infrastructure. Repairs should rotate an explicit model/provider pool while ownership
    stays stable.
40. Model-name diversity is not provider diversity. Gemma 31B and 26B both failed through the same
    Google AI Studio free pool. Allocation evidence must include the upstream/provider failure domain,
    not only the model ID.
41. Requiring critic Work to depend on completed Work prevented v11's invalid pattern of “reviewing”
    an unchanged baseline. Independent review is a downstream relationship, not an actor label alone.
42. Durable evidence is ineffective when it is attached to the wrong layer. V17's exact provider 429s
    existed in Runtime turn transcripts, while the Attempt visible to Exec said only “process ended as
    error.” Exec spent a wake asking Staff to rediscover an error the harness had already observed.
43. A message with `delivery: next_wake` is not an RPC. In the comparison runner, sending one to an
    actor with no ready Work created an undeliverable outbox row and an infinite idle wait. Production
    OrgIntel requires free-form actor wakes; a bounded lab must at least avoid treating deferred inbox
    context as runnable work.
44. Stable model-pool configuration is not evidence of model rotation. Because no revision-5 Attempt
    launched in v18, the implementation remains uncredited until a live repair records a different
    provider under the same Work owner.
45. Actor-independent provider rotation works through the existing ACP/Pi boundary. One durable Work
    stayed owned by `gameplay-systems` while explicit repairs launched Cohere, NVIDIA and Google models
    by revision, each with its own exact free proof and no hidden fallback.
46. Persistent files are necessary but not sufficient recovery context. North's four uncommitted lines
    survived into Nemotron's Attempt, yet the prompt projected only `M js/game.js`; the new model spent
    its entire turn rediscovering where and how to continue. Recovery wakes need the bounded delta and
    prior Attempt outcomes named by the OrgIntel contract.
47. Free does not mean cheap. V19 spent zero dollars but more than 800k accounted input/cache tokens and
    36 Staff tool calls for an incomplete four-line change. Efficiency scoring must retain token, turn
    and attention costs even when provider pricing is zero.
48. A model pool needs failure-domain policy, not endless round-robin. Cycling revision 7 back to the
    already-known Google shared pool reproduced the same zero-token 429. Health evidence should exclude
    a provider temporarily while preserving deterministic, recorded selection; it must not silently
    retry or change the Work owner.
49. Exact recovery context materially improved useful work: North advanced a four-line partial delta to
    a syntax-valid 61-addition implementation after receiving the prior Attempt outcomes and diff. It
    still failed to test, commit, or report within the same turn, so recovery context helps but does not
    substitute for bounded Work sizing and terminal discipline.
50. Prompt-only Work identity is not reliable enough. Despite explicit rules, Exec reassigned an actor,
    abandoned a recoverable workspace, and commissioned three exact duplicate outcomes. A deterministic
    duplicate/replacement guard is justified because it protects coordination truth without prescribing
    how the work is performed.
51. A `revises` edge does not automatically carry recovery state. The replacement Work nodes started at
    the unchanged candidate and lost the original 61-line delta. Replacement must explicitly inherit or
    reference the prior workspace/artifact/Attempt inputs, or repair the original Work instead.
52. Cancellation and process completion can race. In v20 the Pi summary recorded `max_turns`, usage and
    18 calls while the coordinator row recorded `controller_cancelled` with null usage. Runtime result
    ingestion must be atomic and cancellation must not overwrite an already observed terminal result.
53. The singleton Exec needs the same actor/model separation as Staff. Its first v20 launch passed the
    free-model gate and then hit an NVIDIA overload before tokens; model pools cannot be Staff-only.
54. The v23 “strong singleton” result identified the right production unit but assigned it to the
    wrong organisational altitude. The singleton should be the accountable team lead, not Exec. Exec
    must dispatch every executable owner request to a standing or temporary lead and return to
    availability so independent departments can continue in parallel.
55. Team size and executive delegation are separate decisions. Exec always delegates to a lead; the
    lead may then work alone on a tightly coupled outcome or add Staff when specialisation or parallel
    latency is expected to exceed communication, integration and review cost.
