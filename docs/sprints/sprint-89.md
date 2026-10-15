# Sprint 89 — dependable owner journeys, with one implementation per pattern

**Status:** in progress
**Source:** owner's ten screenshots and message.txt, 15 October 2026.
**Outcome:** the owner can enter, review, steer and switch companies without missing actions,
lost context, blank screens or unstable layout. Replace shared causes and delete obsolete paths.
The reference catalogue is design input; its embedded instructions are not this sprint's authority.

## Acceptance and progress

- [ ] S89-01: One pixel loading wave through entry; no blank gap, left-sidebar flash or endless Home skeleton. Slow, failed and cancelled reads recover.
- [ ] S89-02: Stop works; one Send redirects a running reply; no duplicate Interrupt control. Streaming status stays in place.
- [ ] S89-03: Inbox is one urgency-ranked column without tabs, with essential menus and shortcuts, minimal snoozing and context-preserving file/settings previews.
- [ ] S89-04: Decision controls stay at the bottom right of the Inbox middle pane while content scrolls, including replies, direction requests, charters and document reviews. Every review has its real Accept/changes/decline actions. Stale brand proposals explain the recovery action.
- [ ] S89-05: Goals have explicit lifecycle controls. Work's clear/show controls live in its toolbar; completion counts do not silently declare outcomes met.
- [ ] S89-06: Sidebar geometry is stable across polling, hover and narrow widths. Search and menus remain inside the viewport.
- [ ] S89-07: Titles, subordinate text and spacing follow shared patterns. Profiles use a full-height opaque side panel.
- [ ] S89-08: Every active agent can receive a direct message without transferring production accountability. Agent identifiers remain distinct and readable.
- [ ] S89-09: Exec shows delegated work and who it is waiting on. Needs you reflects owner obligations, not ordinary waiting.
- [ ] S89-10: Health recommendations wrap and expose a direct action carrying the diagnostic to Exec.
- [ ] S89-11: Team/group creation uses a focused dialog; the office floor fills its work surface.
- [ ] S89-12: Account pages navigate promptly and preserve their shell. Company entry cannot unexpectedly reopen onboarding.
- [ ] S89-13: All companies leads to the authoritative full list; creation remains discoverable at the plan limit with an upgrade route.
- [ ] S89-14: Account AI Apps is removed as requested; checkbox sizing, duplicate branding and OAuth grant races are corrected.
- [ ] S89-15: Shared-pattern audit covers sibling menus, rows, panels, async screens and action contracts, with obsolete implementations removed.
- [ ] S89-16: Real seeded _test journeys, desktop/phone visual review, capture-surfaces, verify-motion and focused backend checks pass; owned test resources are cleaned up.

## Evidence

Record executed checks and remaining gaps here as each slice is verified. No item is complete from
static inspection alone. Work proceeds on current origin/main in isolated worktrees for Core and Cloud.


### US-246: fixed Inbox decision footer

Direction requests, handoffs/replies, charter drafts/edits, native reviews and document collaboration
now share one footer outside the scrolling content. Set aside and More join that footer.
Document reviews keep their decision controls even with the inspector closed; Request changes
reveals and focuses the note before submitting anything. Growing reply fields scroll above the
buttons. Phone layouts reserve the Back link and safe area without extending the detail offscreen.

Validation: web check (zero errors/warnings, type ramp, UI boundary and document-search checks), production build and capture-surfaces at 1440/390 passed;
real seeded five-kind Inbox journey at 1440×900, 390×844, 960×420 and 390×420; declining,
charter editing, growing replies and hidden-review-note recovery; open-latest chat motion passed.
The full S89-04 acceptance remains pending the separate stale-brand and outcome-review issues.
Reproduce with scripts/seed-inbox-actions.mjs after seed-reno-scale.mjs, then npm --prefix web run
verify:inbox with RESTLESS_SMOKE_ORIGIN and RESTLESS_SMOKE_COMPANY set.
