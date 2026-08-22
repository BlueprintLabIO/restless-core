# v20 — exact recovery context works; prompt-only Work identity does not

## Change under test

Complete the best retained mode-C design with:

- previous Attempt outcomes and a bounded exact Git diff in every recovery lease;
- a workspace-relative executable gate for the requested `X` path and regressions;
- actor-independent model pools for free-form Exec wakes as well as Work Attempts; and
- an explicit workplace rule that messages never reopen blocked Work and `repair` must be used.

Preflight was run twice as the launch contract changed: Python compile passed and the
coordination/adversarial suite remained 34/34 with SQLite quick check `ok`.

## Evidence

- The first Exec launch passed a live zero-price proof for
  `nvidia/nemotron-3-super-120b-a12b:free`, then hit an NVIDIA overload before tokens. A fresh wake
  later completed, proving the singleton Exec also needs provider-independent launch selection.
- The first phase sent guidance with `delivery: queued_for_next_wake` but did not repair Work. The
  bounded runner ended instead of pretending that message was runnable Work.
- On resume, Exec violated the stable-owner rule and reassigned `work-b53ec0f739` from
  `gameplay-systems` to `experience-presentation`.
- Revision 8 received the intended recovery packet: all seven prior Attempt outcomes, exact
  `M js/game.js` state, the four-line uncommitted diff, and the new executable gate. North Mini Code's
  live proof at `2026-08-22T01:05:26.419Z` showed zero price and tool support.
- That recovery Attempt advanced the preserved diff from four inserted lines to a syntax-valid
  61-addition/13-deletion change implementing `_tryExploration` and repairing indentation. It still
  reached 18 turns before creating `verify-exploration-ability.mjs`, committing, or reporting.
- While cancelling that Attempt, Exec abandoned the recoverable Work and commissioned equivalent
  replacements. It eventually created four Work nodes with the same exact outcome:
  `work-b53ec0f739`, `work-e3ea3fff32`, `work-aba2903615`, and `work-0754149601`.
- The replacement Workspaces started from the unchanged candidate, so they did not receive the useful
  61-line delta. One North Attempt consumed another 18 calls and 243,911 input tokens without changing
  its workspace. A second was stopped and reconciled to `unknown`; a fourth Work remained active but
  unclaimed when the churn stop condition fired.
- The original partial workspace still exists and `node --check js/game.js` passes, but there is no
  commit, no new verification file, no artifact, no integration, and no independent review.
- Candidate remains seed `514b7b3d0a65e093af608b08ca142344412181f4`; artifacts are empty; all started
  Attempts are terminal or explicitly unknown; SQLite quick check is `ok`.
- v20 recorded at least 587,611 input/cache tokens and 47 Staff tool calls. A cancellation race caused
  the coordinator turn row to say `controller_cancelled` with null usage while the completed harness
  summary for the same turn says `max_turns`, 144,071 input tokens, and 18 calls. This result/usage
  divergence is an unresolved harness defect.

## Score

Outcome score: **21/100** (no-artifact cap 39).

| Dimension | Points | Evidence |
| --- | ---: | --- |
| Accepted outcome /30 | 0 | no commit, executable outcome check, candidate advance, or artifact |
| Coordination /20 | 4 | bounded Staff work occurred, but ownership changed and one outcome fragmented into four Work nodes |
| Recovery/truth /15 | 10 | useful diff and failures survived; every Attempt is terminal/unknown, but replacements severed recovery continuity |
| Review/evidence /15 | 0 | the required check file, integration and independent review never existed |
| Efficiency/attention /10 | 0 | at least 47 Staff calls and 587k input/cache tokens produced only an uncommitted partial delta |
| Harness/control /10 | 7 | exact prompts/models/free proofs/scopes streamed, but cancellation lost a completed result and usage |

## Final 10x conclusion

The next gain will not come from a longer turn budget, another model, more prompt text, a Pi fork, or a
workflow engine. Restless needs a tiny amount of deterministic coordination integrity around the
existing flexible commands:

1. A provider failure or turn limit repairs the same Work and preserves its owner/workspace by default.
2. `send` adds context; only `repair` resumes blocked Work.
3. Commissioning an exact duplicate open outcome returns the existing Work unless the caller explicitly
   replaces or branches it while carrying forward its recovery inputs.
4. An in-flight Attempt cannot be abandoned and replaced without first preserving its observed
   workspace delta and terminal Runtime result.
5. Runtime result acceptance is atomic: a late cancellation cannot overwrite a completed result or
   discard usage.

These are integrity rules for observable coordination state, not a scripted company workflow. Team
shape, decomposition, tools, implementation and judgement remain model-directed.
