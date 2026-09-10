# S42-T1 — Separate application identity from admitted roles

**Layer:** Runtime Bridge + OrgIntel projection  
**Serves:** One installed build may be a harness, client, tool or service without four duplicate identities.

## Work

- Add the smallest stable application/build identity required by the frozen corpus.
- Record source/provenance, exact version or digest, platform, entrypoints, state roots and readiness
  without turning package metadata into trusted claims.
- Bind closed harness, interactive-client, tool and service roles separately to exact builds and scope.
- Preserve historical Work/session build identity after role change, update or uninstall.
- Reject unknown roles, arbitrary commands and implicit role admission from installation or discovery.

## Acceptance

Codex has one build identity with two independently revocable roles; ordinary tools carry no harness
session contract. Unsupported or stale roles fail before launch and history never rewrites to the
current installed version.

## Makes deletable

Duplicated per-surface build fields and the assumption that package presence grants a role.
