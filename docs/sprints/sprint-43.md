# Sprint 43 — Admit an optional graphical owner workstation

**Status:** Activation-gated; not authorised until T0 passes  
**Date:** 4 September 2026  
**Programme:** [Open Company Runtime](./open-company-runtime-programme.md)  
**Depends on:** Sprint 42's application identity/roles and at least three recorded high-value owner
interventions that remain materially worse through terminal, editor/preview, Company Computer and
host/SSH native-client paths.

## Why this sprint may exist

Codex and Claude desktop applications provide rich native inspection, parallel sessions, diffs,
previews, connectors and OS integrations. Running one inside or against the Company Runtime could make
specialist intervention much better. It could also duplicate credentials, transcripts, configuration,
worktrees, updates and permissions while adding a large nested desktop surface.

This sprint exists as a falsifiable option, not a foregone roadmap commitment. T0 must show a repeated
owner job whose value exceeds those costs and choose the smallest viable placement: host-native client
connected remotely, vendor client inside the Company Runtime, or the existing streamed Company
Computer.

## Outcome

If activated, one exact vendor graphical application opens an owner-operated session against the
correct company workspace through the application and takeover contracts. Its authentication,
configuration, persistent state, updates and external connections are explicitly bounded. Returning
control reconciles the Work exactly as terminal takeover does.

If the activation evidence fails, the sprint closes negative with a recorded decision to keep vendor
desktops outside the Runtime image. That is a successful result; no placeholder GUI or dormant package
is shipped.

## Activation gate

Before implementation, T0 must provide:

1. at least three real interventions across two outcomes where current native terminal, editor,
   preview, browser/desktop takeover or remote/SSH client access caused material repeated friction;
2. the exact missing vendor-UI capability and why a small Restless improvement would not solve it;
3. supported platform, installation, redistribution, authentication, update and enterprise-policy
   evidence for the candidate build;
4. a comparison of host-native remote access versus installation inside Runtime;
5. expected persistent state and cleanup ownership;
6. a credential and external-connection threat model; and
7. a founder decision to activate one candidate and placement.

No ticket after T0 begins without that recorded activation.

## Frozen product decisions if activated

1. **Optional, never base-image required.** A vendor desktop is a company/owner workstation choice.
2. **Owner-operated by default.** Installation does not certify the GUI as an unattended harness.
3. **Use the smallest placement.** Prefer a supported host/SSH/remote client when it preserves the
   workspace and session without duplicating Runtime credentials or state.
4. **One exact candidate.** The sprint does not attempt both vendor desktops, every OS or generic GUI
   application support.
5. **Vendor UI remains vendor UI.** Restless launches or connects it; it does not automate pixels to
   pretend there is a stable harness protocol.
6. **Separate histories stay separate.** Provider application history is linked as session state, not
   imported wholesale as canonical Work.
7. **Personal extensions are visible.** Plugins, connectors, MCP and remembered approvals are labelled
   personal interactive capability and cannot silently enter autonomous mode.
8. **Authentication respects placement.** Host-side login stays host-side; Runtime installation may
   not receive a reusable provider root or subscription credential without a separately approved
   secure deployment contract.
9. **Application updates are explicit.** Vendor auto-update cannot silently invalidate a certified
   client/session contract.
10. **Return remains mandatory for company settlement.** Closing the window does not resume or complete
    Work automatically.

## Success contract if activated

1. The chosen application and placement resolve all three activation cases materially better than the
   earlier native-access path.
2. One bounded Open action selects the correct company, Work, workspace and supported session
   relationship without copied path, port or reusable token.
3. The owner can distinguish host-native, Runtime-native and streamed execution and knows which
   machine/config/account is active.
4. Application state survives intended restart/update boundaries and is deleted or retained exactly
   according to its declared ownership.
5. Native work cannot race the autonomous actor; takeover and uncertain/disconnected states reuse the
   same controller contract.
6. Personal plugins/connectors are enumerated where possible and remain outside certified autonomous
   policy.
7. External actions still produce provider or Authority evidence; opaque UI effects remain explicitly
   unverified until reconciled.
8. Wrong company, actor, workspace, account, version, platform or expired handle fails before useful
   access.
9. Update failure, application crash, host disconnect and Runtime replacement each retain the
   checkpoint and converge on a truthful return state.
10. No credential, installer, update helper, process, socket, mount, clipboard payload or temporary
    launch material remains outside declared persistent state after removal.
11. Image size, startup time, idle resources and maintenance burden remain within the T0 bounds.
12. The final decision records **promote optional**, **host-only**, **retain experimental** or **remove**.

## Slice per layer

**Authority and host.** Own host-side launch/auth exchange, signed installation where needed and
consequential effect boundaries. Do not proxy arbitrary GUI input as an Authority command language.

**OrgIntel.** Link the owner-operated application session to existing Work/control state and retain the
return disposition only.

**Runtime.** Host or expose the declared workspace/application state, supervise only owned processes
and preserve exact app lifecycle. Do not scrape pixels into canonical activity.

**Owner surface.** Use the existing Open, Company Computer and Take control vocabulary. Show placement,
account boundary, trust tier and native-session relationship before launch.

## Salvage

- Reuse Sprints 40–42 launch handles, controller state, application identity and reconciliation.
- Reuse existing Company Computer streaming only as an explicit placement, not as proof that a vendor
  application is product-usable.
- Revalidate vendor-provided packages and terms at implementation time; do not redistribute an
  installer based on this planning document.

## Out of scope

- a general remote desktop platform;
- bundling both Codex and Claude desktops;
- pixel automation as a stable agent integration;
- automatically importing personal accounts, plugins or histories;
- arbitrary host filesystem mounts;
- public app discovery or installation; and
- making graphical desktop availability a prerequisite for Restless autonomy.

## Stop rules

Close negative if the activation corpus is weak, the candidate cannot be distributed or securely
authenticated, host-native access is clearly sufficient, the UI requires a second company canon, or
maintenance/resource cost exceeds the measured intervention benefit.

## Ticket index

| Status | Ticket | Outcome |
| --- | --- | --- |
| [ ] | [S43-T0](./sprint-43/t0-activation-and-placement.md) | Decide whether a vendor desktop and which placement are earned |
| [ ] | [S43-T1](./sprint-43/t1-workstation-boundary.md) | Freeze one optional workstation lifecycle and trust boundary |
| [ ] | [S43-T2](./sprint-43/t2-admit-one-vendor-client.md) | Install or connect one exact supported graphical client |
| [ ] | [S43-T3](./sprint-43/t3-auth-state-and-updates.md) | Bound credentials, personal config, persistent state and upgrades |
| [ ] | [S43-T4](./sprint-43/t4-open-control-and-return.md) | Reuse Open, Take control and reconciliation without a second UI canon |
| [ ] | [S43-T5](./sprint-43/t5-native-workstation-dogfood.md) | Resolve the activation cases and promote, narrow or remove the client |

Expected order if activated: **T0 → T1 → T2/T3 → T4 → T5**.

## Terminal decision

- **Close negative:** existing native terminal/remote paths are sufficient; ship no vendor desktop.
- **Promote optional:** one client materially improves repeated work and passes lifecycle/security.
- **Host-only:** the client is useful but should connect from the owner's machine rather than live in
  Runtime.
- **Remove:** the experiment fails usability, isolation, maintenance or return reconciliation.
