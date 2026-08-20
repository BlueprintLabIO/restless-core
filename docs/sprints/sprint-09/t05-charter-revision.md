# S09-T5 · Let the owner revise the charter without creating a second writer

**Layer:** Authority, Runtime projection and owner surface.

**Observed friction:** The charter is now readable, but changing one word still requires leaving the
cockpit and using the CLI. A naive browser save would either overwrite a newer edit or turn the
presentation layer into a second mandate writer.

## Outcome

The local owner can switch the Company charter between its rendered form and a borderless Markdown
manuscript, edit exact characters, and explicitly save a new owner-authorised revision. The existing
`CompanyConfig.mission` remains canonical; Authority records the revision evidence and the running
Company computer receives the refreshed read-only projection.

## Acceptance

- The Company read returns a deterministic revision derived from the exact canonical Markdown.
- Save requires that base revision and rejects a stale editor without changing source state or
  discarding the browser draft.
- Empty, oversized and NUL-containing documents are refused before any write.
- A successful edit is attributed to `owner`, records the previous and next revisions in Authority,
  and atomically updates `CompanyConfig.mission`.
- A running Runtime receives the new `/company/mission.md`; stopped or absent Runtime projection is
  reported as deferred rather than represented as failure or success.
- Editing is explicit, character-preserving Markdown with Save and Cancel. There is no autosave,
  Tiptap document model, collaboration server, casual-chat mutation or Exec/Staff write path.
- The edit state is borderless, keyboard reachable and responsive at the same widths as the charter.

## Deletion

Makes a future cockpit-specific Charter store, rich-text conversion adapter, autosave protocol and
collaborative editing service unnecessary. The deterministic CLI remains a useful administrative
control, not the only usable owner interface.
