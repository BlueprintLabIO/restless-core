# Core Sheets · Sydney resale

Founder-authorized outcome: a reusable native workbook beside company Docs,
usable by humans and authenticated agents, fully smoked and landed on remote
`dev`. The observed dogfood is deal evaluation, inventory and realised margins;
this needs a shared editable grid and deterministic business edits.

- [x] Pin and embed upstream o-spreadsheet with Owl inside the Svelte cockpit.
- [x] Add company-local metadata/permissions and accepted-revision persistence.
- [x] Implement ordered WS OT acceptance, lost-ACK retries and reconnect replay.
- [x] Give agents range/cell/row/sort/filter/CSV/record operations through the
  same accepted stream, using a bounded credential-free upstream Model worker.
- [x] Make checkpoints viewable/exportable and recoverable into private copies.
- [x] Package the exact engine and Node runtime for local/hosted deployment.
- [x] Verify two real collaborators, structural collision, agent edits,
  worker/service restart, company isolation, history and CSV through the stack.
- [x] Compare the live desktop/mobile interface against the required references.
- [x] Independent review, final smoke and merge into remote `dev`.

The implementation contract and bounded MVP limits are in
[`../specs/native-sheets.md`](../specs/native-sheets.md). Upstream owns spreadsheet
semantics; Restless owns membership, ordering, durable receipts and history. The
existing Yjs Docs service remains a separate body implementation. Presence,
cursor fanout, Excel fidelity and log compaction are outside this MVP.

Completed on 2026-10-02. [PR #41](https://github.com/BlueprintLabIO/restless-core/pull/41)
landed on remote `dev` at `c66bee2151310e217baf1abc2dc85ee9df2f2a2d`.
The supervisor verified that its production tree exactly matches tested
candidate `27e927a3d0dcc74650472651937e760dbd702815`.

Verification evidence:

- Three pinned worker tests passed; web type/UI checks and production build
  passed; `cargo check --workspace --locked` passed for all wire consumers.
- Both real Postgres Sheets integration tests passed together, covering
  exec-Actor edits, atomic failure, stale/duplicate receipts, ACL/company
  isolation, client generations, immutable checkpoints/recovery and fresh
  worker replay. A verified hosted member exercised actual HTTP/WS editing;
  revoking its session closed the connection and denied subsequent requests.
  Member/admin route boundaries, CLI coverage and authenticated TCP actor
  binding also passed.
- The exact-source restarted Rust service reported candidate `27e927a` and
  schema 73. Two real upstream Models converged after colliding structural
  edits; human/headless edits, disconnected catchup, lost-response retries,
  full-range CSV including row 101 and fresh persisted replay passed again.
  Own accepted edits, undo and redo converged with durable formula readback.
- The browser mounted the upstream editor, saved typed/money/formula edits,
  copied and pasted cells, undid/redid its own change, downloaded full-range
  CSV and recovered an immutable version. Desktop/mobile layouts and the
  post-restart reload had no page errors. A real Runtime CLI agent received
  only explicitly granted access and updated a record seen immediately in
  the browser. After restart, the shared fixture retained profits 925/300,
  its immutable recovered copy retained profit 950, and its checkpoint remained.
- A copied installation with its own Node runtime passed under a scrubbed
  service PATH. An independent non-root, read-only, network-disabled Docker
  package probe passed with the production allowlist and package assets.
  This was a bounded packaging probe, not a complete account-plane image build.

The independent reviewer closed all reported integrity/access findings before
landing. Smoke used only the isolated `sheetsmvp` / `core_sheets_test` stack;
the founder's live companies and primary checkout were untouched.
