# Sprint 41 — Make native takeover truthful for Claude Code

**Status:** Planned; activates after Sprint 40 passes  
**Date:** 4 September 2026  
**Programme:** [Open Company Runtime](./open-company-runtime-programme.md)  
**Depends on:** Sprint 40's proved controller/checkpoint/return journey and Sprint 39's pinned Claude
Agent SDK, ACP adapter, scoped Anthropic relay, session identity and cleanup contract.

## Why this sprint exists

One excellent Codex path would still leave native access as a provider-specific exception. Claude Code
uses the same underlying agent engine across its SDK, CLI and desktop surfaces, but that does not prove
that a Restless-owned ACP/SDK session can be transferred to each client under API-relay authentication.

This sprint tests the real boundary rather than declaring interface parity. Exact live attachment,
stored-session resume, fork, canonical reconstruction and workspace-only access remain different
outcomes.

## Outcome

An owner can take control of Claude-backed Work through the genuine pinned Claude Code CLI inside the
exact company workspace. Restless uses the strongest relationship actually supported by the admitted
build: exact session resume when proved, otherwise a new session reconstructed from canonical Restless
context with an explicit label and provenance link.

Return uses the same controller, checkpoint and reconciliation contract proved by Codex. The Claude
path retains isolated configuration, scoped model access, approved tools/MCP and exact cleanup. No
subscription login or provider root key is introduced as a convenience shortcut.

## Frozen product decisions

1. **Genuine CLI first.** Claude Desktop is not required or bundled in this sprint.
2. **Relationship truth outranks symmetry.** Exact resume and reconstruction use different states,
   copy, evidence and recovery behavior.
3. **The pinned Restless profile remains authoritative.** Native access does not load ambient user
   settings, plugins, hooks, subagents, skills, MCP servers or approval caches.
4. **The existing host Anthropic relay remains the provider path.** API access does not become a raw
   Runtime secret and subscription OAuth is not silently substituted.
5. **One controller and one workspace.** Sprint 40's transfer invariants are provider-neutral and may
   not be relaxed to fit Claude.
6. **Canonical context is bounded.** Reconstruction uses Work, messages, evidence and the current
   checkpoint, not a scrape of every private harness transcript.
7. **Native CLI actions remain scoped.** Ordinary filesystem/command work is allowed inside the
   company boundary; external effects remain brokered.
8. **Desktop compatibility is evidence for Sprint 43, not acceptance here.** Record what cannot move
   between CLI and Desktop under the admitted authentication route.

## Success contract

1. A checked-in compatibility matrix records the exact adapter, SDK, Claude Code CLI, model,
   authentication route and session-storage builds used.
2. A credential-free process probe proves CLI readiness, exact isolated config and the intended model
   route before productive input.
3. If exact SDK/CLI resume is claimed, the native CLI and Restless observe the same upstream session id
   and prior controlled turn without replaying tools, messages or usage as new work.
4. If exact resume is not supported, the UI says **New Claude session from Restless context** and records
   the predecessor without implying shared private history.
5. The native session starts in the exact current worktree/revision with the same actor, Work and open
   owner question represented in its launch capsule.
6. Restless sends no autonomous prompt while owner control is active or uncertain.
7. Allowed tools, denied capabilities, MCP endpoints and permission mode match the certified profile;
   hostile ambient fixtures cannot augment them.
8. Provider usage remains attributed where observable; native or subscription-only usage that cannot
   be observed remains explicitly external/unknown rather than zero.
9. Return, no-change return, permission refusal, CLI crash, relay loss, Runtime restart and stale
   session storage pass the common reconciliation and recovery corpus.
10. Cross-company, cross-actor and cross-harness session reuse fail before useful context or control.
11. No provider key, subscription token, unscoped relay capability, session transcript or orphaned
    process remains after cleanup.
12. The final report records which Claude Desktop capabilities remain inaccessible and whether any
    repeated owner job justifies Sprint 43 activation evidence.

## Slice per layer

**Authority and gateway.** Continue scoped model admission and metering without placing provider or
subscription credentials in Runtime. Distinguish Restless-metered from owner-external native usage.

**OrgIntel.** Reuse the provider-neutral control and return state. Record exact-resume versus
reconstruction provenance without importing a second transcript canon.

**Runtime Bridge.** Add the certified Claude native-client profile, verify session compatibility,
isolate config, launch in the exact workspace and normalize lifecycle/cleanup.

**Owner surface.** Present the real attachment relationship, relevant limitations and the same Take
control / Return vocabulary used for Codex.

## Salvage

- Reuse Sprint 39's Claude package pins, integrity checks, relay, ACP profile and isolation only after
  the native CLI path passes the same launch-manifest inspection.
- Reuse Sprint 40's controller, checkpoint, local handle and reconciliation contract byte-for-byte
  where provider behavior does not require an explicitly named capability branch.
- Do not salvage personal Claude config, Desktop login state or ad hoc raw-key experiments.

## Out of scope

- bundling Claude Desktop;
- guaranteeing CLI-to-Desktop transfer under API-key authentication;
- personal plugins/connectors in autonomous mode;
- a general ACP client or arbitrary agent support;
- migrating a live session between Claude and Codex;
- provider-independent private transcript conversion; and
- changing Restless's default harness.

## Stop rules

Stop exact-resume work if the provider does not document or the corpus cannot prove session
compatibility. Ship honest reconstruction if it preserves the outcome. Stop the whole Claude native
path if it requires a reusable provider/subscription credential in Runtime, ambient config authority or
unscoped external tools.

## Ticket index

| Status | Ticket | Outcome |
| --- | --- | --- |
| [ ] | [S41-T0](./sprint-41/t0-freeze-claude-transfer-matrix.md) | Freeze exact resume, fork and reconstruction claims |
| [ ] | [S41-T1](./sprint-41/t1-certified-claude-native-client.md) | Launch the genuine pinned CLI through isolated scoped auth |
| [ ] | [S41-T2](./sprint-41/t2-session-relationship.md) | Prove exact resume or implement explicit reconstruction |
| [ ] | [S41-T3](./sprint-41/t3-control-policy-and-telemetry.md) | Preserve tools, permissions, MCP, usage and controller parity |
| [ ] | [S41-T4](./sprint-41/t4-claude-takeover-surface.md) | Show native relationship and limitations truthfully |
| [ ] | [S41-T5](./sprint-41/t5-cross-harness-dogfood.md) | Run matched Codex/Claude takeover, recovery and cleanup |

Expected order: **T0 → T1 → T2/T3 → T4 → T5**.

## Terminal decision

- **Exact pass:** Claude CLI resumes the same session and meets the common boundary.
- **Reconstruction pass:** native access is useful and loss-bounded, but product copy remains explicit
  that a new session was created.
- **Stop negative:** retain managed Claude Agent support and do not offer native takeover if its auth,
  policy or return path weakens the company boundary.
