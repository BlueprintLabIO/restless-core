# S05-T5 · Persistent remote browser and owner handover

**Layer:** One vertical slice across Company Runtime, Runtime Bridge, Owner Gateway/SPA and the
Authority effect boundary
**Serves:** the prepared-last-mile promise and Sprint 05's real Aris launch review
**Depends on:** S05-T1 (source-owned attention envelope and authenticated owner surface)
**Makes deletable:** manual “open this link, do these steps, tell me when done” handoffs, the
headless-only browser assumption in the company image, and any proposed site-specific browser API

---

## Observed starting state

The current company image has Debian Chromium and Playwright, but only as processes an agent launches
headlessly. It has:

- no visible desktop;
- no long-lived Chromium service;
- no persisted browser-profile path in the image contract;
- no remote pixel/input transport;
- no owner authentication or attach-ticket exchange;
- no controller exclusion between owner and agent;
- no way for a blocked agent to resume from an observed owner action.

Sprint 04 therefore handed the owner compare URLs and instructions outside the company computer. The
fixture SPA contains the right narrative example—a registrar session “held open”—but nothing behind it
can actually hold or transfer that session.

## Product contract

> **When a machine-doable workflow reaches an irreducible human step, the company preserves the exact
> visible browser state, brings that session to the owner in the Attention Inbox, gives the owner
> exclusive control, and resumes the requesting agent against the same state after hand-back.**

The owner is not asked to recreate navigation, copy state into their own browser, or report completion
when Restless can observe it. The lease ending means only “the owner no longer controls input.” The
source request resolves from its own condition or receipt.

## Choose the desktop stack by running it

Two viable open-source candidates exist:

| Candidate | Shape | Expected trade-off |
|---|---|---|
| **KasmVNC + Openbox** | Integrated browser-native remote desktop tuned for web delivery | Better fit and input experience; GPL-2.0 packaging obligations |
| **TigerVNC + noVNC + Openbox** | Conventional VNC server plus separate HTML5 client/proxy | More modular and widely packaged; more moving pieces and likely weaker browser UX |

Build the smallest runnable image of each before choosing. Run the same probe:

- visible Chromium launch against the persistent profile;
- SPA/WebSocket embedding through the owner gateway;
- keyboard, pointer and clipboard;
- reconnect after page refresh;
- Runtime restart and profile restoration;
- 1280×800 legibility and interaction latency on the actual development host.

Record versions, licences, image-size delta, startup time and observed failures. Pick one, remove the
other image path and dependency set, and record the decision in the run report. Apache Guacamole is not
a third candidate unless both fail: it adds a gateway/service tier that this one-desktop workload has
not earned.

## Runtime shape

The selected stack runs as durable company services under one imported process supervisor:

```text
company runtime
├── desktop server
├── Openbox session
├── visible Chromium
│   ├── profile: /company/browser-profile
│   ├── downloads: /company/downloads
│   └── automation endpoint: loopback only
├── web transport endpoint: runtime network only
├── local browser controller/lease
└── Exec and worker ACP processes
```

Use the existing persistent `/company` volume; do not add an artifact-custody path or a database for
tabs and cookies. The browser stays running between agent turns. Chromium starts with the persistent
profile and session restoration enabled, and is shut down cleanly where possible. After a crash, lost
ephemeral page state is reported honestly; persistence is not promised beyond what the browser wrote.

Agents connect to this visible Chromium through its loopback automation endpoint. They do not launch a
second private headless browser for work that may be handed over, because there would be no common
session to transfer. Independent acceptance probes may still use disposable headless browsers when no
handover is involved.

## Controller handover

The controller lease is deterministic, enumerable runtime coordination—not organisational approval:

```text
unclaimed → agent_controls(session)
agent_controls → owner_requested → owner_controls(principal)
owner_controls → agent_controls(requester) or unclaimed
unclaimed → owner_controls (manual inspection/rescue)

any state → unavailable
unavailable → unclaimed after recovery
owner_controls + unclean disconnect → bounded reconnect grace → prior controller or unclaimed
```

Rules:

1. One shared profile has at most one input controller.
2. The requesting session reaches a safe yield point before ordinary takeover. Runtime Bridge records
   the actor/session association and pauses or refuses its browser automation while the owner holds the
   lease.
3. A forced rescue takeover may pause the associated process tree, but it does not freeze unrelated
   internal work or grant new external authority.
4. A second owner tab can observe but cannot inject input while another tab controls.
5. Clean hand-back releases the lease immediately. An unclean disconnect gets a short reconnect grace,
   then releases automatically so the company cannot wedge forever.
6. Before waking/resuming the requester, Runtime Bridge captures checkable browser observations that
   are already available generically—current URL/title and whether the browser process/session still
   exists. A screenshot may aid context but is not proof of an external effect.
7. The agent inspects the actual page and decides how to continue. The broker does not classify website
   content into success/failure with selectors, regexes or provider-specific states.

The lease is reconstructable live state. OrgIntel may retain the request and handover outcome, but
there is no durable lease ledger or custom workflow engine.

## Owner Gateway and remote access

S05-T1's authenticated owner gateway is the only browser-facing entrance:

1. The attention projection contains an opaque attach reference, not a network address or credential.
2. The owner opens the item and requests attachment.
3. The gateway verifies owner principal, company, current Runtime generation and source item.
4. It issues a single-purpose, short-lived attach ticket and proxies the desktop WebSocket.
5. The Runtime desktop and CDP endpoints remain unreachable from the public network.
6. Expiry, wrong company, stale Runtime generation and reused tickets fail closed.

The listener defaults to loopback. Use imported Caddy as the remote HTTPS/WSS ingress and TLS
terminator; the owner gateway and Runtime desktop remain private behind it. Off-machine use needs a
trusted, remotely reachable HTTPS origin, which may use private/VPN DNS or a public hostname. A
public hostname is not an architectural requirement. A local-only mode may stay on loopback but does
not prove off-machine reachability. No plaintext remote
desktop, bearer token in a URL, VNC password in browser storage, or direct published desktop port is
accepted.

## SPA focus mode

The existing inbox remains the launch and return surface:

- the detail pane first shows why the owner is needed, recommendation, evidence, requested action,
  no-response behaviour and whether the company can continue;
- **Open live browser** replaces the main queue/detail canvas with the desktop at useful size;
- a persistent status strip shows Runtime health, page label, requesting actor and exactly who controls;
- **Take control** and **Return control** operate only the lease;
- the executive rail remains available on demand without covering the desktop;
- leaving focus mode does not kill the browser, resolve the item or claim success;
- reconnect and degraded states explain what is known rather than showing a frozen screenshot as live.

Public artifacts remain ordinary links. The desktop is not opened merely because a URL exists.
Attention is the normal prepared-handover entrance; the Authority/runtime rescue surface may also
attach for owner-initiated inspection using the same gateway and lease rather than a second path.

## Consequential actions through the browser

Browser navigation is ordinary Runtime work. When the prepared action creates a material consequence,
it uses the existing effect process:

```text
generic effect intent
→ deterministic authority decision / approval if needed
→ browser or owner performs the prepared action
→ generic JSON outcome attached to receipt
→ provider confirmation when available, otherwise attested or unknown
→ reconciliation before retry
```

Do not add `github.merge`, `stripe.click`, `registrar.submit` or one API/adapter per website merely to
drive the browser. The consequence may have a capability name such as `repo.merge` or
`domain.register`, but the transport stays the one browser path and the receipt outcome stays arbitrary
JSON. Idempotency and unknown-outcome handling remain identical to HTTP effects.

## Sensitive accounts

The persistent profile is authority. During Sprint 05:

- low-risk, scoped company accounts may persist in it;
- personal/root provider identities, owner passkeys and accounts with broad destructive power do not;
- those attention items carry the prepared context plus a direct link for the owner's own browser;
- the company verifies the resulting external state and resumes without asking the owner to narrate it
  when an external probe is available.

Profile segregation, just-in-time browser credentials and parallel profiles are deferred until one
real workload cannot satisfy this boundary.

## Scope

1. Compare the two bounded desktop candidates and purge to one.
2. Add the selected visible desktop, Openbox, Chromium service and imported supervisor to the company
   image.
3. Persist profile and downloads under `/company`; provide honest health for desktop, browser and
   automation endpoint.
4. Make the shared visible Chromium the standard handover-capable browser for agents.
5. Implement the single-controller lease and agent safe-yield/resume path in Runtime Bridge/process
   supervision.
6. Add short-lived attach-ticket exchange and authenticated WebSocket proxy to the owner gateway.
7. Put the owner surface behind Caddy for real HTTPS/WSS remote access; keep owner-gateway, desktop and
   CDP listeners private.
8. Add the SPA desktop focus mode, controller status, reconnect and degraded states.
9. Carry browser-driven material consequences through the existing generic effect/receipt path.
10. Exercise the full handover in `_test`, then use it for Aris review with live email sends still held.

**Not in scope:** multiple profiles, hostile actor isolation, general co-browsing, recording, mobile
optimisation, a browser farm, semantic DOM brokerage, site-specific commands, password management, or
making the browser desktop the source of organisational truth.

## Acceptance

### Headless/checkable substrate

1. A `_test` company starts through `restless up`; the run transcript contains no owner-authored
   `docker build`, `docker run`, `docker exec` or published VNC port.
2. Desktop, browser and automation health are probed live. Killing each process produces the correct
   degraded state and the imported supervisor restores it without restarting the entire company.
3. A cookie, open test tab and downloaded marker survive `restless down` / `restless up` against the
   same company volume. The browser view reconnects to the current Runtime generation.
4. Direct access to the desktop/CDP port fails from outside the Runtime network. Valid attach works;
   expired, reused, cross-company and stale-generation tickets fail for the expected reason.
5. From a second browser client that cannot reach the Runtime network, the owner signs in over HTTPS,
   opens the SPA and establishes the desktop over WSS. Browser developer tools show no mixed content,
   raw desktop credential or direct Runtime address.

### Full owner handover

6. An agent opens a stateful local `_test` page in the shared visible Chromium, completes all
   machine-doable fields, requests owner attention and yields.
7. The owner opens that exact page inside the SPA, takes control, supplies the one human-only value and
   returns control. Refreshing the SPA during the grace window reconnects to the same desktop.
8. While the owner controls, an attempted agent browser action is demonstrably paused/refused and a
   second owner tab is observer-only. On hand-back, the requester observes the changed page and
   completes without the owner sending a “done” message.
9. Closing the SPA without changing the page leaves the source attention item unresolved. Runtime
   death renders `unavailable`, never a stale still image presented as live.

### Effect and real-company proof

10. In a `_test` company, one simulated browser-driven consequential action uses a stable idempotency
   key and produces the existing generic JSON effect receipt. Replaying it does not repeat the action;
   an intentionally ambiguous result remains `unknown` until reconciled.
11. In Aris, the inbox surfaces the real compare and production URLs. The owner reviews them through
    the appropriate remote or direct browser path, and an independent Git/HTTP probe—not lease release
    or agent prose—decides whether the launch gate cleared.
12. Through T5 acceptance, all four real tutoring-centre emails remain unsent and no new live
    `email.send` receipt exists.

### Quality

13. The selected desktop remains usable at 1280×800 with keyboard, pointer, scrolling, clipboard and
    ordinary browser shortcuts. The run report records observed latency and any unsupported input.
14. `cargo test --workspace`, `npm run check` and the image/browser smoke probes pass after the losing
    desktop path is removed.

## Risks

| Risk | Disposition | Why |
|---|---|---|
| Owner/agent input race | **Invariant** | Single controller lease with process/automation exclusion |
| Browser state lost on restart | **Guarded** | Persistent profile/download path and observed restart probe; unwritten page state may still be lost honestly |
| Public remote-desktop exposure | **Invariant** | Runtime-only endpoints; authenticated TLS gateway and scoped tickets are the sole entrance |
| Controller lease wedges after disconnect | **Guarded** | Bounded reconnect grace, expiry and recoverable live-state reconstruction |
| Root credential remains available to agents | **Guarded** | Sensitive identities use direct owner browser; shared profile is explicitly low-risk/scoped |
| GPL packaging surprises | **Pending fix before distribution** | Record the selected stack's licence obligations in the run; commercial use is allowed, distribution obligations need deliberate compliance |
| Pixel stream is mistaken for effect evidence | **Invariant** | Receipts and external reconciliation remain separate; screenshot/connection state never confirms consequence |
| Browser broker becomes a workflow engine | **Guarded** | It owns only attach, health and controller lease; OrgIntel owns why/resume context and Authority owns effects |

## What this makes deletable

- instructions that ask the owner to recreate the agent's navigation;
- “tell me when you have finished” as the normal resume protocol;
- a separate headless browser for work intended for handover;
- proposals for one RPC or provider adapter per browser action;
- direct exposure of a VNC/Kasm/noVNC credential to the SPA.

---
Sprint spec: [`../sprint-05.md`](../sprint-05.md)
