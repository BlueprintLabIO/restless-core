# Experiment Sprint 12 - Exec-installed connected tools

**Status:** Complete; the founder accepted the native Aris Academy test outcome and disclosed
ownership representation. The repaired mechanism worked, but the autonomous-installation treatment
failed its no-developer-access gate. No outreach is authorised.

**Decision owner:** Founder

**Date:** 29 August 2026

**Depends on:** The current Runtime Bridge MCP attachment seam, the owner provider-authentication
handoff contract, and one real external capability worth installing.

## Decision this sprint must produce

Determine whether a Restless Exec can discover, connect, verify, install and make a useful external
MCP capability available to the right Staff actor without the owner editing configuration, running a
CLI, copying credentials or understanding MCP.

This is the product hypothesis: a non-technical owner should be able to say "set up our CRM" or
"connect this tool" and receive a prepared identity/consent last mile while Exec completes everything
machine-doable.

Choosing Attio instead of Pipedrive is not an experiment question. It is a reversible procurement and
product-fit decision. Attio is the first implementation candidate because it has an official remote
MCP and fits the immediate Aris sales workload. Use Pipedrive only if Attio fails the native outcome,
creates material owner friction or the founder rejects its working experience.

## Central hypothesis

> A provider-neutral connected-tool seam plus a prepared owner authentication handoff lets Exec add a
> mature external capability as ordinary company work: Exec discovers and probes the official tool,
> explains the requested access, prepares the exact consent surface, observes completion, verifies the
> connected account and scopes, attaches the MCP only to relevant actor sessions and proves a native
> outcome. The owner supplies identity and consent, not technical integration labour.

The credible failure is that "install MCP" remains a developer operation hidden behind owner setup:
editing JSON, selecting transport details, creating secrets, copying tokens, restarting a harness or
reporting that OAuth completed. Another credible failure is that provider records leak into Core and
create a second, stale domain application.

## Canonical boundary under test

```text
Exec installation Work
  -> live discovery and capability probe
  -> provider-neutral Core connection request
  -> prepared owner OAuth/account-consent handoff when identity is irreducible
  -> observed provider connection
  -> already-authorised MCP attachment in the Runtime Bridge launch contract
  -> Staff proves a useful native outcome
  -> external provider remains source truth
```

Hosted Cloud may use Nango to implement connection mechanics. Local Core may connect directly to a
standards-compatible official remote MCP. Both paths must resolve to the same Core-owned connection
and attachment contract; neither may introduce provider-specific application semantics into Core.

## What "installed by Exec" means

For a supported remote MCP, Exec must be able to complete every machine-doable step:

1. discover the provider's official MCP endpoint or supported API from live provider evidence;
2. inspect transport, authentication and tool metadata without guessing;
3. choose the connection for a named company purpose and request only the useful scope;
4. prepare one owner Attention handoff at the provider-hosted login/consent surface;
5. observe the callback or provider connection state without asking the owner to report completion;
6. verify the exact provider account/workspace, scopes and required tools with a live probe;
7. attach the connection to one relevant Exec or Staff session through the Runtime Bridge;
8. run the native success scenario, retain provider record links and report the observed outcome;
9. recover an expired/revoked connection by preparing the reconnect last mile; and
10. disable or remove the connection when it no longer helps.

The owner may be required to sign in, pass MFA, choose an account and consent. The owner must not need
to edit an MCP manifest, create a local config file, handle a client secret, run a command, restart a
service or translate provider scopes.

Package installation and remote-account connection are distinct. An official remote MCP normally
requires no package in the Company Runtime; "install" means make the verified connection available to
the selected actor. A local MCP package, if later needed, remains ordinary Exec-commissioned Runtime
work plus the same capability probe and attachment contract.

## Experiment design

Use Attio's official remote MCP and a fresh `_test` workspace. Freeze one owner request:

> Set up a CRM for our tutoring-centre sales team, load the supplied unsent prospect set and give the
> sales lead a useful pipeline with ownership and next actions.

Compare the current technical path with the intended product path:

```text
Baseline M - technical installation
  a technical operator performs the currently necessary MCP configuration and connection work

Treatment E - Exec installation
  the owner issues the plain-language request once
  Exec performs every machine-doable step and brings only provider login/consent to the owner
```

Both paths use fresh `_test` workspaces, the same model/runtime, scopes, prospect fixture and native
CRM workload. Baseline M records the technical burden to eliminate; it is not another installation
system to retain.

Treatment E must:

1. install and live-probe the connection;
2. upsert the supplied tutoring-centre accounts using domain and public email identity;
3. retain source URL, qualification evidence, tier, owner, status, next action and due date;
4. return qualified accounts with no completed contact and an action due this week;
5. record controlled positive and not-interested outcomes without sending anything;
6. detect an attempted duplicate account/contact;
7. produce a pipeline summary with direct links to native records; and
8. prepare the native CRM as the founder ReviewTarget.

Stage execution stops if the connection cannot be installed without technical owner work, the wrong
workspace is reached, required tools are unavailable, or any path attempts external outreach.

## Nango placement and validation

Nango is not a second experiment arm and is not part of the Core product dependency set.

- **Core owns** the provider-neutral connection reference, Authority permission, live capability
  observation, MCP attachment contract and effect boundary.
- **Cloud owns** the optional Nango adapter, tenant/company mapping and hosted connection operations.
- **Nango owns** replaceable OAuth/token refresh and provider-call mechanics when selected.
- **The external application owns** its actual domain records and state.

After the direct official MCP passes locally, Cloud should run one implementation probe through its
already-selected Nango trial adapter. The probe asks only whether the same Exec-owned installation
experience and Core attachment contract survive hosted OAuth, reconnect and tenant mapping. It is not
a direct-versus-Nango bake-off.

Retain Nango when it materially reduces hosted connection plumbing or improves reconnect/diagnosis.
Remove it when the official MCP path already supplies those outcomes. Either result leaves Nango
outside Core and replaceable. Local Restless must not require a Cloud/Nango account merely to connect
a standards-compatible official MCP.

## Native success contract

- One plain-language owner request starts installation.
- Exec independently finds and validates the official connection path.
- Attention contains one bounded provider-hosted identity/consent action and the reason/scopes.
- Restless observes completion and resumes without an owner "done" message.
- A fresh Staff session receives exactly the selected MCP and passes workspace identity, read and
  recoverable write probes against the intended `_test` workspace.
- The supplied prospects are visible in a useful native CRM pipeline with source evidence, ownership
  and next actions.
- Duplicate input does not create a second account/contact.
- Removing or revoking the connection prevents its use in the next fresh session.
- The provider remains the only writer of its domain truth; Core stores identifiers, observations and
  Work/effect references only.
- No outreach message is sent and no test result enters the live Aris company.

## Measurements

- active owner minutes, clicks and technical operations;
- active Exec and Staff time, model usage and intervention count;
- time from plain-language request to first verified useful tool call;
- installation and reconnect completion without developer access;
- live probe success for workspace identity, scopes and required tools;
- MCP tool-call success, repair loops, rate-limit failures and ambiguous errors;
- native workload completion, duplicate rate and query correctness;
- native founder acceptance and time required to understand the result;
- amount and type of provider-specific Core code introduced;
- hosted Nango probe setup, reconnect, diagnosis and tool-fidelity observations; and
- local sales-state machinery made removable by the external application.

## Decision rule

Promote the generic connected-tool slice only if Exec completes installation and recovery with no
owner technical work, the exact connection is live-probed, the native workload is useful and
undeclared MCPs remain absent from other actor sessions.

If the Attio workload is poor but installation succeeds, record the connected-tool hypothesis as a
win and treat CRM replacement as an ordinary procurement decision. Probe Pipedrive only then; do not
rerun the installation experiment merely because the selected provider changes.

If installation fails, preserve the exact friction and repair only the observed missing seam. Do not
respond by building a provider catalogue, fixed installation wizard, CRM schema or universal
integration layer.

Nango remains a Cloud adapter only when the hosted probe shows a concrete operational benefit. Its
presence or absence does not change the Core experiment result.

## Risks and dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| "Exec-installed" still hides owner CLI/config labour | **Invariant for promotion** | Count every owner action; any technical operation fails the treatment |
| Test CRM activity is mistaken for live Aris demand | **Invariant** | Fresh `_test` workspace, marked fixture and zero live-company projection |
| Tool installation accidentally enables outreach | **Invariant** | Do not expose or exercise send/sequence-enrolment tools; the fixture remains unsent |
| Provider-specific concepts enter Core | **Invariant** | Review the diff and schemas; Core may retain only generic connection/effect references |
| Wrong provider workspace is connected | **Guarded** | Live identity/workspace probe before productive use and after reconnect |
| Nango becomes an unnecessary mandatory hop | **Guarded** | Direct official MCP remains the local path; Cloud retains Nango only on observed benefit |
| Attio is not the best CRM | **Accepted** | It is replaceable; use Pipedrive only if native friction or founder judgement warrants it |
| One-owner OAuth identity is insufficient for later multiplayer | **Pending fix** | Revisit only after a second human/company connection creates real attribution friction |
| Vendor lock-in during the test | **Accepted** | Use a disposable workspace and require a clean export |

## Required evidence bundle

1. frozen workload, provider endpoint/version, requested scopes and actor connection assignment;
2. owner-action and technical-action logs for Baseline M and Treatment E;
3. connection identity, capability and fresh-session attachment probe results;
4. native CRM ReviewTarget with the complete equivalent test fixture;
5. provider MCP transcript and failure/recovery observations;
6. one hosted Nango implementation probe if the direct local path passes;
7. founder acceptance/rejection of the connected-tool experience;
8. exact Core/Cloud changes promoted, rejected or made deletable; and
9. a terminal report distinguishing observed provider behaviour from provider claims.

## Stop boundary

This plan records an experiment and authorises nothing by itself. A founder-approved execution may
create only a disposable provider workspace/connection and `_test` company evidence within its named
budget. It does not authorise importing records into live Aris, sending email or LinkedIn messages,
enrolling a prospect in a sequence, purchasing a provider plan, changing the live sales pipeline,
installing a permanent provider catalogue, or promoting Nango/Core machinery to production.

## Observed amendment and current disposition — 29 August 2026

The founder explicitly replaced the fresh Attio workspace condition with a bounded Aris Academy test:
only unmistakably labelled new EXP-12 records, no alteration of pre-existing records and no outreach,
drafts, sequences or purchase. The Company Runtime remained `exp12_attio_test`. This deviation is
recorded rather than retrofitted into the original design.

The productive run created one isolated native list, 20 new labelled People records and 20 linked
incomplete tasks, reconciled all three excluded addresses as absent and observed zero post-create
email activity. The official MCP exposed 39 tools. The exact ReviewTarget and native transcript
evidence are recorded in `coordination/experiments/EXP-12/run-log.md`.

The treatment is a provisional loss despite the useful provider result. Duplicate OAuth, stale owner
handoffs, actor-wide rather than Work-scoped attachment and destructive reconnect required technical
repair and manual recovery of the already-authorised credential. The repaired implementation passed
focused tests and a live disable/fresh-session probe, but the original one-request/no-developer-access
claim was not observed. The hosted-Nango condition is therefore not activated. The founder accepted
the exact native list and disclosed ownership representation through handoff
`99f9d552-88d0-47e5-8ed5-e09f31d52ece`; that usefulness judgement does not change the treatment
result. The stable judgement is recorded in
`coordination/experiments/EXP-12/FOUNDER_REVIEW.md`.
