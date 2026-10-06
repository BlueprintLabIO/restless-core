# S61-T6 — Gate promotion and release on the Core smoke

**Layer:** Operations. **Serves:** acceptance 4 and 5.

- `restless-dev promote` runs `connections-smoke --target core` against the image it just built,
  before `restless appliance upgrade`. Whether it blocks or warns is open question 1.
- The release workflow runs it on the release candidate.
- `docs/operations/connections-smoke.md` records how to run it, how long it takes, what it costs,
  and how to read the evidence.
- Sprint 57's acceptance items 3 (local stdio), 4 (plugin import) and 7 (deletion) cite the smoke's
  evidence instead of a hand run.

**Deletes:** nothing.
