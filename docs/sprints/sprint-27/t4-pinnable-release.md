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

**Deletion target:** Mutable container tags; version notes maintained by hand; "whatever Core was on
that day".
