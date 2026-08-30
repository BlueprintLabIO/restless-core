# S27-T4 · Publish a pinnable, self-identifying release

**Layer:** Company Runtime + Authority Plane.

**Observed outcome or friction:** Cloud cannot deploy what it cannot pin. Its release contract §3
forbids resolving a mutable tag or a branch name, and Cloud 01 acceptance 3 and 4 are both blocked on
Core publishing something to pin. Today the repository has no CI workflow, no service image, and only
the company Runtime `Dockerfile` — so "which Core is running" has no answer during an incident, and
the answer is needed precisely when nobody can go and look.

**Work:** Build and publish, from a tagged revision:

- an account-plane image and the company Runtime image, by immutable digest;
- one release manifest naming the Core version and source revision, both digests, schema/migration
  versions, the API contract version, the assertion contract version, and the plane/cell compatibility
  range;
- a `/health` probe on the plane and on the cell reporting the release identity it is actually running.

The manifest is the contract Cloud's lock pins and its compatibility probe reads. Keep it a published
artifact, not a document describing one.

**Evidence:** A tagged build produces the manifest and both digests. A plane and a cell deployed from
that manifest report, through `/health`, the same release identity the manifest names — compared
field by field, not eyeballed. Deploying a deliberately mismatched digest is detectable from the
health output alone, verified by doing it.

**Found by building it.** Two defects that no amount of reading would have surfaced:

1. **`dev` does not compile.** `780342a` added `work_id`/`attempt_id` to `StaffRun` without updating
   its caller, and a second half-landed change leaves `interrupt_message` undefined on
   `AgentActivityStreams`. Every `cargo build` and `cargo test` here compiles the *working tree*,
   which carries uncommitted fixes; the image build was the first thing to compile HEAD. This blocks
   a release image, and the fix belongs to whoever owns that in-flight work.
2. **The build context excluded `docs/`.** `crates/restlessd/src/context.rs:21` `include_str!`s
   `docs/COMPANY_OPERATING_RULES.md` at compile time, so the account-plane build failed on a missing
   file. `.dockerignore` is an allowlist and the company image never needed it, because that image
   builds `-p restless`, not `-p restlessd`. Fixed by allowlisting the exact file — `docs/` is
   hundreds of files and none of the rest belongs in a build context.

**Deletion target:** Mutable container tags; version notes maintained by hand; "whatever Core was on
that day".
