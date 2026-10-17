# Sprint 94 — business books and compact conversations

The owner asked to split Spend into Tokens and Accounting. Accounting must record business
revenue and expenses, not only AI costs. The default Exec conversation label “General” adds
clutter, and a meaningful topic must still fit in one compact header row.

- [x] Tokens owns consumption, employee/model/day breakdowns and clearly labelled estimates.
  Existing Spend URLs redirect without losing query parameters. Limits stays the budget control.
- [x] Accounting records income and expenses with integer minor-unit amounts, currencies,
  settled/outstanding/void status, date, counterparty, category and source evidence. Corrections
  retain one identity; retries never duplicate a record, and stale edits cannot overwrite changes.
- [x] Ordinary business observations live in the company's OrgIntel schema, with no payment
  execution, budget bypass or new immutable ledger. Owner writes; admins read; members cannot read.
  Existing payment-rail observations remain separate until reconciled. No fictional historical
  revenue, automatic bank import, invoice reconciliation or tax-compliance claim.
- [x] The Exec rail hides the default label. Identity and meaningful topics share one 44px bounded
  row; long titles truncate with full text available, controls remain reachable and history stays.
- [x] Verify money precision, currency/status separation, duplicate retries, stale edits, company
  isolation and restart persistence. Review populated desktop and phone journeys in the built-in
  browser; run the standard surface/motion checks against a scripted-model test company.
- [ ] Land verified source, install the supported local appliance and qualify hosted rollout.

Reuses existing page, dialog, query and payment-report primitives; deletes the mixed Spend surface.
No new UI runtime, financial service or rate table. Accounting's first useful scope is a simple
company money book; automatic imports and reconciliation need a later observed integration.

## Verification — 18 October

- Frontend diagnostics: zero errors/warnings; type/UI boundaries and document-search checks pass.
- Ten money and role checks pass, including exact decimal parsing, zero/three-decimal currencies,
  unsafe precision refusal, separate currencies, outstanding amounts and void records.
- The real PostgreSQL scenario passes: create retry counts once, changed replay/stale edit refuses,
  current-version correction succeeds, void retains evidence, reopen persists and companies isolate.
- Owner crate check and the server's owner/admin/member financial-route boundary test pass.
- Accounting's real API and desktop/phone UI save/edit/void, held-data refresh failure, accessible
  states and Spend-link redirect pass; serious/critical axe findings are empty after removing faded
  void text. Explicit select labels preserve concise accessible names.
- Both default and long-topic journeys pass: the header is 44px, the full topic remains available,
  its text truncates and every header control stays within the pane at 1440px and 390px widths.
- Standard capture reports eight clean desktop/phone surfaces; the seeded conversation opens at
  its latest message before and after switching away. A scripted model is used, with no paid calls.
- The built-in browser review covers empty and populated Accounting, a real income save, desktop
  chat and phone rows. Review caught the shared disclosure marker consuming a grid column; both
  Accounting and Tokens now reserve the marker outside their data columns.
- Beautiful UI (records/chat), Cult UI (Minimal Card) and Origin UI Svelte were consulted for aligned
  rows, restrained edges and compact conventional controls. No upstream source or UI runtime copied.
- The first development profile exposed old fixture credential conflicts, so it was stopped and
  acceptance moved to a dedicated `ta94` profile. No stable company data was used as test data.

Accounting is an operational money book, with manually evidenced business records and a separate
payment-rail feed. Automatic bank/invoice imports and reconciliation remain future integrations;
model estimates never become accounting entries or charge totals.
