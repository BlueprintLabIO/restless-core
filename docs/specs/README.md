# Restless specifications

[ARCHITECTURE.md](../../ARCHITECTURE.md) is the cross-plane authority. The specifications below add
implementation detail; they do not compete with it. When an implementation, a specification and a
real run disagree, investigate the run first and reconcile the documents deliberately.

| Work touches | Read |
| --- | --- |
| authority, secrets, effects, budgets or lifecycle | [Authority Plane](authority-plane.md) |
| actors, Work, messages, wakes or organisation | [OrgIntel](orgintel.md) |
| company computer, processes, browser or Runtime Bridge | [Company Runtime](company-runtime.md) |
| an owner-facing cockpit surface | [Owner Cockpit](owner-cockpit.md) |
| bootstrap, identifiers or cross-plane ownership | [Cross-layer Contract](cross-layer-contract.md) |
| proof, dogfood or evidence | [Evaluation and Dogfood](evaluation-dogfood.md) |
| human Actors, Rooms, native Docs, mobile sync or recipient-relative judgement | [OrgIntel](orgintel.md), [Owner Cockpit](owner-cockpit.md), [Cross-layer Contract](cross-layer-contract.md), and the [company collaboration programme](../sprints/company-collaboration-programme.md) |
| the public Restless surface or a future managed Cloud deployment | [Core/Cloud boundary pointer](restless-cloud.md), [cell architecture](../CELL_ARCHITECTURE.md) and the [Cloud specification set](https://github.com/BlueprintLabIO/restless-cloud/tree/main/docs/specs) |

## Reconciled Cloud material

[Core/Cloud boundary pointer](restless-cloud.md) records the Core-owned side of the split. The detailed
Cloud architecture, Fleet/cell/multiplayer contracts and public-surface roadmap live only in the
separate Cloud repository.

The accepted collaboration seam decisions are recorded in
[ADR 0008](../adr/0008-activate-bounded-company-collaboration.md),
[ADR 0009](../adr/0009-provider-neutral-company-access-context.md),
[ADR 0010](../adr/0010-native-docs-are-a-core-subsystem.md), and
[ADR 0011](../adr/0011-transactional-outbox-for-client-delivery.md). Detailed implementation is
dependency-ordered in the collaboration programme rather than installed as a second peer architecture.

[The v1.2 archive reconciliation](https://github.com/BlueprintLabIO/restless-cloud/blob/main/docs/migration/v1.2-reconciliation.md)
records how the unpacked `restless_specs_latest_v1.2` archive was compared with the current canon,
including material deliberately deferred or rejected.
