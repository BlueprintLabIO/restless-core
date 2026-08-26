# Scenario evidence tools

`restless-scenario` is a small Runtime executable for repeatable, evidence-bearing project runs. It
executes scenario-owned argv commands and writes ordinary files; it is not a scheduler, task system,
agent router, effect runner, or automatic reviewer.

## Run

Inside a Company Runtime after the image is reconciled:

```sh
restless-scenario doctor /company/projects/example/scenario
restless-scenario run /company/projects/example/scenario \
  --output /company/outputs/example-run-001 \
  --seed example-001
```

For local development in this repository:

```sh
node tools/scenario/restless-scenario.mjs validate tools/scenario/fixtures/thymelake-menu-launch
node tools/scenario/test-runner.mjs
```

`doctor` is read-only. `run` requires a new, empty output directory outside the source package, so a
rerun cannot silently overwrite prior evidence.

## Package shape

Each package is an ordinary directory containing `scenario.json` and any native scripts, project files
or inputs it needs. The JSON contract is intentionally compact:

```json
{
  "schema": "restless.scenario-package/v1",
  "id": "example-scenario",
  "version": "1",
  "title": "One observable outcome",
  "run_kind": "test_world_only",
  "human_review_required": true,
  "capabilities": [
    { "id": "node", "argv": ["node", "--version"], "required": true }
  ],
  "phases": [
    { "id": "exercise", "argv": ["node", "run.mjs"] }
  ],
  "evidence": [
    { "id": "result", "path": "result.json", "required": true }
  ],
  "review_target": { "kind": "file", "target": "review.html", "label": "Prepared review" }
}
```

The runner supplies `RESTLESS_SCENARIO_OUTPUT`, `RESTLESS_SCENARIO_SEED`,
`RESTLESS_SCENARIO_ID`, `RESTLESS_SCENARIO_VERSION` and `RESTLESS_SCENARIO_RUN_ID` to each phase.
Phases use those variables to write their outputs. They run in the package directory with an argv array,
never a shell string.

The resulting `run-manifest.json` says only whether the declared mechanical checks verified, failed or
were blocked by a missing capability. It never means the underlying game, business, document or
operation was accepted. When under a Work Attempt, attach the run manifest and chosen native target
through the existing `restless work artifact` command; do not create another persistence path.

## Boundaries

- A package owns its native inputs, commands, evidence and domain assertions.
- The Runtime owns its files, engine, local services and builds.
- OrgIntel owns Work/Attempt responsibility and existing artifact references.
- A lead or owner chooses the review target and accepts judged quality.
- Authority owns all real external effects. Test-world packages cannot manufacture a real effect claim.

Keep packages small and project-local. Promote a repeated convention only after more than one real
outcome has proved it useful.
