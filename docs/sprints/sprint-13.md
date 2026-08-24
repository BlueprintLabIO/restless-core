# Sprint 13 — Archived TypeScript feasibility branch

**Status:** Not started; superseded by Sprint 14 on 24 August 2026.

## Decision

The proposed TypeScript ecosystem comparison does not proceed.

The investigation answered the narrower technical question without product code:

- durable OrgIntel would duplicate Rust-owned migrations, atomic claims, recovery and its Postgres
  authority;
- Rust already has mature OAuth/OIDC libraries, so an OIDC adapter is not a differentiated enough
  TypeScript seam to justify an additional trusted runtime;
- the next useful pre-release investment is Rust safety/evidence hardening and modularisation, now
  specified by [Sprint 14](sprint-14.md).

TypeScript remains appropriate for the cockpit and optional Runtime tooling where a concrete
ecosystem advantage is observed. It is not an alternative owner of OrgIntel or Authority.

## Preserved evidence

- [The original EXP-02 entry-gate audit](sprint-13/entry-gate-audit.md) remains the product evidence
  that no coordination affordance earned a TypeScript implementation.
- Sprint 14 retains the language-boundary decision and deletes the unstarted OIDC test scaffold.

No TypeScript process, database access, Authority integration, model gateway integration, provider
connection or production code was added by this sprint.
