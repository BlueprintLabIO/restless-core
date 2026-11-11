# OI shared task bank

The task bank for the Organisational Intelligence experiments, as specified in
`../02-telephone-game.md` (section "Tasks") and `../METHOD.md`. It is inputs and checkers only.
No experiment has been run with it.

## What is here

| Path | What it is |
| --- | --- |
| `src/code.py`, `src/research.py`, `src/resale.py`, `src/email.py` | The authoring files. 10 tasks each. Edit these, never `bank/`. |
| `build.py` | Writes `bank/` and `MANIFEST.sha256` from `src/`. `--check` fails if they differ. |
| `bank/<task-id>/task.json` | The frozen task: ask, background, starting files, 10 constraints with checker and arguments. |
| `bank/<task-id>/v1.md` to `v4.md` | The owner request in 4 versions. |
| `bank/<task-id>/versions.json` | Where each constraint sits in each version and how it is phrased. |
| `bank/<task-id>/fixtures.json` | One known-good answer and one known-bad answer. |
| `bank/index.json` | The 40 task ids, kinds and titles. |
| `MANIFEST.sha256` | sha256 of every file in `bank/` and of `checkers.py`. |
| `checkers.py` | One function per checker kind, and a command line. |
| `validate.py` | Proves the bank is well formed. |

Counts: 40 tasks (10 code, 10 research, 10 resale, 10 customer email), 400 constraints, 160 versions.
Every task has 10 constraints and at least one of each type: number, format, phrase, style,
action, include.

## The four versions

Each constraint is placed in one of four cells: edge or middle of the message, phrased as important
or as an aside. Across v1 to v4 every constraint appears once in each cell. "Edge" is the block
right after the ask or right before the sign-off. "Middle" sits between background sentences.
Order inside a block and the wording frame come from a sha256 hash of the task id, version and
constraint id, so the build gives the same bytes on any machine. A constraint phrased as an aside
is still binding.

## Submission format

A submission is a directory holding:

- the worker's workspace: the task's starting files plus whatever the worker wrote;
- `toollog.jsonl` at the top, one JSON object per line:
  `{"time": "2026-03-02T09:05", "action": "send_email", "target": "priya@customer.example"}`.

Actions used by the bank: `send_email`, `send_message`, `post_listing`, `read_file`, `write_file`,
`delete_file`, `web_fetch`, `run_command`. Times are on the task's simulated clock (`clock` in
`task.json` is the start) and fall on one simulated day.

## How to run

Python 3 standard library only. Tested with Python 3.12.3.

```
python3 build.py            # rewrite bank/ and MANIFEST.sha256 from src/
python3 build.py --check    # fail if bank/ is not what src/ builds
python3 validate.py         # exit 0 only if the whole bank holds
python3 checkers.py TASK_ID SUBMISSION_DIR   # one JSON line per constraint: kept true or false
```

`checkers.py` exits 0 whenever it could check. Read the `kept` values, not the exit code.

`validate.py` checks, per task: the shape; 10 constraints covering all six types; a checker that
exists for each; 4 versions that each state every constraint once and balance position and
phrasing; that every checker passes on the known-good answer and fails on the known-bad answer.
It also checks 40 tasks, 10 per kind, and every hash in the manifest. Fixtures are written to a
temporary directory and read back the way a real submission is.

## What the harness must supply

None of this exists yet.

- A sandbox per run that starts with the files in `task["start"]` and the text of one `vN.md`.
- Stub tools that do nothing in the world and append a row to `toollog.jsonl` for each action
  listed above.
- A simulated clock that starts at `task["clock"]` and stamps each row.
- A copy of the finished workspace plus `toollog.jsonl` handed to `checkers.py`.

## Limits

- Checkers read text and the tool log. They never run submitted code. Whether a code answer works
  is for the plan's separate quality rubric. The code tasks do carry starting tests the rubric can use.
- A checker measures the rule as written, not its intent. "British spelling" is a word list of
  common American forms. "No contractions" is a pattern. Both can miss cases.
- "Do not do X" action rules pass when the log is empty, so a worker that does nothing keeps them.
  Score them alongside the rules that need something written.
- Each task has one known-bad answer that breaks all 10 constraints. There is no separate bad
  answer per constraint, so the validator does not show that each checker fails alone on its own
  rule with everything else correct.
- If the deliverable has the wrong file name, content rules are judged on the alphabetically first
  new file. The file name rule fails on its own.
- The 5 to 15 minute size of each task is an estimate. No agent has attempted any task.
- Everything is invented. Names, firms, prices and addresses are not real, domains end in
  `.example`, and the resale price files and research snapshots are frozen files supplied with the
  task. Nothing is fetched live.
