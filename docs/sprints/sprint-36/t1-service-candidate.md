# S36-T1 — Produce one immutable service candidate

**Layer:** Runtime and OrgIntel

**Observed outcome or friction:** Runtime paths, open development ports and mutable image tags cannot
identify what an external user actually experienced.

**Work:** Build a service candidate from an exact source/artifact lineage into an immutable OCI digest
and a released-profile manifest. Run the profile's local readiness and protocol probe before recording
the candidate. Refuse undeclared ports, invalid entrypoints, mutable references and mismatched content.

**Evidence:** Both frozen profiles yield exact candidate identities; any source or manifest change
changes the digest; every negative corpus candidate is rejected before provider activity.

**Deletion target:** Publishing arbitrary Runtime processes, directories, `latest` images or whatever
currently listens on a port.
