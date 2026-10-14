# UXG-296 — Activate sharing and Tailscale addresses without leaving the cockpit

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Endpoint & CLI parity | high | missing-function | US-183 |

## Problem

Enable sharing ends by downloading a setup file and showing three terminal commands (prepare-sharing with four file paths, tailscale-addresses, enable-sharing --tailscale), plus SOCKS-tunnel advice. Until those run, invitations do not work. Distinct from UXG-083, which asks for a way forward when email invites are missing. Here the way forward exists but is an installer procedure in a code block, so the CEO receives instructions rather than a prepared state.

## Evidence

- `web/src/lib/components/SharingSetup.svelte:85-104` (terminal commands shown as the activation path)
- `crates/restless/src/main.rs:510-531` (PrepareSharing / EnableSharing / TailscaleAddresses / TailscaleDoctor)
- `crates/restless/src/main.rs:2267-2292` (dispatch)

## Suggested direction

Have the plane detect Tailscale and fill in the addresses itself, then run prepare and enable as one confirmed action in Members. Show only the single irreducible human step, such as a Tailscale sign-in.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
