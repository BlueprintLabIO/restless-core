# S27-T2 · Derive company scope from the assertion

**Layer:** Authority Plane.

**Observed outcome or friction:** One plane serves every company its owner holds, and — once Cloud
multiplayer exists — humans who may see only one of them. Scope taken from the request URL is scope
the client chooses. The existing owner API is company-scoped by path (`/companies/{company}/...`,
`owner.rs:740`), which is correct addressing and worthless as authorisation.

This is the boundary Cloud 03's entire multiplayer story rests on, and the one an attacker reaches by
editing a URL.

**Work:** Resolve company scope from the verified assertion on **every** request, not once at entry.
A request whose path names a company outside the assertion's scope is refused, regardless of the
hostname it arrived at, any forwarding header, or a previously established session for a different
company.

Keep the derivation in one place. Two call sites that each decide scope is how one of them ends up
deciding it differently.

**Evidence:** An assertion scoped to company A, presented against `/companies/B/...` on the same
plane, is refused — checked against a plane that genuinely holds both companies, so that a pass proves
scoping rather than absence. The same request with an assertion scoped to B succeeds, which is what
makes the refusal meaningful. An owner-scoped assertion reaches every company on its plane.

**Deletion target:** Route- and host-derived company scope; the assumption that path addressing is
authorisation.
