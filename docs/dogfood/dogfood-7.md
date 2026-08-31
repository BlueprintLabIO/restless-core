# Dogfood 7 — Aris recurring sales operation

**Company:** Aris  
**Scenario version:** 0.1  
**Activated:** 2026-08-31  
**Status:** active; first natural scheduled occurrence is pending

## Outcome charter

Aris sustains a small, high-quality tutoring-centre outreach operation without an owner manually
starting each day. At 09:00 on Sydney weekdays, Restless wakes the accountable commercial lead to
inspect current evidence. The timer runs no sales command and grants no authority to send.

The lead processes replies, bounces and opt-outs first, reconciles Attio and provider receipts,
maintains a buffer of ten qualified unsent prospects, and may commission up to five eligible sends
staggered through the local morning. Suitable private tutors may qualify when they operate publicly
as a business and publish a relevant business contact. The owner-approved sales playbook and
three-touch per-address limit remain controlling operating state.

## Evaluation question

Can one timezone-aware recurring actor wake sustain this real sales cadence while remaining
idempotent across repeated owner commands, restarts and missed weekdays—and without turning the
schedule itself into permission or production logic?

## Acceptance

1. The owner can install one weekday schedule using an IANA timezone and local wall-clock time.
2. Repeating the exact installation command returns the existing live schedule.
3. One due occurrence creates one durable addressed wake fact and advances to the next Sydney
   weekday, including across daylight-saving changes.
4. Missed weekdays coalesce; they are not replayed as a burst.
5. The schedule contains no executable command and creates no email effect by itself.
6. The accountable actor can cancel the schedule explicitly while fired history remains inspectable.
7. A natural 09:00 occurrence is observed end to end in Aris before this dogfood is called complete.

## Review target

The native review surface is `restless schedule list -c aris --as opportunity-analyst`, supported by
Aris Attention/Work, provider receipts and the run log. A green unit test alone is not completion.

