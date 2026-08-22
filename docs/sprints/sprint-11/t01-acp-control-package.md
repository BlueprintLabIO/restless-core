# S11-T1 · Deliver a fully Restless-controlled ACP session

**Layer:** Company Runtime for process/tool/skill delivery; OrgIntel for actor identity and durable
model preference.

**Observed friction:** Agents saw only ACP-native tool names and concluded installed commands were
absent. Temporary model failover could become the actor's next-wake model. Ambient runtime rules were
hard to distinguish from Restless-owned instructions.

## Outcome

Every Exec and Staff launch receives an inspectable Restless-owned control package: authoritative
company rules, role, assignment/context, selected model candidates, bounded ACP tools, exact working
directory and skill roots. Linux commands remain discoverable through `bash`. Model preference changes
are explicit organisational actions; attempts record actual use independently.

## Acceptance

- A live Exec and two Staff roles expose the exact launch/control package without exposing secrets.
- `command -v`, help and live probes correctly discover `restless`, Git and one project CLI.
- Extensions, ambient rules and unapproved native tools are absent.
- An explicit Sonnet preference survives one forced provider failover and appears again next wake.
- Repo-less Work is described as persistent `/company`; repository Work receives its dedicated
  bound worktree.

## Deletion

Makes per-agent hard-coded model updates, “tool not listed means unavailable” instructions and false
isolated-worktree copy for repo-less Work deletable.
