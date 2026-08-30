# S26-T2 — Materialise hermetic actor workspaces

Every Attempt receives a writable candidate tree while generated caches and transient evidence live
outside Git custody.

**Observed friction:** Godot created `.godot/` in a shared product repository, making integration
refuse a dirty tree. A copied source tree retained uid 501/root ownership while the actor ran as uid
2000, so promotion failed despite correct product work.

**Layer:** Runtime.

**Deletion target:** repo-local engine/model caches, shared mutable product worktrees and privileged
ownership repair.

## Scope

- Materialise one Attempt workspace from the frozen tree and normalise it to the actor's uid/gid.
- Mount or configure build, engine, model and capture caches outside the repository.
- Declare writable output paths and reject writes outside the grant.
- Record clean-tree state before and after every gate and promotion.
- Clean transient workspace/capture state at terminal completion while retaining compact evidence and
  source checkpoints.

## Acceptance

- A Godot import/build/play run leaves the Git tree clean without relying on a newly added ignore rule.
- A mixed-owner fixture is writable by the assigned actor and promotes without root intervention.
- Two Attempts from the same commit cannot mutate one another's workspace or caches except through an
  explicitly shared read-only cache.
- Crash cleanup removes transient captures and preserves the candidate checkpoint and compact receipt.

