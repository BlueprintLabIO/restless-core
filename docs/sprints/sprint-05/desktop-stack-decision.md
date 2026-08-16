# Sprint 05 desktop-stack decision

**Selected:** TigerVNC + noVNC + Openbox, using the Debian 12 packages.

**Discarded:** KasmVNC + Openbox.

This is the observed Sprint 05 implementation decision. The winning path is in
`infra/company-image`; the Kasm candidate container and image were removed after the comparison, and
there is no dormant Kasm image, adapter or credential path in the tree.

## Why this branch ran first

Both candidates are open source and produced a browser-native desktop. The Debian base already ships
TigerVNC, noVNC, websockify and Openbox as maintained packages, so the selected candidate adds no
external package repository and keeps its components independently replaceable. KasmVNC 1.5.0 was
installed from its separately distributed Bookworm release package and served its authenticated web
client with visible Chromium. Its desktop selector rejected Openbox, however; it ran only after a
manual `xstartup` selection. It also introduced its own Basic-auth credential and a non-RFB web proxy
contract. Those are real integration branches the selected private RFB stream does not need.

The deciding risk is operational weight, not feature count: Sprint 05 needs one persistent 1280×800
Chromium desktop, reconnect and clipboard—not a browser-farm control plane.

## Licence and distribution

- TigerVNC: GPL-2.0-or-later
- noVNC: MPL-2.0
- Openbox: GPL-2.0-or-later
- websockify: LGPL-3.0

Commercial use is permitted. Distribution of an image containing these packages must preserve their
licences and corresponding-source obligations where applicable. This repository does not yet claim a
redistributable product image.

## Acceptance probe — observed 15 August 2026

| Check | TigerVNC/noVNC | KasmVNC |
|---|---|---|
| Versions | TigerVNC 1.12.0, noVNC 1.3.0, websockify 0.10.0, Openbox 3.6.1 | KasmVNC 1.5.0, Openbox 3.6.1 |
| Image footprint | Current full company image: `864,488,192` bytes from image inspect (`3.57GB` Docker CLI virtual-size display) | Minimal comparison image with Chromium/Openbox: `408,058,578` bytes; not directly comparable because it omitted OMP, Restless, Playwright and the company toolchain |
| Cold start | `3.713s` from `restless up` to all browser health probes available | `0.520s` from manual `vncserver` start to authenticated HTTPS response, after the automatic Openbox selection failed |
| Visible Chromium | Chrome 151 available through the shared display and loopback CDP | Chrome 151 available through display `:1` and loopback CDP |
| Gateway fit | noVNC assets and the RFB WebSocket are proxied behind the existing owner session; no VNC credential reaches the browser | Built-in client returned 401 without its Basic-auth credential and 200 with it; integration would need a second credential/proxy contract |
| Refresh / reconnect | Repeated authenticated attachment and WSS-to-RFB negotiation succeeded | Repeated authenticated client fetches returned 200; a full pixel-session refresh was not visually exercised |
| Runtime restart restores profile/tab/download | Passed against the same `_test` volume; a flushed persistent cookie, restored tab and downloaded marker survived down/up | Not promoted to the persistent Runtime after the gateway mismatch was observed |
| Keyboard / pointer / scroll / clipboard | Automated end-to-end pass through the real SPA/noVNC canvas: typed value, pointer click, 120px scroll and clipboard paste all reached visible Chromium; a human owner visual acceptance pass remains | Not visually exercised before the losing branch was purged |

The selected path remains conditional on the human owner's own visual acceptance pass. Automation
proved the real input path and layout, but it cannot decide whether the experience is comfortable or
trustworthy to the owner. A failure there reopens the decision. The Kasm branch has already
demonstrated that it is a viable fallback rather than a paper comparison; restoring it would still
mean deliberately accepting the extra package, credential and proxy contracts.

## Candidate cleanup

The exact test-only Kasm container `restless-s05-kasm-candidate`, image
`restless-kasm-candidate:s05`, and two `/tmp/restless-kasm-*` probe files were deleted after evidence
was recorded. The container had no mount and no published port, so this removed no company data.
