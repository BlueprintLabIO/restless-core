# v10 — perception works; executive role and wake context fail

## Change under test

Retry mode C from the same seed after adding path-scoped `list` and `search` to the first-party
harness. The coordination machinery and model allocation were unchanged from v09.

## Evidence

- Seed: `514b7b3d0a65e093af608b08ca142344412181f4`, clean and unchanged
- Exec model: `nvidia/nemotron-3-super-120b-a12b:free`; live prompt/completion prices `0`
- First turn: 7/7 turns, 7 tool calls, 67,943 input + 38,016 cache-read / 712 output, $0
- Scoped `list` succeeded and exposed the real repository; five bounded reads succeeded; the final
  oversized read was rejected by schema validation
- Exec nevertheless spent the whole turn reading `index.html`, `game.js`, and `creatures.js`; it
  commissioned nothing and ended truthfully at `max_turns`
- The event controller supplied a fresh idle wake. Because the harness process is session-bounded,
  that wake contained events and empty coordination state but omitted the durable owner directive and
  current capability summary
- On the fresh wake, Exec commissioned explicit Work to `gameplay-systems`: “Basic combat and creature
  switching system implemented and verified”—capabilities the seed README already marks implemented
- Exec supplied `base_ref=main`; the cloned seed exposes `candidate` and `master`, not `main`
- Claiming that Work raised an unhandled `git rev-parse main` error. The Work record survived active,
  but no Attempt was created and the controller stopped
- Commits/artifacts: 0; repository remained clean at the exact seed

## Score

Outcome score: **16/100** (no-artifact cap 39).

| Dimension | Points | Evidence |
| --- | ---: | --- |
| Accepted outcome /30 | 0 | no Attempt, commit, or artifact |
| Coordination /20 | 4 | one explicit accountable Work existed, but duplicated completed capability |
| Recovery/truth /15 | 3 | records survived; claim failure stopped the controller and stranded active Work |
| Review/evidence /15 | 0 | no candidate existed |
| Efficiency/attention /10 | 1 | perception worked, but executive inspection consumed the entire first wake |
| Harness/control /10 | 8 | scopes, events, model proof, usage, and max-turn state remained explicit |

## Dominant failure

Better perception improved tool success but not company output. The role was wrong: Exec used production
tools for implementation archaeology rather than commissioning reconnaissance. The fresh wake then
lost the stable Goal/current-capability context and made a stale assignment. Finally, exposing an
optional Git base made repository folklore an agent responsibility and converted a bad default guess
into a controller crash.

## 10x decision

Change the executive posture, not its turn budget:

1. Every fresh Exec wake receives the durable directive, current candidate README, and canonical
   coordination delta.
2. Exec receives coordination tools only for this experiment. If implementation knowledge is
   insufficient, it commissions bounded discovery to Staff.
3. Remove `base_ref` from the public commission schema; ordinary producer Work snapshots `candidate`
   automatically. Exact input revisions remain explicit through dependency artifacts.

Retain scoped perception for Staff. Preserve v10 as evidence that a good tool in the wrong role does
not fix coordination.
