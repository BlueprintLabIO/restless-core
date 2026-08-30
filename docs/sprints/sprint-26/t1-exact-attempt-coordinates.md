# S26-T1 — Bind Attempts to exact execution coordinates

An Attempt launches only from typed, immutable coordinates that the Runtime can prove it materialised.

**Observed friction:** EXP-15 Product Work repeatedly carried the intended candidate in its brief while
its executable base remained `main` at an older commit. A lead had to notice the mismatch, abandon
useful Work and recreate/reset it manually.

**Layer:** OrgIntel + Runtime.

**Deletion target:** prose-parsed base refs, default-to-`main` launch behaviour and manual lineage
repair.

## Scope

- Make source repository, commit, tree hash, required artifact refs, gate-set version and environment
  profile explicit Attempt inputs.
- Materialisation returns a receipt containing requested and observed identities before any model is
  invoked.
- Refuse a symbolic ref unless it was resolved and frozen when the Attempt was created.
- Bind every produced artifact and gate result back to the Attempt and candidate tree.
- Report mismatch, missing input and unreachable source as distinct terminal facts.

## Acceptance

- Launching commit A into a workspace containing commit B spends no model tokens and reports both.
- Moving a branch after Attempt creation does not change the frozen commit.
- A resumed Attempt verifies the same tree before continuing.
- CLI and cockpit show the exact candidate and input artifact lineage without reading the brief.

