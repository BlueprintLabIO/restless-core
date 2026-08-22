# Sprint 10 run report

**Run date:** 20 August 2026
**Synthetic company:** `sprint08_ui_test`
**Status:** All technical acceptance evidence complete; separate owner outcome review pending
**Prepared human-step handoff:** `03020472-da7f-4912-b1cf-0383dbe964a4`

## Decision reached by evidence

Sprint 10 keeps Openbox, TigerVNC and noVNC. The corrected `_test` Runtime produced a full
1280×770 Chromium work area plus a 30-pixel taskbar, opened PCManFM against persistent company
files and returned to the same Chromium process and profile. Playwright then drove the real owner
surface, resized the X framebuffer through noVNC, switched applications and exercised the exclusive
lease. No observed latency, fidelity, audio, high-motion or GPU-density workload failed. There is
therefore no value evidence in this run for adding Wayland, PipeWire capture, a video encoder and
WebRTC.

This is not a claim that the old transport is permanently superior. It is the smaller decision:
the screenshot's dead space came from cockpit geometry, and the first multi-application failure came
from the desktop shell. Both were corrected without changing transport. A Wayland/Selkies comparison
remains gated on a named workload that fails after the rendered pass below.

## Implemented slice

- The owner gateway owns the noVNC client policy behind generic observe/control routes. Observers use
  local scaling and the sole controller uses remote resize; the frontend no longer contains the
  noVNC asset, WebSocket or resize query.
- Direct Company computer focus and prepared Attention focus use URL state and an immersive height
  chain. Ordinary cockpit chrome is removed at the threshold, with bounded computer/Exec controls.
  The rendered pass found that the optional error row caused the desktop to auto-place into the
  browser's default 150-pixel iframe row; explicit toolbar/error/desktop row ownership removed the
  original dead space.
- A prepared handoff keeps its exact `requested_action`, responsible actor and Work-scoped
  conversation on the live-computer surface. Take control and Return control explain their
  lease-only effect.
- A fresh browser-status observation now wins immediately over an older Runtime summary after Return,
  so the toolbar settles on `Ready for control` instead of briefly repeating stale controller state.
- The Runtime image keeps Openbox and adds supervised Tint2, PCManFM, a Browser launcher that focuses
  the existing supervised Chromium, and non-destructive Home links for Downloads, Projects and
  Outputs. Chromium requests a maximised start.

No GNOME, KDE, Xfce, LXQt, second desktop lifecycle, application catalogue or fixed prepared-state
enum was introduced.

## Runtime evidence

The final image built as `restless-company-image:latest` with image ID
`sha256:7e2615863df18287276823a625aeb4e24af976534dfecf3c14c68c0bd13c6046` and source digest
`4e3577f2fee848b5629586b069b18b82225cbd714392ef82561f6148d94a0173`. Static image probing found
`tint2`, `pcmanfm`, `wmctrl`, both Restless launch scripts, the two desktop entries and the supervised
`desktop-panel` declaration. Docker reports an image content size of 871,397,134 bytes; no comparable
pre-sprint image remained locally, so this run does not invent an incremental-size claim.

`restless up -c sprint08_ui_test --reconcile` replaced only the `_test` shell and kept
`restless-vol-sprint08_ui_test`. After supervisor warm-up, `restless doctor -c sprint08_ui_test`
reported `live`, current reconciliation, no proposed action, and all six imported services running:

- `desktop`
- `openbox`
- `desktop-panel`
- `chromium`
- `browser-broker`
- `desktop-web`

The first exact `restless-dev sprint08_ui_test --reconcile` continuation exposed a readiness defect:
the launcher treated normal post-replacement Supervisor warm-up as terminal degradation and stopped
the host stack. `restless-dev` now polls doctor for at most 20 seconds before emitting the definitive
probe. With the exact `_test` container deliberately stopped, the same command waited through startup,
returned `live` with no actions and left the current host stack available.

The first live shell probe found a real Tint2/Openbox integration defect: `panel_dock = 1` advertised
a bottom strut but made Openbox reserve only the right-hand 640 pixels. The distro's shipped
`panel_dock = 0` setting was then tried against the same running X server before being selected. The
rebuilt result reported:

```text
_NET_WORKAREA = 0,0 1280×770
Chromium       = 0,0 1280×770; maximized vertically and horizontally
Tint2          = 0,770 1280×30
```

PCManFM opened `/company/projects` as a separate X window. Activating the Browser launcher helper
returned `_NET_ACTIVE_WINDOW` to Chromium while the Chromium process count remained 10 before and
after, proving it focused the supervised browser instead of opening a second profile. The durable
Home links resolved exactly to `/company/downloads`, `/company/projects` and `/company/outputs`.

The gated CDP broker then navigated the supervised browser to `file:///company/mission.md`. Its
generic URL checkpoint recorded that exact durable file. After an actual Runtime-container restart,
all six services returned, Chromium reopened `file:///company/mission.md` in the same persistent
profile at 1280×770, and PCManFM reopened `/company/projects`. This closes S10-T3 with a useful
browser/file task and persistent state rather than with package presence alone.

## Prepared-handoff and control evidence

The final non-review fixture uses existing Work
`11111111-1111-4111-8111-000000000006`, owned by the durable `market-research` actor Theo Marsh.
Before creating its `identity` handoff, the supervised Chromium was driven through the gated CDP
broker to GitHub's real sign-in page. The live probe returned HTTP 200, title
`Sign in to GitHub · GitHub`, one login field and one password field; no credential was entered. Its
source data says:

- requested action: sign in on that already-open page with the owner's GitHub identity, return
  control, and do not create or change repositories;
- prepared state: the exact URL, title and visible fields in the persistent company profile;
- resume condition: the same profile leaves `/login`, exposes an authenticated account menu and is
  re-probed by market research before Work resumes;
- responsible actor and brief author: Theo Marsh (`market-research`);
- uncertainty: GitHub may next require MFA or a human check, in which case this same handoff is
  refreshed instead of creating a second owner request.

The current Attention projection reports category `human_step`, the exact current Runtime generation,
`requesting_actor = market-research`, a current owner brief, and no fixed application enum. Its
Work-scoped conversation loaded without an API error. The prepared route rendered that exact ask and
Theo Marsh at all required widths. It took the exclusive lease, changed the prepared framebuffer to
1144×824, returned to observer mode and left the handoff pending. Return delivered the existing
lease-only notification to Theo's inbox. A post-return CDP probe still found `/login`, one login field
and no authenticated account menu, so the source condition remained unmet and no completion,
approval or owner self-attestation was recorded.

The earlier Exec-authored outcome-review fixture
`e176da21-2d11-4aff-ae6c-95ed7ec15fd8` usefully exposed an invalid actor/Work conversation pairing and
is no longer used as prepared-human-step evidence. It is still separately owner-owed. Exec was
correctly refused when it tried to withdraw that review; only an explicit owner Accept or Request
changes decision may settle it.

After the cofounder's older host stack stopped, the current daemon and Vite proxy were started through
`restless-dev`. Two direct attachments were issued through the actual `http://127.0.0.1:5173`
same-origin path. The observer redirect contained reconnect/shared state, `resize=scale` and
`view_only=1`; the controller redirect contained reconnect/shared state, `resize=remote` and
`view_only=0`. A second attached client received HTTP 409 while the first lease was live, then took
control successfully after the first returned it. Both returns ended in `unclaimed`.

The existing `sprint07_test` outcome-review item contains both a live `review_target` and a
`runtime_attach`, making it the exact ambiguity Sprint 10 must resolve. The current same-origin API
issued an isolated `*.localhost:7794` review ticket, that URL returned the real HTML outcome, and the
browser controller remained `unclaimed` before and after. The Attention route selects `?review=` for
that category before considering its computer attachment. Native review therefore bypasses the
desktop without weakening or acquiring the desktop lease.

## Rendered Playwright evidence

The founder authorised standalone Playwright as the rendered owner-browser harness after the
configured in-app browser remained unavailable. Playwright 1.55 drove a cached Chromium build against
the real `http://127.0.0.1:5173` cockpit; it did not replace Runtime-side observation. The first
1440×900 render disproved the source-level layout claim: every height ancestor was 900 pixels, but the
desktop iframe was 1438×150 and an empty third grid row consumed 705 pixels. After the explicit grid
placement correction, the same route reported:

| Owner viewport | Toolbar | Live desktop iframe | Document |
| --- | ---: | ---: | ---: |
| 390×844 | 388×84.734 | 388×759.266 | 390×844; no scroll |
| 768×900 | 766×45.234 | 766×854.766 | 768×900; no scroll |
| 1440×900 | 1438×45.234 | 1438×854.766 | 1440×900; no scroll |

Observer canvases retained the shared 1280×800 framebuffer and scaled locally. At 390 pixels the
16:10 canvas therefore letterboxed inside the tall viewport instead of changing company geometry.
Taking control in a 900×700 owner viewport changed the noVNC canvas from 1280×800 scaled to 898×561
into an exact 898×655 remote framebuffer. At that same instant the Runtime independently reported:

```text
Screen/VNC-0  = 898×655
_NET_WORKAREA = 898×625
Chromium      = 0,0 898×625
Tint2         = 0,625 898×30
```

At 1200×800 the second valid controller similarly produced a 1198×755 canvas. Control survived a
full page reload in the owning tab. A second tab rendered `Another owner tab controls`, remained on
`resize=scale&view_only=1`, and showed the server's conflict message when it tried to take control.
After the first tab returned, the second changed to `resize=remote&view_only=0`, resized the desktop
and returned to `Ready for control`. In all cases Return left the last framebuffer size in place and
changed only the lease and attachment mode.

The final prepared Attention route also had no document scroll and preserved the exact requested
action at all three widths. Its desktop iframe was 370×315 at 390×844, 524×811 at 768×900 and
1146×826 at 1440×900; the remaining area belonged to the source-owned ask and Theo Marsh conversation,
not an empty page. Reload preserved both direct and prepared focus URL state. Direct Leave restored
the Company computer portal, then the originating Company route restored the prior open Exec rail.
Prepared Leave removed only `computer=` and returned to the still-pending Attention item. Opening the
bounded Exec overlay did not reflow the desktop.

Rendered canvas clicks opened PCManFM from Tint2, used the taskbar to bring its window forward, and
returned to the same maximised `file:///company/mission.md` Chromium tab. PCManFM's Home view visibly
contained `Downloads`, `Outputs` and `Projects`. A separate `sprint07_test` item with both target types
rendered its isolated live website in the native review frame; no desktop frame appeared.

One measured warmed-local sample reached a connected observer canvas 4.256 seconds after navigation
and settled an actual controller resize 3.098 seconds after Take control. This is run evidence, not a
performance target. Reduced-motion emulation was honoured by the route, keyboard traversal reached
visible focus rings on compact actions and Exec, and all control consequences remained available as
hover explanations. A rapid repeated prepared attachment also produced the bounded `Preview offline`
/ `Live outcome unavailable` state instead of a black implied success; a fresh retry connected.

The final comparison used Beautiful UI's calm dense AI surfaces and compact tool state, Cult UI's
expandable-screen continuity between trigger and full-screen state, and Origin UI Svelte's precise
ordinary buttons and tooltips as the bar. The selected Restless treatment keeps the exact action as
content priority, uses the existing view transition for portal-to-desktop continuity, and keeps
controls compact with clear focus/hover states. No React runtime, second design system, ornamental
animation or additional decoration was added.

## Success-contract audit

| # | State | Authoritative evidence or missing proof |
| - | ----- | --------------------------------------- |
| 1 | **Proven** | Playwright recorded a full-height live desktop at 390, 768 and 1440 CSS pixels with no document overflow; it also caught and verified the repair of the 150-pixel grid-row defect. |
| 2 | **Proven** | Direct and prepared focus survive reload; ordinary chrome/spine disappear only in focus, the bounded Exec overlay remains reachable, and Leave restores the portal/source route plus prior rail state. |
| 3 | **Proven** | Current same-origin live tickets/cookies work; frontend transport search is empty; generic observe/control redirects are server-owned. |
| 4 | **Proven** | Observer scaling retained 1280×800; sole-controller takeover changed both rendered canvas and Runtime X to 898×655; a competing rendered tab stayed observer-only and received the explicit conflict. |
| 5 | **Proven** | Chromium remained maximised at 898×625 plus the 30-pixel taskbar, later fitted 1198×725, survived controller reload and returned to the same persistent `mission.md` tab. |
| 6 | **Proven** | The accountable `market-research` actor prepared the exact live-probed GitHub URL, title, fields and observable resume condition without an application enum; the cockpit only projected that source. |
| 7 | **Proven** | The exact human-step title, requested action and responsible Theo Marsh rendered beside the computer at all widths, and the Work-matched conversation loaded with no API error. |
| 8 | **Proven** | Take/heartbeat/return changed only the lease; Theo received the return notification, the source probe still found sign-in incomplete, and the handoff remained pending with no review, Work or authority mutation. |
| 9 | **Proven** | A live item with both target types rendered real review HTML through the isolated review origin with no desktop frame while desktop control stayed unclaimed. |
| 10 | **Proven** | Rendered launcher/task switching plus Runtime/X/CDP folder and Supervisor evidence passed; PCManFM showed durable company folders and no full desktop environment is installed. |
| 11 | **Proven** | Named volume survived replacement/restart and restored the exact useful file URL in the persistent Chromium profile. |
| 12 | **Proven** | Daemon boundary tests keep missing/stale/expired/lost authority states owner-readable, and a failed prepared attach rendered the bounded offline context instead of an empty or black implied success. |
| 13 | **Proven** | The bounded-readiness fix made `restless-dev sprint08_ui_test --reconcile` return live from a deliberately stopped Runtime on macOS. |
| 14 | **Proven** | Frontend has no noVNC/WebSocket/resize policy, focus/footer alternatives were removed, and only the selected shell remains. |
| 15 | **Proven** | The rendered owner workflow connected, resized, reloaded, switched applications and returned control without a named latency/fidelity/audio/high-motion/GPU failure; no workload crossed the replacement gate. |

## Checks observed

- `docker build -f infra/company-image/Dockerfile -t restless-company-image:latest .` — passed twice,
  including the selected Tint2 correction.
- `cargo test -p restlessd` — 101 passed on the final combined worktree.
- `cargo test -p restless` — 2 passed.
- `cargo test -p restless-orgintel` — 13 passed across unit and integration suites.
- `cargo test -p restlessd owner::tests::only_the_controller_can_resize_the_shared_desktop -- --exact`
  — passed after concurrent worktree edits.
- `npm run check --silent` — passed with zero errors and warnings on the current combined worktree.
- `npm run build --silent` — passed on the current combined worktree and wrote the static site.
- `cargo fmt --all -- --check` — passed after mechanically formatting the cofounder's three
  semantically unchanged owner-projection lines.
- `bash -n scripts/restless-dev` and a stopped-Runtime readiness run — passed.
- image static probe, X geometry probe, task-switch probe, prepared Attention ticket and
  control/heartbeat/return probe — passed.
- Playwright responsive, reload, control resize, competing-tab, launcher, durable-folder, Leave,
  reduced-motion, keyboard-focus, coherent prepared-human-step and native-review runs — passed.
- `git diff --check` — passed.
- `shellcheck` — unavailable on the host; the scripts were exercised in the built image instead.

During verification a separate cofounder Sprint 10b office branch changed owner and web files in the
shared worktree. Its intermediate 13 OfficeCanvas/type errors were not attributed to this slice or
worked around. The cofounder branch subsequently converged: the current combined Svelte check,
production build, Rust suite, formatting check and source/image digest are green. Its product changes
remain preserved as founder-owned work.

## Owner decision remains separate

The rendered-browser blocker and all 15 technical success criteria are closed. The transport decision
is based on a real owner workflow rather than source inspection, and S10-T1 through S10-T4 now pass.
The earlier outcome-review handoff remains pending because accepting or requesting changes is an owner
judgement, not a verification side effect. Its invalid Exec/Work conversation pairing is no longer
used as evidence; the coherent Theo Marsh human-step fixture supplies that proof without weakening
Work ownership.

Two non-blocking observations remain recorded rather than inflated into a stack migration: noVNC's
optional `package.json` version fetch returns 502 through the proxy while the canvas connects, and one
rapid repeated prepared attachment showed the bounded offline fallback before retrying successfully.
Neither failed the owner workload; either becomes a ticket only if it repeats in ordinary use.
