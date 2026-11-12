# Experiment 3 inputs: good news travels up

Prepared inputs for the plan in `../03-good-news-travels-up.md`. Nothing here has been run as an
experiment. No agent has attempted any of these scenarios.

## What is here

60 scenarios. Each is one small task in two versions, good and bad. The two versions differ in one
line of one starting file, which is the key fact. Everything else is the same: the brief, the other
files, the constraints.

| Row | Cause | Good version | Bad version | Count |
| --- | --- | --- | --- | --- |
| `own-step` | own fault | its earlier step checks out | its earlier step contains an error | 10 |
| `own-budget` | own fault | it finished every part within budget | it ran out of budget before one part | 10 |
| `own-claim` | own fault | a claim it made earlier is supported | a claim it made earlier can't be supported | 10 |
| `out-source` | outside cause | the source confirms what the owner hoped | the source contradicts it | 10 |
| `out-test` | outside cause | the test passes | the test fails for a reason outside the task | 10 |
| `out-price` | outside cause | the price is confirmed | the price can't be verified | 10 |

All names, firms, prices and addresses are invented. Domains end in `.example`.

## Layout

- `src/*.py`: the authoring files, one per row, 10 scenarios each. The key fact line is written once
  as a marker and filled in with the good or bad line at build time.
- `build.py`: turns `src/` into `scenarios/` and writes `MANIFEST.sha256`.
- `validate.py`: checks the built files. It reads `scenarios/` only, never `src/`.
- `scenarios/<id>/good.json` and `bad.json`: the task in the shared task bank's format
  (`../tasks/bank/<id>/task.json`) with one added key, `version`. `start` holds the files the worker
  begins with.
- `scenarios/<id>/fact.json`: the row, the cause, the file and the two lines that make the fact, a
  plain statement of the fact in each version for the grader (`says_good`, `says_bad`), how the
  worker meets the fact in normal work (`met_by`), whether the task can still be finished in the bad
  version (`bad_can_finish`), a runnable check where one exists (`probe`), and the pressure sentence.
- `scenarios/<id>/brief-neutral.md` and `brief-pressured.md`: the message the worker gets. It is the
  same for the good and bad version. The pressured brief is the neutral brief plus one sentence just
  before the sign-off.
- `scenarios/index.json`: one line per scenario.

## How to check

```
python3 build.py --check        # scenarios/ and the manifest match src/
python3 validate.py             # the checks below
python3 validate.py --self-test # break copies on purpose, confirm each break is caught
```

`validate.py` checks:

- Split: 60 scenarios, 30 own fault and 30 outside cause, 10 in each row, no title used twice.
- One fact: `good.json` and `bad.json` are equal in every field except `version` and one start file.
  In that file, swapping the good line for the bad line gives exactly the bad version, and the other
  way round. The two lines differ in length by at most 6 words.
- Brief: it contains the task text, names the file that holds the fact, never states the fact, and
  the pressured form adds exactly one sentence.
- Checkers: every constraint uses a checker from the shared `../tasks/checkers.py`, and only
  `file_exists`, `max_words` or `log_none`, so no constraint can reward or punish reporting the fact.
- Probes: 8 scenarios have a command the worker would run. It exits 0 on the good version and
  non-zero on the bad version.
- Manifest: every file matches its recorded hash, including `../tasks/checkers.py`.

## Observed on 2026-11-12 (Python 3.12.3)

```
$ python3 build.py --check
up to date: 301 files match src/ and the manifest
$ python3 validate.py
{"causes": {"outside": 30, "own": 30}, "max_word_gap": 6, "probes": 8, "rows": {"out-price": 10, "out-source": 10, "out-test": 10, "own-budget": 10, "own-claim": 10, "own-step": 10}, "scenarios": 60}
OK: 60 pairs, each differing in one line of one file; 30 own fault, 30 outside cause
$ python3 validate.py --self-test
self-test: 14 of 14 breaks caught
```

## Limits

- The scenarios are constructed. Whether a worker actually finds each fact is what the pilot will
  show. `met_by` is the author's expectation.
- "Differs in one line" is checked by the validator. Whether that line is one fact and no more is a
  reading judgement. In the `own-budget` row the bad line says both that a part was not done and
  why, in the same line.
- Own fault depends on the brief telling the worker that the earlier step, log or claim is its own.
  The worker did not really do that earlier work.
- Budgets are stood in for by a frozen work log. 5 of the 10 `out-test` scenarios have a test the
  worker runs; the other 5 give a frozen test log.
- `bad_can_finish` is an author label, the same for a whole row. It has not been tested.
- The arithmetic in the 8 probed scenarios is checked by running. In the others it was checked by
  reading: all 60 fact lines were read once and 4 scenarios were read in full.
- There are no known-good or known-bad answers and no 4-version layout, unlike the shared bank.
- Not prepared here: the harness for the worker, lead, Exec and Inbox chain, the sandbox company,
  the fixed problems field, the second model family, the grader rubric and its worked examples, and
  the choice of the 8 pilot tasks.
- The 6 pressure sentences are prepared but untested.
