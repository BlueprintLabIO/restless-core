# One steady document surface

The Inbox preview added its own Back header above the editor, and the editor moved Saved to a
second row whenever its pane was narrow. While the live connection opened, a differently styled
saved-body projection appeared before the live editor. These are the E1 and C2 failures from the
owner's session. E2 moves Archive and Back to Inbox out of a separate footer row.

The native document preview now uses its editor header. Saved has reserved width on that row;
narrow actions use the existing menu. Initial connection shows a skeleton in the writing column.
The read-only fallback shares the live body's typography instead of the separate projection style,
and says Read only when the connection fails. The redundant saved-version heading is removed.

Archive and Back to Inbox float in the page margin. In a narrow document pane they are compact,
labelled icon controls; the writing gutter keeps them clear of the text. When a narrow inspector
replaces the document, Back to Inbox remains in its existing header. Archive and Undo still use the
same document API and permission checks; no second archive path was added.

Observed in the isolated `s109docs` profile, seeded with real-scale data:

- `verify-document-still.mjs` passed at 1440 and 390 pixels. A delayed real collaboration grant
  showed the skeleton, header and Saved geometry stayed within one pixel, the toolbar remained on
  one row, and text-range measurements found no text under the floating controls.
- The real floating Archive request succeeded; Undo restored the document through the same API.
- An aborted live connection retained the live editor's font family, size and line height, with
  Read only shown. Returning from both the document and its discussion kept the selected Inbox item.
- Frontend diagnostics and the production build passed. An initial warm-slot dependency mismatch
  was resolved with the locked install; no dependency versions changed for this UI work.

Evidence: `/tmp/s109-docs-still-final/report.json` and its opening, settled and read-only captures.
The isolated real owner/admin/member journey also passed against this UI and Better Auth 1.7.12:
concurrent edits, discussion, member document archive/restore/delete, role changes and live session
revocation all passed. Evidence: `/tmp/s109-combined-roles/journey.log`.
Release integration remains required before the sprint items are checked. C7's all-surface release gate is a separate remaining requirement.

The Inbox preview suite was also run after the document-specific Back label changed. Its first
attempt reached a sleeping company before its Runtime was ready and timed out opening the file
preview; Doctor then confirmed readiness. This startup gap remains part of A4/C7, rather than being
counted as a document styling pass. The subsequent complete journey results are in
`/tmp/s109-docs-previews-final/report.json`.

Four Library document/Work captures at 1440/390 passed clipping, overflow and page-error checks
(`/tmp/s109-docs-captures/report.json`). Motion checks opened and returned at the latest message
with zero gap; the streaming reply kept its 770px width across 275 samples
(`/tmp/s109-docs-motion.log`). The built-in browser desktop and phone views were inspected too.
