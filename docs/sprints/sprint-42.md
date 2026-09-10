# Sprint 42 — Give installed applications governed runtime roles

**Status:** Planned; activates after Sprints 40 and 41 pass their vertical paths  
**Date:** 4 September 2026  
**Programme:** [Open Company Runtime](./open-company-runtime-programme.md)  
**Depends on:** Two observed harness/client implementations, the Company Runtime's existing installed
tools and persistent state, and Sprint 38's exact launch/update ownership where available.

## Why this sprint exists

After Codex and Claude exercise native takeover, Restless will have evidence that one installed binary
may serve several roles: autonomous harness, owner-facing client, ordinary tool or background service.
Encoding each as a special product type would duplicate lifecycle, readiness and security behavior.

The useful abstraction is smaller: installed software has an exact observable identity; Restless
separately admits particular roles for a build and company scope. Installation alone grants no
autonomous authority.

## Outcome

The Company Runtime can inventory and manage a small set of applications with exact build,
provenance, entrypoints, state roots, readiness and admitted roles. The owner sees relevant installed
applications through the existing Company Resources and Company Computer surfaces, can request one
bounded install/update/remove operation and can tell whether an application is certified autonomous,
governed interactive, personal interactive or unavailable.

The counted corpus proves a multi-role agent application, an ordinary CLI tool, a graphical/runtime
application and a company-built tool without introducing an app marketplace or arbitrary executable
launch API.

## Frozen product decisions

1. **Application and role are separate.** A binary is not a harness merely because it can speak ACP or
   call a model.
2. **Roles are closed and semantic.** Initial roles are harness, tool, interactive client and service;
   arbitrary strings and user-authored authority semantics are refused.
3. **Certification binds an exact build and entrypoint.** Package name or executable discovery alone is
   insufficient.
4. **Company installation does not widen host authority.** Downloads, licences, credentials and
   consequential setup cross existing owner/Authority paths where applicable.
5. **State ownership is explicit.** Image-owned, company-persistent, project-local and ephemeral state
   are distinguishable before update or removal.
6. **No ambient execution.** Installing software does not add it to actor prompts, PATH allowlists,
   MCP, desktop launchers or autonomous policy without a separate admitted role.
7. **Applications remain Runtime truth.** OrgIntel stores references and readiness projections, not a
   package-manager mirror or file inventory.
8. **Owner surfaces stay contextual.** Resources explains readiness and lifecycle; Company Computer
   opens usable applications. There is no storefront grid.
9. **Company-built tools are first-class.** They can be observed and admitted without pretending to be
   globally certified vendor software.
10. **Unknown remains unknown.** Restless does not infer licence, security, version or cleanup safety
    from a friendly package name.

## Success contract

1. The minimal application record names stable id, display name, exact build/digest, source,
   entrypoints, platform, state roots, readiness probe, admitted roles and trust classification.
2. Codex is represented once while retaining separate certified harness and interactive-client roles;
   Claude records the actually proved native relationship from Sprint 41.
3. One ordinary tool remains usable without harness lifecycle or agent-session fields.
4. One GUI/runtime application opens through its existing bounded launch path and owns persistent state
   without receiving autonomous authority.
5. One company-built tool can be promoted, revised and retired within that company without entering a
   global registry.
6. Install, update, failed update, rollback, disable, remove and state-retention choices are exact,
   recoverable and observable.
7. Arbitrary paths, package names, registry URLs, entrypoint arguments and readiness scripts are
   refused at owner APIs and durable configuration boundaries.
8. Personal configuration, connectors and plugins remain outside certified autonomous roles unless
   separately admitted by exact policy.
9. Removing a role does not uninstall or destroy application data; uninstall does not rewrite
   historical Work that used the application.
10. Runtime replacement reconstructs image-owned applications and preserves or explicitly migrates
    company-owned state.
11. Cleanup proves absence only for exact owned paths/processes and never performs broad package,
    filesystem or container deletion.
12. The owner can answer what is installed, whether it is ready, which roles it may play and what will
    happen to its state before an update or removal.

## Slice per layer

**Authority and host.** Govern software acquisition only where it crosses payment, licence, signature,
host installation or secret boundaries. Preserve exact launch/update receipts where consequential.

**OrgIntel.** Refer to applications used by Work and retain company-local promotion or retirement
decisions. Do not own package files or invent application authority.

**Runtime.** Observe builds, entrypoints, state roots, processes and readiness; execute bounded
install/update/rollback/remove plans from certified or company-authored sources.

**Owner UI.** Extend Resources and Company Computer with contextual application identity, role,
readiness and lifecycle actions without becoming a marketplace.

## Salvage

- Reuse Sprint 39's exact harness build/readiness ledger as one input, not as the whole application
  model.
- Reuse Sprint 38's image identity, native launch, upgrade and owned-cleanup contracts after T0 maps
  their state assumptions to application scope.
- Revalidate existing browser, Godot and company-created skill/tool launchers against the closed role
  model; do not bulk-register every executable on PATH.

## Out of scope

- public or third-party application marketplace;
- search, ratings, recommendations or revenue share;
- arbitrary package-manager access from owner APIs;
- automatic certification of ACP agents, MCP servers or desktop apps;
- universal licence or vulnerability scanning;
- a cross-company global application state store;
- changing the canonical Work or artifact lifecycle; and
- full vendor desktop acceptance, which remains Sprint 43's evidence-gated question.

## Stop rules

Stop if the model grows into a generic package manager, if every executable needs a durable database
row, if role admission becomes arbitrary policy code, or if installation is treated as permission to
act autonomously. Preserve provider-specific profiles until at least two real applications share each
generalised field.

## Ticket index

| Status | Ticket | Outcome |
| --- | --- | --- |
| [ ] | [S42-T0](./sprint-42/t0-freeze-application-corpus.md) | Freeze the smallest mixed application and lifecycle corpus |
| [ ] | [S42-T1](./sprint-42/t1-application-identity-and-roles.md) | Separate exact installed builds from admitted roles |
| [ ] | [S42-T2](./sprint-42/t2-install-update-rollback.md) | Add bounded persistent lifecycle and exact ownership |
| [ ] | [S42-T3](./sprint-42/t3-trust-authority-and-state.md) | Keep credentials, personal capability and state boundaries honest |
| [ ] | [S42-T4](./sprint-42/t4-resources-and-computer-surface.md) | Show and open applications contextually without a storefront |
| [ ] | [S42-T5](./sprint-42/t5-mixed-role-dogfood-and-purge.md) | Prove four application shapes and delete special-case duplication |

Expected order: **T0 → T1 → T2/T3 → T4 → T5**.

## Terminal decision

- **Pass:** the small role model removes real duplication while preserving application-specific
  capability and state truth.
- **Revise once:** narrow fields or lifecycle after one mixed-corpus defect.
- **Stop negative:** keep static certified profiles and ordinary installed tools if a shared registry
  adds more ontology or owner burden than operational value.
