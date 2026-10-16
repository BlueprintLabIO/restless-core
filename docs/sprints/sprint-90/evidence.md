# Sprint 90 observed evidence

Worktree: `codex/product-polish`, based on `c9fbc870`. Uncommitted work is not an installed release. All mutations below are confined to the isolated `polish_test` company, namespace `uipolish90`, with a scripted model.

## Recorded on 16 October

- Final `verify-product-quality`: 14/14 passed, including desktop/phone discovery/detail/search/pagination/Back, the compact short-window picker, long draft/fixed footer, held/failed catalogue, setup-dialog failure, truthful fresh-session status, know-how create/review/edit/remove/restore, receipt failure/recovery, archived-only groups and late company reads.
- `verify-overlays`: 26/26 passed after repairing an introduced fast outside-click dismissal race. Earlier failed runs are retained, not counted as passes.
- Final `verify-motion`: latest-message gaps 0px on opening and returning; scripted streaming/final reply width 770px, 277 samples. Scripted messages are evidence of UI behaviour, not autonomous business quality.
- Final `capture-surfaces`: 56 captures across 28 route entries at 1440/390 passed clipping, overflow, raw-text and page-error probes. Additional Apps light/dark captures passed. Human desktop/phone review supplements those probes.
- All 12 Inbox preview/review variants passed across focused runs: actual Accept outcome/Request changes, files/editors/Limits, document/charter recovery, linked Exec context and independent update selection at both widths. Consumed review fixtures are not silently re-used as new approvals. The final native recovery run also checks the concise Brand preview heading.
- Frontend check: zero errors/warnings. Production build passed; the last small Brand preview copy change receives another source check, and promotion builds its exact clean main source.
- 28 focused Apps, directory, persistence and steady-state tests passed, including safe hydration while a live read is in flight. Three app catalogue Rust tests passed. Owner API unit tests: 127 passed, one intentionally ignored long-running visual server. Final engine/owner/OrgIntel source check passed after Brand copy updates.

## Performance scope

Same 34-file populated fixture, same backend, Chromium 1440×900, three visits per source, baseline `c9fbc870` frontend versus current frontend. Development builds, not a production latency forecast. Global window resize listeners fell 60→2, window scroll listeners 59→1, document pointer listeners 60→1. Median script time for 60 scroll changes fell 9.16ms→2.31ms. Median page-ready times were 1468ms→1570ms: **no page-open speedup is claimed**. DOM count remained 2026, so a large virtualisation rewrite is not justified by this fixture.

The same two actual decisions were 94,743 bytes in the old combined read and 3,821 bytes in the slim queue read (96% less). Warm API time changed about 220ms→209ms, a modest improvement. Current references load separately on surfaces without the full graph; full-graph surfaces join one graph query. History is retained. This does not claim that backend projection cost is eliminated.

## Remaining gates

The implementation/interaction/discovery/restraint/performance/acceptance slices have observed evidence. Main integration, the required exact-build connections smoke, supported appliance promotion and read-only Sydney Resale acceptance remain. No overall completion percentage is inferred from test counts.

The late-company read journey caught an existing `state_unsafe_mutation`: the first-paint query-cache subscriber changed reveal state inside a reactive query-key calculation, halting navigation with stale wide sidebar links. Deferring/coalescing reveal work repaired the transition; the delayed old response and wide links now pass. The staged second company projection is read-only and is not permission-enforcement evidence.

## Extra regressions found during acceptance

- Switching from the bottom of Installed to Discover inherited the old pane scroll. The Apps view now remounts its scroll surface when the view changes; search/category/Back state still persists. Desktop and phone discovery journeys check the new view's search control is on screen.
- Actual GitHub rate limiting during a catalogue refresh kept the last good listings. Installed and first-party app details no longer display a discovery-only failure. The manual refresh returned 3,675 cached entries in 95ms while its background task ran, and persisted public metadata survived the daemon restart. A missing company Work graph returned 404.
- A native-review reload exposed a pre-existing interaction between safe query hydration and the write-wins guard: hydrating the filtered summary cancelled the fresh read. The omitted Authority rows were briefly treated as resolved. Hydration now remains a read, and a saved partial summary cannot establish that a request disappeared. The new regression passes with the actual steady client; native document/charter preview recovery passes on desktop. Phone document/charter recovery also passed. File/editor/settings previews pass at both widths, as do real Accept outcome and Request changes flows. The phone conversation refresh check passed after its response matcher was updated for the slim queue URL.

## Audit coverage

| Finding | Result and evidence |
| --- | --- |
| A01 | Canonical registry IDs, publisher disambiguation, safe ambiguous legacy links; Apps identity regressions. |
| A02–A04 | Discover/Installed, useful app detail and one primary action, complete searchable/paginated listings, preserved Back/search/category; actual desktop/phone journeys. |
| A05–A08 | Installed reads are independent of public discovery; partial failures/retry, shared detail reads, company cancellation and delayed-response/mutation checks. |
| A09 | Background single-flight refresh, source deadlines/freshness, persistent public snapshot and manual retry; three Rust tests plus live cached-read/restart probes. Individual skill-file read failures retain the last good source rather than silently dropping entries. |
| A10–A13 | Selection popover, Older models stays open, compact effort choices with scope explained, bounded short-window picker and composer; keyboard, overlay and geometry checks. |
| A14 | Brand naming across navigation, main page, preview, generated notices and agent navigation guidance; one screen title and readable print. Historical records are not rewritten. |
| A15–A18 | Removed Recently decided, retained Activity and group history, conditional empty Chats, consistent Advanced bodies and truthful fresh-session activity without a production Attempt. |
| A19 | Decision queue and Work reads separated; history preserved and full-graph surfaces share their query. Wire-size improvement is measured; database projection cost is not claimed eliminated. |
| A20 | Closed overlay listeners removed; controlled runtime trace improves scroll script cost. No unmeasured virtualisation rewrite. |
| A21 | Replaced Apps' competing layout/forms/read controllers and intent heuristics with shared data, cards, focused setup and native table structure; preserved existing permission/install/know-how paths. |
| A22 | Current Cloud main fan-out risk not reproduced. Two-company desktop/phone section-navigation checks landed as Cloud commit `3172afd`; no unsupported data-layer replacement or hosted-latency claim. |
| A23 | Fourteen product journeys, actual Inbox/document/file/review paths, serious/critical accessibility gate, selected-surface/journey validation, traces on failure and explicit local motion target. All reported failures remain in evidence; only passing reruns count. |
| A24 | Required Python connections smoke no longer depends on executable mode and a missing gate fails closed. Its exact-release run and supported activation remain the release gate. |

No additional frontend runtime was added. The new dev-only accessibility dependency is pinned `axe-core`; Svelte and the existing TanStack cache remain the implementation. Cleanup removed competing behaviours, not retained company history.
