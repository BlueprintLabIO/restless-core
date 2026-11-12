# Experiment 5, stage 0: the 100-task suite

The plan is `../05-org-chart-search.md`. The method rules are `../METHOD.md`.
This folder is preparation only. No calibration run and no experiment run has happened.

## What is here

| Family | Count | Where | What a task is |
|---|---|---|---|
| Code | 30 | `tasks/code/code-NN/` | A real Restless change from October or November 2026. The agent starts from the commit before it and gets a brief. The change's own tests are withheld and used to grade. |
| Research | 30 | `tasks/research/research-NN/` and `tasks/research/worlds/w-NN/` | A question about nine frozen documents, with an answer key. |
| Operations | 20 | `tasks/operations/index.json` | The 10 resale and 10 email tasks of the experiment 2 task bank, named by id and file hash. Not copied. |
| Forecast | 20 | `tasks/forecast/slots.json` | Empty slots. Each is filled at run time with one auction that is still open. |

`suite.json` lists all 100 with their content hashes. `split.json` is the draft 60/40 split.
`MANIFEST.sha256` lists every frozen file. `DECISIONS.md` lists what was settled in code and still needs the owner's answer.

## How to run it

Python 3.12, standard library only. Run from this folder.

    python3 validate.py            # counts, hashes, split, leak checks. Exit 0 is a pass.
    python3 validate.py --final    # the same, and fails while the split is still a draft

To rebuild from scratch (gives the same bytes):

    python3 code/freeze.py         # code tasks, from code/selection.json and the Restless history
    python3 research/build.py      # research worlds, questions and keys
    python3 build_suite.py         # operations list, forecast slots, suite.json
    python3 split.py               # draft split.json
    python3 manifest.py            # MANIFEST.sha256

Checks that take longer:

    python3 code/verify.py         # applies every hidden test patch; runs the JavaScript ones before and after the real change
    python3 research/solve.py      # answers every question from the documents alone and compares with the key

To lay out one code task:

    python3 code/materialise.py code-14 /some/empty/dir                     # what the agent gets
    python3 code/materialise.py code-14 /the/agent/dir --add-hidden-tests   # before grading; prints the grade command

The rebuild and `code/verify.py` need the Restless history (`origin/main` up to `9e867631`), the task bank
commit `51f18bf6` and the forecast rules commit `fa229c99` in the local repository. `validate.py` skips
the bank comparison and says so if that commit is absent.

## Code tasks

`code/mine.py` lists every change on `origin/main` in the window that touches both source and test
files, is small enough to be one task, and is not a merge. 79 passed. `code/selection.json` names
the 30 used and 7 reserves.

Each task folder has:

- `task.json`: the commit, its parent, the parent's tree hash, the source and test paths, the grade command.
- `brief.md`: the change's own description with its paragraphs about checks removed, then the hidden test paths and the lines those tests use to load the code.
- `hidden-tests.patch`: the test files of the change, as a patch on the parent.

The agent gets the parent tree with no `.git` folder, so no later history. Grading puts the hidden
test files in place, over anything the agent wrote at those paths, and runs the grade command. Exit 0 is a pass.

20 tasks are graded with `node --test` and need nothing installed. 10 are graded with `cargo test`
on one integration test file.

From the plan, for whoever runs stage 1: no network beyond a package mirror, memory and Library
off, a task is "hard" if the solo agent passes at most 1 of 3 calibration runs, and calibration runs are never reused for scoring.

## Research tasks

Live web questions cannot be frozen, and METHOD.md asks for frozen inputs. So each research task
is a question over a small invented record set: a register of 36 things, an annual return, a units note, a
correction notice, a draft return that was superseded, a withdrawals list, a former names list, a log of 170
entries and a list of void entries. There are 15 such worlds and two questions on each. One asks which things
were above a threshold on the final figures. The other asks which had at least so many log entries in a date range.

The answer is a list of names. The key comes from the generator's own records. A task passes when the
answer's names equal the key exactly. Recall and precision are also recorded.

A question is only kept if every lazy reading gives a different answer from the key: ignoring the correction
notice, ignoring the withdrawals, using the wrong unit, ignoring former names in the log, counting void entries.

`research/solve.py` does not import the generator. It reads only the documents and the question's parameters.

## Operations jobs

`tasks/operations/index.json` names `resale-01..10` and `email-01..10` from the task bank at commit
`51f18bf6` on `research/oi-task-bank`, with the hash of each of the seven files per task.
That branch is not merged into `research/org-intel-programme` yet.

## Forecast slots

20 slots: four categories, five slots each, every price band once per category and one band twice.
The rules come from experiment 4's `rules.json` at `fa229c99`: an auction must be open, have at least one bid,
and end between 2 and 7 days after it is collected. A slot is never filled from an auction that has ended.
Fills belong in a file per run, outside `tasks/`, so the frozen files do not change.

## The split

`split.py` splits each family 60/40 on its own: 18/12 code, 18/12 research, 12/8 operations, 12/8 forecast.
The seed is in the script. The two questions on one research world stay together. Within a family, each sort of
task (test runner, job kind, auction category) is held out in proportion.

The split is a draft. The plan asks for it to be balanced by difficulty, and difficulty is measured in stage 1.
After calibration, save the labels as `labels.json` (task id to true for hard) and run
`python3 split.py --labels labels.json`, then `python3 validate.py --final`.

Held-out tasks must not be run, read into a prompt, or used in the search until stage 3.

## What was run, and what was seen

Run on 12 November 2026 on the final tree:

- `python3 validate.py`: 17 checks pass, exit 0. `--final` fails with one check, as intended for a draft.
- The validator was fed ten broken inputs and failed each one: a task moved across the split, a task on both sides, a research world on both sides with counts kept, a duplicate id, a changed hash, the same brief on both sides, an edited key file, a filled forecast slot, a stray file, and a draft passed as final.
- `split.py --labels` was run once with made-up labels to prove the path works. Those labels were deleted. They say nothing about real difficulty.
- `python3 code/verify.py`: 30 of 30 hidden test patches apply to their parent. For the 20 JavaScript tasks the hidden tests fail on the parent and pass on the real change. Results are in `code/verification.json`.
- `python3 research/solve.py`: 30 of 30 answers equal the key, and no lazy reading gives the key. Results are in `research/solve-report.json`.
- `research/build.py` run twice gives identical files.
- `code/materialise.py` was run for `code-14`: the folder had no `.git`, and the hidden test failed before and after adding it to an untouched copy.

## Not built

- No calibration. No task has a difficulty label. The split is a draft.
- No run harness, no scoring script for agent output, no cost accounting. Those belong to stage 1.
- No research answer grader beyond the stated rule (exact set match on names). The rule is simple, but no script reads an agent's answer file yet.
- No forecast slot is filled.
- No sandbox setup. Experiments run only in sandbox companies, and that route is blocked until the owner signs in.

## Limits

- The 10 Rust tasks were never executed. The machine has no `cargo`. Their patches apply, but nobody has seen those tests fail before the change or pass after it. Some probably need a Postgres database. Check them before calibration and swap in reserves if needed.
- The briefs come from the real change descriptions. Some name the function or module the real change added, which makes the task easier than a bare request.
- The hidden tests are the tests the change's author wrote. A different correct design with other names would fail them.
- The research worlds are invented and tabular. They test careful reading across documents, not searching the web or judging sources. Whether they are hard for a solo agent is unknown.
- The research solver and the generator have the same author. It guards against a slip in the generator, not against a flaw in the design that both share.
- Many search code tasks start from a snapshot of the repository that already contains a held-out change. `validate.py` reports the count. See `DECISIONS.md`.
- The operations jobs and forecast rules point at branches that are not merged. If those branches change, the hashes here will say so.
