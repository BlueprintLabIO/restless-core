# S57-T12 — Telegram Attention channel

**Layer:** Kernel (pairing, decision binding, delivery claim in the Authority store) with a thin
Cockpit control. **Serves:** the owner sees Attention only while a browser tab is open. Approvals
then wait for the next visit, and the owner chose Telegram as the first channel outside the browser.
This is a roadmap item outside the sprint theme, recorded here so its status has one home.

`telegram` (engine) owns pairing, the outbox claim, the callback binding and the decision path.
`owner_telegram` (owner API) owns `GET/POST/DELETE /companies/{company}/telegram` and the poll and
delivery loops. The Limits page has a Telegram section. See ADR 0015.

**Deletes:** nothing. Cloud's email projection remains for hosted members.

**Evidence:** `telegram::tests::{only_the_paired_chat_can_decide, a_button_decides_only_its_own_item,
a_replayed_callback_does_not_decide_twice}` against Postgres and a fake Bot API in-process. Still
open: a real BotFather bot pairing, a message and a decision on a `_test` company.
