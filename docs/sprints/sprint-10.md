# Sprint 10 — The Company computer becomes a prepared working surface

**Status:** Active. Founder-authorised for specification and implementation on 20 August 2026.  
**Date:** 20 August 2026  
**Spec refs:** `ARCHITECTURE.md` §2.1 / §5 / §9.2 / §16,
`owner-cockpit` §2 / §3 / §8 / §12.5 / §14,
`company-runtime` §2 / §3 / §5 / §12 / §14,
`cross-layer-contract` §3 / §5 / §7 / §8,
Sprint 05 T5 and Sprint 09 T3/T4

---

## Observed product gap

Sprint 05 proved the authenticated persistent TigerVNC/noVNC attachment and one-controller lease.
Sprint 09 gave it a stable Company entrance, but its rendered implementation still fails the owner
outcome:

- the entered desktop grid sizes itself to its contents instead of the available cockpit height, so
  a 1280×800 framebuffer is compressed into a shallow strip over an otherwise empty dark canvas;
- every observer and controller URL requests local scaling through copied noVNC query strings in the
  daemon and two owner routes;
- Chromium starts without an explicit maximised state;
- the focus header and explanatory footer consume permanent space while the computer is in use;
- a prepared Attention handoff names its Work but does not keep the exact requested owner action
  visible beside the live browser;
- returning browser control is visually adjacent to resolving the handoff even though it proves no
  external condition and records no judgement;
- Openbox provides window management but no visible task switching, application launcher or ordinary
  file browser, so a second GUI application turns the persistent Runtime into a hidden-window puzzle.

The screenshot does **not** prove that VNC, X11 or Openbox is the failed mechanism. Sprint 05 observed
the same transport filling a 1420×753 focus surface, and the current failure is present in the outer
layout before transport quality can be judged. Replacing the stack with Wayland, PipeWire, a video
encoder and WebRTC would add a remote-desktop integration project without first demonstrating an
owner workload the current path cannot serve.

## Value decision

> **Keep the proved TigerVNC/noVNC transport for Sprint 10. Spend the sprint on a native-sized,
> prepared owner handoff and the smallest coherent persistent desktop. Preserve one server-owned
> transport seam, then replace the stack only after a corrected real run fails an observed company
> workload.**

“Wayland” and “WebRTC” are implementation choices, not product outcomes. Smooth video, high-motion
graphics, audio review, GPU density or unacceptable measured input latency may later justify a
Selkies/pixelflux comparison. Fullscreen composition, exact prepared state, control semantics and
ordinary application switching do not.

## Outcome

The owner enters a calm, full-window Company computer whose desktop consumes all available space.
The global cockpit chrome becomes one bounded overlay control, the Company spine disappears, and the
remote framebuffer is never confined to a content-height strip. Observe-only attachment cannot
resize the shared computer; the sole owner controller may resize it to the live viewport. Chromium
starts maximised beneath a small persistent taskbar. The owner can switch to the company file manager
and back without finding or administering Linux internals.

Prepared Attention handoffs open the same computer at the state chosen and live-probed by the
accountable Staff member. The exact requested action remains visible. **Take control**, **Return
control**, the external human action and any owner judgement remain separate operations. Returning
control tells the responsible actor to inspect the observable resume condition; it never completes
Work, records approval or asserts that the external condition occurred.

## Success contract

The sprint passes only when one rendered `_test` company run demonstrates all of the following:

1. **The entered computer fills the owner window.** At 390, 768 and 1440 CSS-pixel widths, the desktop
   viewport owns the available height after intentional compact controls. There is no unused page-
   sized dark region beneath it and no document scrollbar around it.
2. **Focus is a real threshold.** The ordinary top bar and Company spine disappear only while the
   desktop is entered. A compact computer control surface and, for direct inspection, one Exec toggle
   remain reachable. Leaving restores the prior Company route and the previous Exec-rail state.
3. **The transport remains private and singular.** Attachment still uses the Sprint 05 one-use
   ticket, attachment cookie, private Runtime endpoint and controller lease. The frontend receives
   generic observe/control routes and contains no noVNC asset path, WebSocket path or resize policy.
4. **Geometry follows control authority.** Observe-only sessions use local scaling and cannot change
   the shared framebuffer. The sole valid owner controller requests remote resizing. A competing
   owner remains observer-only and cannot take control or resize the Runtime.
5. **The remote application uses the surface.** Chromium starts maximised and remains fitted after a
   controller viewport change. Reconnect preserves the same persistent profile and useful tabs.
6. **Prepared state remains intelligence-owned.** Staff chooses and live-probes the exact application,
   tab, route, document or dialogue before creating the handoff. The kernel and cockpit do not infer a
   destination through a URL catalogue or fixed application enum.
7. **The owner sees the exact ask.** A prepared desktop handoff keeps its title and `requested_action`
   visible beside the live computer, identifies the responsible actor and preserves the Work-scoped
   conversation.
8. **Control is not completion.** Take control changes only the exclusive input lease. Return control
   changes only that lease and wakes the responsible actor to inspect the source condition. Neither
   action resolves the handoff, accepts a review, records a decision, grants authority or asks the
   owner to self-report completion.
9. **Native review still wins.** A live site, rendered document, player or other outcome-native
   `ReviewTarget` continues to bypass the desktop. Sprint 10 does not make remote desktop the default
   renderer for every result.
10. **The desktop is minimally coherent.** Openbox remains the window manager. A supervised compact
    taskbar provides launch and task switching, Chromium and a file manager are reachable, and the
    company’s persistent downloads/projects/outputs are recognisable. No full GNOME, KDE, Xfce or
    LXQt environment is installed.
11. **Ordinary GUI state persists.** Restarting the replaceable Runtime shell preserves the company
    volume, browser profile, downloads and company files. It does not create a per-attachment desktop
    or disposable home.
12. **Failure is explicit.** A missing Runtime, stale generation, expired attachment, unavailable
    desktop, lost controller lease or unprepared handoff produces a bounded owner-readable state; it
    never becomes an empty black computer or an implied success.
13. **Local development remains the floor.** The corrected computer works through `restless-dev` on
    macOS without a Linux GPU. Hardware video encoding, AV1 and WebRTC are not required for the V0
    owner path.
14. **The old alternatives are purged.** Copied frontend noVNC URLs, `resize=scale` controller paths,
    the content-height focus layout and permanent explanatory desktop footer are removed rather than
    retained beside the corrected path.
15. **A future replacement has an evidence gate.** Sprint evidence records any remaining latency,
    fidelity, audio, high-motion, GPU-economics or visual-control failure. A transport comparison is
    opened only when a named real workload still fails after this sprint.

## Scope and ownership

| Concern                                                      | Owner                      | Sprint 10 change                                                  |
| ------------------------------------------------------------ | -------------------------- | ----------------------------------------------------------------- |
| Work, handoff meaning, requested action and resume condition | OrgIntel                   | Read and present the existing projection; no second handoff state |
| Attach authority, ticket, cookie and controller lease        | Runtime/owner gateway      | Preserve; centralise display-client policy behind generic routes  |
| Files, browser profile, window state and GUI applications    | Company Runtime            | Maximise Chromium and add a minimal supervised desktop shell      |
| Focus composition and explanatory context                    | Owner cockpit              | Full-window threshold and compact source-owned context            |
| Remote display implementation                                | Imported Runtime component | TigerVNC/noVNC remains canonical for this sprint                  |

The cockpit never gains filesystem, shell or generic application-control endpoints. PCManFM is an
ordinary application inside the Runtime. Tint2 and Openbox are ordinary imported desktop components.
The existing browser broker and semantic CDP automation remain the preferred browser actuator;
pixel-level owner control does not replace them.

## Geometry and control contract

```text
observer attachment
  → generic /desktop/:company/observe
  → noVNC local scaling
  → cannot change shared framebuffer

valid sole owner controller
  → generic /desktop/:company/control
  → noVNC remote resizing
  → TigerVNC framebuffer follows that controller viewport

return control
  → generic observer route
  → requesting actor is notified
  → source condition is inspected separately
```

The asymmetry is deliberate. Allowing every observer to request remote resizing would let two owner
tabs fight over the dimensions of one persistent shared computer. Geometry follows the same single
controller boundary as keyboard and pointer input.

## Desktop choice

Sprint 10 retains Openbox and adds only:

- Tint2 as a supervised 30-pixel taskbar and application switcher;
- a launcher for the already-supervised Chromium profile;
- PCManFM for ordinary navigation of persistent company files;
- stable links from the company home to Downloads, Projects and Outputs;
- a quiet Runtime-owned background.

This is enough to prove a multi-application company computer. A full desktop environment remains
deferred until repeated company scenarios demonstrate missing session, accessibility, tray, settings
or application-integration behaviour that these components cannot provide.

## Risks and dispositions

| Risk                                                 | Disposition          | Why                                                                                               |
| ---------------------------------------------------- | -------------------- | ------------------------------------------------------------------------------------------------- |
| X11/noVNC is not the newest stack                    | **Accepted**         | Modernity is not an owner outcome; the current path is proved and replaceable                     |
| Local scaling remains less fluid for observers       | **Accepted**         | Observers must not mutate shared geometry; controller-native sizing carries the interaction value |
| Remote resizing disrupts an agent-controlled desktop | **Guarded**          | Only the current owner controller route enables it; return restores observation semantics         |
| A taskbar or file manager adds unused image weight   | **Pending proof**    | One real Chromium + PCManFM scenario must show whether the multi-app value earns the packages     |
| Wayland/Selkies could outperform VNC                 | **Pending evidence** | Compare only after a corrected run names latency, fidelity, media or GPU friction                 |
| Frontend transport knowledge forks again             | **Invariant**        | noVNC paths and mode policy stay server-owned behind generic routes                               |
| Owner return is mistaken for external completion     | **Invariant**        | Return cannot resolve handoff, judgement, approval or Work state                                  |
| Existing founder changes are overwritten             | **Invariant**        | Sprint edits remain scoped and preserve unrelated dirty-worktree changes                          |

## Tickets

- [ ] [S10-T1 — Make desktop focus immersive and centralise display policy](./sprint-10/t01-immersive-desktop.md)
- [ ] [S10-T2 — Keep the prepared action attached to the live computer](./sprint-10/t02-prepared-handoff.md)
- [ ] [S10-T3 — Add the smallest coherent persistent desktop shell](./sprint-10/t03-minimal-desktop.md)
- [ ] [S10-T4 — Dogfood, compare against the baseline and purge](./sprint-10/t04-dogfood-and-purge.md)

Ticket status lives only in this checklist. A ticket is checked only after its stated evidence has
been observed; code presence alone is insufficient.

## Verification story

Use a `_test` company and `restless-dev <company> --reconcile`; do not exercise simulated capability
against a live company. The final run must record:

1. headless type/build and focused Rust boundary checks;
2. `restless doctor -c <company>` before attachment;
3. authenticated observer attachment and invalid/expired/wrong-controller refusal;
4. rendered viewport dimensions at 390, 768 and 1440 CSS pixels;
5. observer local scaling followed by controller remote resize and return;
6. maximised Chromium, task switching to PCManFM and return to the same browser state;
7. one freshly prepared non-review handoff showing the exact action and responsible actor;
8. return control without handoff resolution, followed by source-observed continuation;
9. one native review target bypassing the desktop;
10. a final transport-value judgement naming any remaining failure and whether it crosses the
    replacement gate.

The final visual pass consults Beautiful UI and Cult UI as the general polish bar, then compares the
live desktop and mobile result with the source-first Origin UI Svelte patterns named in
`docs/FRONTEND_DESIGN_REFERENCES.md`. No second design system or React runtime is imported.
