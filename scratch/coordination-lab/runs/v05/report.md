# v05 — broad-mandate single-agent baseline

## Change under test

Run one strong free coding model against the unchanged Cosmon seed and complete vertical-slice mandate.
There is no Exec, delegation, Work graph, or team rule beyond “choose one coherent playable increment,
implement, verify, and commit.”

## Evidence

- Mode: A — single agent
- Model: `poolside/laguna-s-2.1:free`; live prompt/completion price `0`
- Seed/HEAD after run: `514b7b3d0a65e093af608b08ca142344412181f4`
- Git status after run: clean
- New commit/artifact: none
- Turns: 10/10
- Tool calls: 20
- Usage: 83,959 input / 824 output / 244,576 cache-read tokens; $0
- Tool pattern: repository listing and log, then 18 reads across README and game modules
- No edit, write, test, browser run, or commit occurred
- The only answer text was the opening sentence: “I'll start by inspecting…”

## Score

Outcome score: **7/100** (no-artifact cap 39; raw score already below it).

| Dimension | Points | Evidence |
| --- | ---: | --- |
| Accepted outcome /30 | 0 | no changed file, commit, or artifact |
| Coordination /20 | 0 | broad mandate remained unframed; no coordination in mode A |
| Recovery/truth /15 | 0 | harness incorrectly labelled turn-ceiling stop as completed |
| Review/evidence /15 | 0 | no outcome to review |
| Efficiency/attention /10 | 2 | no owner interruption, but no useful output after 10 turns |
| Harness/control /10 | 5 | exact launch and live events/usage; terminal semantics were false |

## Dominant failure

The raw agent treated understanding the full game as prerequisite to choosing a bounded improvement.
The important intervention is not more context or turns. It is outcome framing by a coordinator and a
truthful bounded-turn terminal state.

## Decision

Preserve as the A baseline. Do not rerun with a larger budget: that would reward the failure mode.
V06 repairs max-turn truth. V07 tests whether a loose Exec/worker handoff can turn the same broad
mandate into a concrete artifact.
