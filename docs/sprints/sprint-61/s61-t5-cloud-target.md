# S61-T5 — Cloud target

**Layer:** Cloud, with a paired `restless-cloud` ticket. **Serves:** acceptance 3.

`--target cloud` provisions a `_test` company on a Cloud test plane, serves the fixture at a
public TLS address the plane can reach, and enters the cockpit through `app.restless.run` with a
test owner. It then runs the same assertions as Core, except 15 until it lands. Anything that cannot
run on Cloud is recorded as skipped with the reason, never omitted. Teardown removes the company,
its Runtime and the fixture service.

Depends on T1's fixes. Founders decide between a permanent test plane and one per run (open
question 2).

**Deletes:** nothing.
