# Inbox filters and compact Work board

Owner friction: the Skipped drawer clips its rows (#20/#35), empty board columns take a full
queue's width (#51), and titles grow to four or five lines (#52).

Layer: cockpit projections and layout. OrgIntel remains the writer for dismissals; Authority and
OrgIntel retain their existing decision history. No new status or second writer is introduced.

Acceptance:
- Open, Skipped, Done and All are available in the header filter. Filters survive reload and Back.
- Skipped is a normal scrollable list with Restore. Done is paginated and links to the original
  decision. A failed history read says so, never that no decisions exist.
- The empty Inbox still reaches All. Assigned teammates do not fetch owner-only lists.
- Empty board columns are narrower; titles fit two lines with the full title in Work detail and
  available on hover. Phone columns stack in the existing order.

Deleted: Skipped's bottom drawer, duplicate empty-Inbox drawer, its disclosure styling and the
reserved drawer wrapper. The existing shared top-layer popover supplies the filter menu.

Visual calibration: Beautiful UI for compact reading density, Cult UI for clear column hierarchy,
and Origin UI Svelte for ordinary control sizing. No external component code or runtime imported.
Named browser, geometry and role evidence is in the sprint spec; the wider sprint stays open.
