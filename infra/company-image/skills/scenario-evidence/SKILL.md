---
name: scenario-evidence
description: Run a versioned Runtime scenario, probe its real capabilities, preserve compact evidence, and prepare its native review target. Use for a project outcome that benefits from repeatable setup/run/assertion/evidence—not to choose a plan, team, or business verdict.
---

# Scenario evidence

Use the scenario package already owned by the project. A package is ordinary files and native tools;
it is not a replacement for Work, Git, project services, or lead judgement.

1. Read the package's `scenario.json`, project instructions and source inputs. Identify what the run can
   mechanically prove and what still needs a lead or owner to judge.
2. Probe first:

   ```sh
   restless-scenario doctor <package-directory>
   ```

   Treat a missing required capability as `blocked`. Do not substitute a host executable, a fake
   success, or a narrative claim.
3. Choose a fresh output directory outside the package and run the exact scenario:

   ```sh
   restless-scenario run <package-directory> \
     --output /company/outputs/<scenario>-<run-id> \
     --seed <recorded-seed>
   ```

   The runner writes `run-manifest.json`, phase logs and declared evidence there. It neither chooses
   the project strategy nor performs external effects.
4. Inspect the native review target named by the manifest. A `verified` mechanical result is not a
   product, game, operational or commercial acceptance. State the exact judgement still requested.
5. When this is Work output, link the manifest and the selected native target through the existing
   `restless work artifact` command, tied to the current Attempt. Do not create an ad hoc scenario
   database or claim the output replaced the candidate.

For game work, use the installed `godot` only after `doctor` observes it. Preserve input/network
profiles, client/server logs, exported build and a prepared image/video/native target. For non-code
work, preserve the source inputs, deterministic validation report, rendered output and the human
readiness question. Controlled `_test` input proves the workflow mechanism only; it never proves
customer demand, real operations, gameplay fun, or an external provider effect.
