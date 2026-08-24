# S14-T4 — Split daemon transport and Staff supervision by ownership

**Layer:** daemon plus Runtime Bridge.

**Observed friction served:** `main.rs` combines a 57-optional-field socket request with dispatch, and
`staff.rs` combines conversation context, ACP lifecycle, spend, workspace observation, completion and
recovery. S12 proved these concerns must change together only rarely.

## Outcome

Transport handlers and Staff implementation live in small Rust modules aligned to their current
responsibilities while callers retain the same observed API and socket/CLI behaviour.

## Acceptance

- Decode request payloads per lifecycle, Authority, OrgIntel and owner domain; dispatch remains one
  explicit daemon entrypoint but no all-optional domain payload grows.
- Do not introduce a universal `Command` enum, protocol crate or new cross-plane writer.
- Separate Staff context/conversation, ACP execution/metering, workspace observation, completion and
  recovery implementation; preserve the existing public dispatch entry points.
- Move code before changing it. Every intentional semantic correction is its own small commit/evidence
  note and must name the observed defect.
- Rerun direct-message, late-feedback, terminal-observation, orphan-recovery and budget-fuse scenarios.
- Delete helper duplication made obsolete by the new internal module boundary.

## Non-goals

- new microservices;
- reworking the actor model, runtime process model or owner API;
- changing Work/Attempt lifecycle semantics;
- adopting a generic message bus or workflow engine.

## Deletion target

God-shaped transport inputs and mixed Staff implementation regions that hide ownership.

## Evidence

- The flat JSON boundary now groups common, lifecycle, Authority, OrgIntel and owner fields in
  `wire.rs`; a focused decoding test passed without introducing a universal payload type.
- Staff is split into context, conversation, execution, recovery and workspace modules while the
  existing public dispatch and orphan-sweep entry points remain unchanged.
- The complete daemon suite passed with 117 tests, including direct-message, late-feedback,
  terminal-observation, orphan-recovery and budget-fuse scenarios.
