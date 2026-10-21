# Sprint 104: Stop the bleeding, fix what the owner found, lay the foundations

Owner direction (2026-10-21): triage of a 32-item batch, eight decisions (D1–D8), and a redesign:
one self-hostable Core with three layers (control, company runtime, browser sandbox) that deploy,
sleep and fail on their own; Cloud packs many sleeping Cores onto shared servers. "Fully action the
entire sprint and anything that comes up along the way", including the coupling and tech debt found.

Design: the Restless Three-Layer Redesign page (claude.ai artifact NUcRyh4UjDLjZnDmy3w5j9).

## Decisions

- **D1** Split deploys: UI fixes restart nothing; runtime updates install when a company is idle; sessions resume.
- **D2** Anything that needs the owner (a login, a decision, a finished deliverable, a failure) becomes one merged Inbox item.
- **D3** A DM to a busy agent is injected into its running attempt.
- **D4** The computer stays in the Singapore cloud, always available.
- **D5** Company computers move into their own containers: real Chrome, Sydney locale, Chrome's screen feed instead of VNC.
- **D6** The Library indexes the outputs folder and is responsive.
- **D7** A comp flag gives blueprintlabio unlimited credit.
- **D8** Measure, then shrink the plane. Done 21 Oct: moved to cpx22 (2 cores, 4 GB, €34.09/mo).

Owner answers (21 Oct): shared logins stay one company at a time; CPU burst jobs only, no GPUs
yet; self-hosting is one machine (`docker compose up`); a whole Core sleeps after 30 idle minutes on
Cloud; Meta's Muse comes in through a direct Meta Model API provider; the target is a ~$20/month plan.

## Tracks

### A. Deploys and durability
- [ ] A1 Serve the UI apart from the runtime; a UI-only release restarts no company.
- [ ] A2 Install runtime updates when a company is idle; never interrupt Work, chat or computer use.
- [ ] A3 Resume agent sessions after a restart (session record written as it goes).
- [ ] A4 Stop works while reconnecting (queued, sent through the plane).
- [ ] A5 Credentials apply without a restart; agents point at a provider, not a key.
- [ ] A6 Exec reports platform faults instead of "nothing we control".
- [ ] A7 Rewrite the connection error screens.

### B. Attention
- [ ] B1 Exec can raise an Inbox item for anything that needs the owner, including finished deliverables.
- [ ] B2 Items for the same request merge (the $200 budget question and the spend-limit setting).

### C. Chat
- [x] C1 Every send scrolls the owner's message to the top (option answers missed it).
- [x] C2 New focus scrolls clear and keeps "Company memory retained" and the capability hint.
- [ ] C3 DMs reach a busy agent mid-task; show "mid-task" until picked up.
- [ ] C4 "Thinking" sits inline as the last transcript item.
- [ ] C5 People replies don't snap narrow when they finish.
- [x] C6 Upload progress is drawn on the file's thumbnail.

### D. Models and credit
- [x] D1 One model picker everywhere (`lib/primitives/ModelPicker.svelte`); one entry per release.
- [ ] D2 Gemini, Grok, OpenRouter, and Meta (Muse) through the Meta Model API.
- [ ] D3 Usage limits name the account they read.
- [ ] D4 Comp flag: unlimited credit for blueprintlabio (Cloud).
- [ ] D5 Budget guards: hard per-company caps, loop detection.

### E. Library
- [x] E1 "Open in Library" opens the Library; the viewer fills its pane (no 150px frame).
- [ ] E2 Auto-index the outputs folder, with a Folders view.
- [ ] E3 Responsive Library.

### F. Browser sandbox
- [x] F1 Expand fills the main area and keeps the Exec rail.
- [ ] F2 The computer moves into its own container; downloads sync to the Library.
- [ ] F3 Chrome stable, Sydney timezone and locale, persistent profile.
- [ ] F4 Chrome's screen feed replaces VNC.
- [ ] F5 One sandbox type for the company computer and saved logins.
- [ ] F6 Browsers sleep on their own.
- [ ] F7 Check whether an agent can read a filled password field.
- [ ] F8 Website login form failure (needs the error or time from the owner).
- [ ] F9 Recover "The desktop isn't answering".

### G. Cost and capacity
- [ ] G1 Memory telemetry in the plane heartbeat (Cloud).
- [x] G2 Plane moved to cpx22, keeping its IP addresses (cpx21 and cpx31 are retired in Singapore).
- [ ] G3 CPU priority for chat over builds.
- [ ] G4 Burst compute interface and local provider.

### H. Polish
- [x] H1 Work filters match the buttons beside them.
- [ ] H2 App store refresh: stop hitting GitHub's per-IP limit.
- [ ] H3 Favicon check after #395 ships.

### T. Coupling and tech debt (found along the way)
- [x] T1 Company CPU bound was a hard-coded 4 and refused every start on a 2-core host; it now fits the host, and a stopped container is refit before start.
- [x] T2 Automatic computer setup dropped its error; "Exec can't start" had no cause anywhere. The error is logged and reported (`setup_error`).
- [ ] T3 A plane restart drops every browser session (in-memory session store).
- [ ] T4 The plane exits when Postgres isn't up yet at boot instead of waiting for it.
- [ ] T5 Cloud has no plane-move tooling; the 19 and 21 October moves were done by hand.
- [ ] T6 Cloud's server price list includes types retired in Singapore (cpx21, cpx31).
- [ ] T7 The Inbox intercepted every Library link, wherever it was followed (fixed with E1); audit other page-level navigation interceptors.
- [ ] T8 A container memory cap of 4 GB is larger than a 4 GB host; caps should fit the host too.

Measured 21 Oct on cpx22: plane service 680 MB, Postgres 490 MB, one login browser 240 MB, Reno
840 MB (Chrome ~750 MB), Potion Scheme 450 MB. Two companies awake used 3.26 of 3.8 GB; a 4 GB
swapfile was added as a net.

## Verification

Each ticket is checked live on Cloud as owner and member, at desktop and phone widths, with
screenshots looked at. A UI-only release is shipped once to prove no company restarted.
