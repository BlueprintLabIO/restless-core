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
- [x] A1 A UI-only release restarts no company. The plane's roll-forward and doctor now compare a company computer's image by its build-input label (`io.restless.build-input-sha256`, which publish refuses to seal unless it matches the image's inputs), not by image id. A release that changed nothing the computer is built from keeps every computer running. The label now covers the Rust binaries' inputs too (RUNTIME_INPUTS plus RUST_INPUTS), so the same label means the same computer. Owner plane companies run in local runtime mode, so the hosted bridge's revision fence does not apply to them. Coordination health still forces a rebuild when an old computer can't talk to the new plane. Earlier in the sprint: `/health` reports `busy` (#403), and the Cloud agent holds a new bundle until the plane is quiet (restless-cloud#120).
- [x] A2 Checked: release roll-forward already waits for a quiet company (`company_busy`, up to `MAX_DEFERRAL_HOURS`) before rebuilding its computer. The remaining restart on every release is the plane service itself (A1).
- [x] A3 Checked: a Work attempt's agent session is already resumable. Its ACP locator is written to the company volume (`work:<id>`), and the next attempt calls `session/load`, so a restart costs the in-flight turn, not the context. Fixed: an attempt whose computer restarted under it (so `/tmp` and its ownership marker were gone) no longer fails with "marker is missing; broad process cleanup was refused" (Aang's attempt 2). Continuing: Work whose Attempt a plane restart killed now resumes once on its own (`resume_after_platform_restart`). The successor Attempt reloads the saved agent session. A second restart in the same revision leaves it blocked for the lead, so a crash loop never retries forever.
- [x] A4 Stop keeps trying for up to a minute while the company is unreachable (network errors, 5xx, 408, 429) instead of failing the moment the connection drops.
- [x] A5 Checked: the gateway already reloads within ~5s when credentials or sign-ins change (restlessd's fingerprint loop), so the "Ready once the company restarts" hint is stale. Fixed: an agent assigned to a connection that was removed or replaced keeps thinking on the company's current connection for that provider (`provider_fallback`), and no blocker is filed while that works. Only an agent whose provider has no connection left is blocked. Separate, still needs the plane log: a Claude sign-in that fails to reconcile leaves `NO_DIRECT_PROVIDER` set ("No model connection is active yet").
- [x] A6 Exec reports platform faults instead of "nothing we control".
- [x] A7 Rewrite the connection error screens.

### B. Attention
- [x] B1 A finished result Exec names in `ownerReview` becomes an Inbox item ("Ready for you: …") until the owner replies. Exec files missing logins itself with `restless apps ask-account` instead of sending the owner to Settings.
- [x] B2 A question whose `ownerNeedHandoff` names a setting proposal becomes that proposal's context, so one item shows instead of two.

### C. Chat
- [x] C1 Every send scrolls the owner's message to the top (option answers missed it).
- [x] C2 New focus scrolls clear and keeps "Company memory retained" and the capability hint.
- [x] C3 People shows when an agent is busy with Work and offers Interrupt, which keeps the Work. A DM to a busy agent now steers the running Attempt. The live turn checks for a person's unread direct message every 15 s; when one arrives it stops the turn and hands the message to the same session as a checkpoint input. The agent answers with `restless message` and continues the Work. The message counts as delivered, so no second conversation turn repeats it.
- [x] C4 "Thinking" sits inline as the last transcript item.
- [x] C5 People replies don't snap narrow when they finish.
- [x] C6 Upload progress is drawn on the file's thumbnail.

### D. Models and credit
- [x] D1 One model picker everywhere (`lib/primitives/ModelPicker.svelte`); one entry per release.
- [x] D2 Checked: Google Gemini, xAI (now labelled "xAI Grok"), OpenRouter, Groq, Mistral, DeepSeek, Z.ai and Moonshot already connect with an API key. Meta Muse is added as a provider ("Meta Muse", Muse Spark 1.2 and 1.1). The key is validated against `api.meta.ai/v1/models`, and OMP's built-in `meta` provider serves it through the Responses API.
- [x] D3 Usage limits name the account they read.
- [x] D4 Operator grant for unlimited AI credit (Cloud #118, migration 0063); the row for blueprintlabio is inserted after deploy.
- [x] D5 Model spend already has a hard per-company ceiling with Inbox warnings (`spend_ceiling_usd`, spend_warning and spend_stopped). Loop detection: a Work Attempt that produces the same result (final message and tool calls) three turns running, or asks to continue 25 times, stops as Blocked with its work kept. Burst jobs are capped by their timeout and host-fitted memory (G4). The sandbox cap moves with F2.

### E. Library
- [x] E1 "Open in Library" opens the Library; the viewer fills its pane (no 150px frame).
- [x] E2 Library › Folders lists every file under /company/outputs (owner only, newest first, read from the computer every minute, last listing kept while it sleeps). A file already in the Library opens as that entry; the rest open by path.
- [x] E3 Responsive Library. Checked live with headless Chromium at 375, 768 and 1024 px against real company data, with no page-level horizontal overflow. Fixed: cockpit.css's later `display: flex` kept the wordmark at tablet and phone widths, so at 768 px the tabs ran under search and the company name was cut to "Sydne…". On a phone the eight Library views wrapped onto three rows before the first file; they're now one row that scrolls sideways. Documents and sheets fit a phone.

### F. Browser sandbox
- [x] F1 Expand fills the main area and keeps the Exec rail.
- [~] F2 The computer moves into its own container; downloads sync to the Library. Shipped: what the company browser downloads (`/company/downloads`) is listed in Library › Folders as "Downloads/…" by "Browser", and opens there. Still open: the computer as its own container, which is the F5 sandbox work.
- [~] F3 Chrome stable, Sydney timezone and locale, persistent profile. Shipped: the company browser takes the owner's timezone and language. The owner's own browser reports them to the plane (`PUT /api/browser-locale`, owner only, kept in `browser-locale.json`), and a company computer picks them up when it next starts. A plane's RESTLESS_BROWSER_TIMEZONE or RESTLESS_BROWSER_LANG still wins. This needs no Cloud contract change and works self-hosted too. Still open: Chrome stable on amd64 (there is no arm64 build).
- [x] F4 Chrome's screen feed, with the desktop one switch away (owner's choice, 22 Oct: both, switchable, Browser by default). The Computer page has a Browser | Desktop switch, remembered per browser. Browser streams `Page.startScreencast` JPEG frames of the tab the company used last (`/desktop/{company}/screencast`, owner_screencast.rs), and follows tab changes every 3 s. Input (mouse, wheel, keys) reaches the page only while this tab holds the owner control lease, the same rule the VNC desktop applies. The first press asks for control. Desktop is the existing noVNC view. The DevTools sequence was replayed on Chromium: frames flowed with acks, and a click plus typing reached the page.
- [x] F5 One sign-in, both browsers (owner's choice, 22 Oct: both work at once). Live check on Sydney Resale: the act tool worked (borrow, open, read), but the facebook.com login browser was never signed in. The owner had signed in on the company computer's Chrome, and health read signed_in only because Facebook sets consent cookies first (fixed in #425). Now, before an agent gets a login browser, the plane checks for a real sign-in. If there isn't one, it copies the account's own-domain session cookies from the company computer's Chrome, inside the plane where no agent sees them (`sign_in_from_company`), and updates the account's health. Signing in through Settings › Accounts, or on the company computer, both work. The copy was replayed between two Chromiums: session and httpOnly cookies arrived, and other domains stayed behind.
- [x] F6 The company browser sleeps after 15 idle minutes and wakes for any agent request, desktop viewer, Core DevTools use or browser app (tabs reopened). Verified in a container: 873 MiB to 482 MiB with three tabs, wake in about 4 s. Chrome renderers are also capped at 4.
- [x] F7 Checked: it can. `REFUSED_METHODS` blocks cookie and storage reads, but `Runtime.evaluate` is relayed, so page script can read a typed password, `document.cookie` and `localStorage`. Fix tracked as S1.
- [x] F8 Website login form failure. The owner says it was Facebook on the company computer, which said the computer was in California. Checked 22 Oct: both the company computer and the login browser leave from the plane's Hetzner Singapore IP (5.223.54.19, which public geolocation puts in Singapore), with no proxy. Facebook's judgement comes from treating a datacenter address as suspect, not from anything Restless configured. The fix is a home IP through the owner's own machines (S99). Timezone and language now follow the owner (F3), which removes one mismatch sites flag.
- [x] F9 When a running computer's desktop doesn't answer, the page's primary action is "Restart the desktop" (files and profile kept), with Health as a secondary link.

### G. Cost and capacity
- [x] G1 Memory telemetry in the plane heartbeat (Cloud #118).
- [x] G2 Plane moved to cpx22, keeping its IP addresses (cpx21 and cpx31 are retired in Singapore).
- [x] G3 CPU priority for chat over builds.
- [x] G4 Burst compute: `restless job run` / `restless job status` with a local provider (own container, CPU, memory, pids and time bounds, log in /company/run/jobs). A Cloud provider can sit behind the same calls.

### H. Polish
- [x] H1 Work filters match the buttons beside them.
- [x] H2 App store refresh: stop hitting GitHub's per-IP limit.
- [x] H3 The light-tile favicon is served everywhere: the plane, app.restless.run (/favicon.ico after restless-cloud#117 deployed with release 9) and restless.run (Pages deploy, 22 Oct).

### S. Security
- [x] S1 Agents drive login browsers through high-level actions only, so no page script runs on their behalf in a signed-in profile. Stage 1: the `__act` tool (open, read, click, type, scroll, screenshot) through the tool gateway. Stage 2: the borrow tool hands out no DevTools endpoint, and the proxy allows only Page, Input, Accessibility, read-only DOM and Target steps (`devtools_allowed`), refusing Runtime, Debugger, injected scripts, DOM rewrites, Network, Fetch and storage reads. Live check on Sydney Resale (22 Oct, release 9): Facebook Marketplace opened signed in through the act tool (sign-in carried over from the company computer, F5); borrow, open, click, scroll and screenshot worked. Read returned only page controls, because Marketplace's header filled the read, so read now gives the main region first, drops text that repeats its parent's name, and holds 48 kB.

### T. Coupling and tech debt (found along the way)
- [x] T1 Company CPU bound was a hard-coded 4 and refused every start on a 2-core host; it now fits the host, and a stopped container is refit before start.
- [x] T2 Automatic computer setup dropped its error; "Exec can't start" had no cause anywhere. The error is logged and reported (`setup_error`).
- [x] T3 A plane restart drops every browser session (in-memory session store).
- [x] T4 The plane exits when Postgres isn't up yet at boot instead of waiting for it.
- [x] T5 Cloud `docs/operations/move-plane.md` and `scripts/ops/move-plane.sh` (BlueprintLabIO/restless-cloud#119).
- [x] T6 Not a bug: Cloud's own fallback list for Singapore (cpx32, ccx13) only names types still sold, and the provider checks live stock. It was Hetzner's price list that still showed cpx21 and cpx31.
- [x] T7 The Inbox intercepted every Library link (fixed with E1). Audited the other `beforeNavigate` handlers: all are unsaved-change guards.
- [x] T8 The company memory bound is kept to three quarters of the host's memory (a 4 GB host gives 3 GB companies).

**Incident, 22 Oct 01:00–03:00 (Sydney time): the 4 GB plane thrashed into an outage.**
- **Causes:**
  - Each release's roll-forward woke every sleeping company to rebuild it. Fixed by #407.
  - Every plane boot started every company. Fixed by #409.
  - Four awake companies with Chrome need more than 4 GB (Sydney Resale's Chrome reached 2.2 GB).
- **Effects:** CPU at 200%, `/health` unreachable, Fleet showing every card as "Updating" or "Can't open yet", and SSH timing out.
- **Relief overnight:** stopping idle companies and writing their sleep markers, and two reboots.
- **Owner decision:** resize to 8 GB RAM-only (cpx32; keep the 80 GB disk so it can shrink again), or stay on 4 GB with browser sleep (#406).

**Memory after the first overnight release (22 Oct, from the new heartbeat):**
- All four companies awake used everything: about 20 MB free and 1.2 GB of swap. Sydney Resale was at 1.8 GB and Reno at 0.8 GB.
- The plane then answered in 2.5–7 s. Fleet's readiness probe timed out, marked it degraded, and every card showed "Updating".
- A 4 GB plane holds two or three awake companies, not four. Browsers sleeping on their own (F6) is the real fix; the alternative is going back to 8 GB.

Measured 21 Oct on cpx22: plane service 680 MB, Postgres 490 MB, one login browser 240 MB, Reno
840 MB (Chrome ~750 MB), Potion Scheme 450 MB. Two companies awake used 3.26 of 3.8 GB; a 4 GB
swapfile was added as a net.

## Verification

Each ticket is checked live on Cloud as owner and member, at desktop and phone widths, with
screenshots looked at. A UI-only release is shipped once to prove no company restarted.
