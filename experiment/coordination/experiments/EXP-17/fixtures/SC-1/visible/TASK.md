# Outcome: make callback terminal state safe to enable

You own this small service end to end for the operations engineer who must decide whether provider
callbacks can be enabled. Diagnose and repair the callback store without broadening its public API.

Required behaviour:

- each provider `event_id` has an effect at most once, including after process restart;
- an operation exposes one materialized terminal outcome and one keyed terminal outbox entry;
- a callback with a lower `sequence` than the stored operation cannot overwrite newer evidence;
- a newer sequence may replace the materialized outcome and the existing keyed outbox entry, but must
  not append a second terminal delivery;
- persistence remains atomic and valid first delivery behaviour is preserved.

Run the visible tests, add proportionate regression coverage for your diagnosis, and leave the working
tree with the repaired implementation. Write `RESULT.md` with a concise diagnosis, changed invariant,
commands run and result. Do not inspect paths outside this fixture, use network access, or perform any
external effect.
