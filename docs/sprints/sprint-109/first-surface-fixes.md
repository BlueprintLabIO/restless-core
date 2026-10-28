# First surface fixes: G1 default, C3 hints, C5 notices

Observed friction: the owner reaches a list instead of the board; composer keyboard hints appear
and disappear over the conversation; a successful Vault save leaves a permanent banner (#19, #55).

Layer: owner cockpit projection. No domain state or authority transition changes.

Acceptance:
- Opening Work without a view parameter selects Board. Explicit ?view=list selects List;
  choosing either view and reloading preserves that choice.
- Focusing, typing and blurring the composer adds no keyboard hint element or changes its geometry.
  Send's tooltip retains Enter/Shift+Enter, menu hints and the streaming Alt+Enter shortcut.
- Successful Vault create/replace/delete uses the shared transient toast. Save failures remain
  visible and secret values stay cleared after submission.

Delete: composer hint localStorage counter, focus tracking and hint CSS; Vault's duplicate success
notice state. Keep shared toast timing and error behavior authoritative.

Evidence: frontend checks pass. Browser, role matrix, motion/capture and production evidence pending.
Status remains in the sprint checklist; these findings are not completion claims.