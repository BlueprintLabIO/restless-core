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
- [ ] Verify two real collaborators, structural collision, agent edits,
  worker/service restart, company isolation, history and CSV through the stack.
- [x] Compare the live desktop/mobile interface against the required references.
- [ ] Independent review, final smoke and merge into remote `dev`.

The implementation contract and bounded MVP limits are in
[`../specs/native-sheets.md`](../specs/native-sheets.md). Upstream owns spreadsheet
semantics; Restless owns membership, ordering, durable receipts and history. The
existing Yjs Docs service remains a separate body implementation. Presence,
cursor fanout, Excel fidelity and log compaction are outside this MVP.

Producer evidence: three pinned worker tests; real exec-Actor DB integration
covering atomicity, stale/duplicate receipts, ACL, reconnect generations,
immutable history/recovery and fresh worker replay; real Rust/Postgres WS smoke
with two upstream Models and structural rebasing; browser typed edit persisted
into a full-range CSV download, named history visible, desktop and mobile grid
without page errors. Installed worker with copied Node also passed under a
scrubbed PATH; an independent read-only, network-disabled Docker package probe
passed. Supervisor owns final service restart and exact-source landing checks.
