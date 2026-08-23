# v18 — model pool exists, but failure evidence does not reach coordination

## Change under test

Keep the accountable Work owner stable while selecting the free model for each Attempt from a
recorded per-actor pool. Attempt revision determines the model deterministically, so repair is an
explicit OrgIntel decision while provider substitution stays a Runtime launch concern.

Preflight: Python compile passed; coordination/adversarial suite 33/33; SQLite quick check OK.

## Evidence

- Resumed the exact v17 state and Work `work-b53ec0f739`; no Work or Attempt history was reset.
- Exec launch: `nvidia/nemotron-3-super-120b-a12b:free`, with live zero-price/text/tool proof at
  `2026-08-22T00:47:45.010Z`.
- The v17 Attempt rows exposed only the generic summary `Actor process ended as error without a
  terminal report`; the actual 429, model ID, and Google AI Studio failure domain existed only in
  the Runtime turn transcript.
- Exec retained `gameplay-systems` as owner, but sent it a diagnostic message instead of repairing
  the Work. The command returned `delivery: next_wake` and created outbox row 9.
- No ready Work existed to wake that actor, and the comparison runner cannot launch a free-form Staff
  conversation wake. With no active tasks, it waited indefinitely on an undeliverable diagnostic.
- The run was stopped after this state was observed. No revision-5 Attempt launched, so the new model
  pool was not credited as working.
- The Exec trace is 21,818 bytes; actor ownership, four prior unknown Attempts, clean workspace, and
  empty artifact set remain intact. SQLite quick check is `ok`.

## Score

Outcome score: **30/100** (no-artifact cap 39).

| Dimension | Points | Evidence |
| --- | ---: | --- |
| Accepted outcome /30 | 0 | no new Attempt or artifact |
| Coordination /20 | 7 | ownership stayed stable, but a diagnostic message became an unschedulable dependency |
| Recovery/truth /15 | 10 | exact Work/Attempt history and workspace survived; no completion was inferred |
| Review/evidence /15 | 0 | no candidate existed |
| Efficiency/attention /10 | 3 | stopped after one wake rather than rotating actors/models blindly |
| Harness/control /10 | 10 | exact free proof, launch contract, ordered trace, message and outbox state retained |

## Dominant failure and 10x decision

Runtime failure evidence was durable but not projected onto the organisational object responsible
for recovery. Asking another model to rediscover a transport error is waste: the Runtime already
observed it.

Record a bounded runtime outcome, exact model and safe error excerpt on an Attempt when a process
ends without its terminal callback. Exec can then judge repair directly. Also make clear that a
`next_wake` message is context for a future actor wake, not an immediate diagnostic RPC; it must not
keep an otherwise idle comparison run alive.
