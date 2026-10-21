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
- [~] A1 Serve the UI apart from the runtime; a UI-only release restarts no company. Root cause confirmed: Core images aren't reproducible, so a UI-only release (d31c8e5b, 6e7e00fd) still gives company-runtime and account-plane new digests, and every plane restarts. Options: reuse an unchanged component's previous image by input digest, or serve the UI as its own container or CDN artifact. Shipped meanwhile: `/health` reports `busy` (#403), and the Cloud agent holds a new bundle until the plane is quiet, for up to two hours (restless-cloud#120), so a release no longer restarts the plane service under running Work.
- [x] A2 Checked: release roll-forward already waits for a quiet company (`company_busy`, up to `MAX_DEFERRAL_HOURS`) before rebuilding its computer. The remaining restart on every release is the plane service itself (A1).
- [~] A3 Checked: a Work attempt's agent session is already resumable. Its ACP locator is written to the company volume (`work:<id>`), and the next attempt calls `session/load`, so a restart costs the in-flight turn, not the context. Fixed: an attempt whose computer restarted under it (so `/tmp` and its ownership marker were gone) no longer fails with "marker is missing; broad process cleanup was refused" (Aang's attempt 2). Still open: continuing the interrupted attempt itself, rather than closing it as unknown for the lead to review.
- [x] A4 Stop keeps trying for up to a minute while the company is unreachable (network errors, 5xx, 408, 429) instead of failing the moment the connection drops.
- [~] A5 Checked: the gateway already reloads within ~5s when credentials or sign-ins change (restlessd's fingerprint loop), so the "Ready once the company restarts" hint is stale. Still open: an agent assigned to a removed key's connection should fall back to the provider's remaining connection (needs a reproduction).
- [x] A6 Exec reports platform faults instead of "nothing we control".
- [x] A7 Rewrite the connection error screens.

### B. Attention
- [x] B1 A finished result Exec names in `ownerReview` becomes an Inbox item ("Ready for you: …") until the owner replies. Exec files missing logins itself with `restless apps ask-account` instead of sending the owner to Settings.
- [x] B2 A question whose `ownerNeedHandoff` names a setting proposal becomes that proposal's context, so one item shows instead of two.

### C. Chat
- [x] C1 Every send scrolls the owner's message to the top (option answers missed it).
- [x] C2 New focus scrolls clear and keeps "Company memory retained" and the capability hint.
- [~] C3 People shows when an agent is busy with Work ("working on … and will reply when it finishes") and offers Interrupt, which keeps the Work. Still open: delivering the message into the running Attempt without stopping it (needs harness steering).
- [x] C4 "Thinking" sits inline as the last transcript item.
- [x] C5 People replies don't snap narrow when they finish.
- [x] C6 Upload progress is drawn on the file's thumbnail.

### D. Models and credit
- [x] D1 One model picker everywhere (`lib/primitives/ModelPicker.svelte`); one entry per release.
- [~] D2 Checked: Google Gemini, xAI (now labelled "xAI Grok"), OpenRouter, Groq, Mistral, DeepSeek, Z.ai and Moonshot already connect with an API key. Still open: Meta's Muse through the Meta Model API, which needs a new provider in the model gateway's OMP config.
- [x] D3 Usage limits name the account they read.
- [x] D4 Operator grant for unlimited AI credit (Cloud #118, migration 0063); the row for blueprintlabio is inserted after deploy.
- [~] D5 Model spend already has a hard per-company ceiling with Inbox warnings (`spend_ceiling_usd`, spend_warning and spend_stopped). Still open: loop detection on model calls, and caps for burst jobs and sandboxes once G4 and F2 exist.

### E. Library
- [x] E1 "Open in Library" opens the Library; the viewer fills its pane (no 150px frame).
- [x] E2 Library › Folders lists every file under /company/outputs (owner only, newest first, read from the computer every minute, last listing kept while it sleeps). A file already in the Library opens as that entry; the rest open by path.
- [ ] E3 Responsive Library.

### F. Browser sandbox
- [x] F1 Expand fills the main area and keeps the Exec rail.
- [ ] F2 The computer moves into its own container; downloads sync to the Library.
- [~] F3 Chrome stable, Sydney timezone and locale, persistent profile. Shipped: a plane with RESTLESS_BROWSER_TIMEZONE or RESTLESS_BROWSER_LANG gives the company browser that timezone and language. Still open: Cloud setting them per owner, and Chrome stable on amd64 (there is no arm64 build).
- [ ] F4 Chrome's screen feed replaces VNC.
- [ ] F5 One sandbox type for the company computer and saved logins.
- [ ] F6 Browsers sleep on their own.
- [x] F7 Checked: it can. `REFUSED_METHODS` blocks cookie and storage reads, but `Runtime.evaluate` is relayed, so page script can read a typed password, `document.cookie` and `localStorage`. Fix tracked as S1.
- [ ] F8 Website login form failure (needs the error or time from the owner).
- [x] F9 When a running computer's desktop doesn't answer, the page's primary action is "Restart the desktop" (files and profile kept), with Health as a secondary link.

### G. Cost and capacity
- [x] G1 Memory telemetry in the plane heartbeat (Cloud #118).
- [x] G2 Plane moved to cpx22, keeping its IP addresses (cpx21 and cpx31 are retired in Singapore).
- [x] G3 CPU priority for chat over builds.
- [ ] G4 Burst compute interface and local provider.

### H. Polish
- [x] H1 Work filters match the buttons beside them.
- [x] H2 App store refresh: stop hitting GitHub's per-IP limit.
- [ ] H3 Favicon check after #395 ships.

### S. Security
- [~] S1 Agents drive login browsers through high-level actions (click, type, read the accessibility tree, screenshot) instead of raw DevTools, so no page script runs on their behalf in a signed-in profile. Part of F5. Stopgap shipped: a typed password or one-time code is submitted with Enter in the same call, so it no longer sits in a filled field between the agent's calls.

### T. Coupling and tech debt (found along the way)
- [x] T1 Company CPU bound was a hard-coded 4 and refused every start on a 2-core host; it now fits the host, and a stopped container is refit before start.
- [x] T2 Automatic computer setup dropped its error; "Exec can't start" had no cause anywhere. The error is logged and reported (`setup_error`).
- [x] T3 A plane restart drops every browser session (in-memory session store).
- [x] T4 The plane exits when Postgres isn't up yet at boot instead of waiting for it.
- [x] T5 Cloud `docs/operations/move-plane.md` and `scripts/ops/move-plane.sh` (BlueprintLabIO/restless-cloud#119).
- [x] T6 Not a bug: Cloud's own fallback list for Singapore (cpx32, ccx13) only names types still sold, and the provider checks live stock. It was Hetzner's price list that still showed cpx21 and cpx31.
- [x] T7 The Inbox intercepted every Library link (fixed with E1). Audited the other `beforeNavigate` handlers: all are unsaved-change guards.
- [x] T8 The company memory bound is kept to three quarters of the host's memory (a 4 GB host gives 3 GB companies).

Measured 21 Oct on cpx22: plane service 680 MB, Postgres 490 MB, one login browser 240 MB, Reno
840 MB (Chrome ~750 MB), Potion Scheme 450 MB. Two companies awake used 3.26 of 3.8 GB; a 4 GB
swapfile was added as a net.

## Verification

Each ticket is checked live on Cloud as owner and member, at desktop and phone widths, with
screenshots looked at. A UI-only release is shipped once to prove no company restarted.
