# S19-T1 — Install scenario tooling and the Godot delivery lane

**Layer:** Company Runtime.

**Observed friction served:** A game-oriented Staff member has no live-probed engine, export template,
or one-command evidence loop in its persistent workspace.

## Outcome

The Company image supplies a pinned Godot engine with export templates and an ordinary
`restless-scenario` executable. A company skill explains truthful probes, scenario execution, evidence
collection and the existing artifact-reference handoff.

## Acceptance

- The image installs Godot 4.7.2 from the official arm64/amd64 Linux releases with pinned checksums,
  together with the matching Windows x86_64 export templates proved by this sprint.
- Existing and newly reconciled company volumes can resolve export templates without copying engine
  files into project repositories.
- `restless-scenario doctor <package>` checks only declared commands and writes/prints their observed
  availability.
- `restless-scenario run <package>` invokes scenario-owned commands from the package directory,
  captures bounded phase logs, and emits one ordinary run/evidence manifest below the requested output
  location.
- The runner never calls Restless owner/OrgIntel mutations, external effects, hidden LLM APIs or a
  shell parser. A worker attaches a useful result through the existing `restless work artifact` path.
- The image live-probes `godot --version` and a small Windows export through a scenario, not only a
  Docker build layer.

## Deletion target

Host-only Godot assumptions and copied ad hoc command transcripts used as pseudo-evidence.
