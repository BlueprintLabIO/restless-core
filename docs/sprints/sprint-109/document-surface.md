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
Authenticated owner/admin/member verification and release integration remain required before the
sprint items are checked. C7's all-surface release gate is a separate remaining requirement.
