# Dogfood 7 run log

## 2026-08-31 — activation

The live Aris OrgIntel database retained its company history but initially refused startup because
migration 20's recorded checksum predated the committed file. The live schema was checked for all
three migration-20 tables and its terminal columns before the checksum was repaired to the committed
SHA-384. Migrations 21 and 22 then applied normally. This was a pre-existing migration-discipline
defect exposed by the dogfood, not evidence for recurrence.

Aris already had a `contact` team. Its accountable lead is `opportunity-analyst` (Maya Chen), with
`opportunity-writer` (Noah Park) as producer. No new team or actor was invented.

The owner CLI installed:

- schedule: `45155efc-fa27-4347-9459-0da474850873`;
- recurrence: weekdays;
- local time: `09:00`;
- timezone: `Australia/Sydney`;
- first natural fire: `2026-08-31T23:00:00Z`, Tuesday 1 September at 09:00 AEST;
- target: `opportunity-analyst`.

Repeating the exact installation command returned the same UUID with `created: false`. A native list
read showed one live recurring row plus the earlier one-shot follow-up scheduled for 4 September.
The latest `customer-contact.email` receipt remained dated 28 August; activation created no email,
draft, sequence or provider call.

Focused verification passed:

- two timezone tests: Sydney weekend rollover and daylight-saving offset;
- live-Postgres `company_schema_round_trip`, including duplicate installation, one due delivery,
  advancement rather than replay, persistence as a live recurrence and cancellation;
- existing daemon scheduler tests: 8 passed;
- wire decoder tests: 6 passed;
- targeted crates compiled with `cargo check`.

The first natural scheduled wake remains deliberately pending. Dogfood 7 is active, not complete,
until that occurrence is observed producing one lead wake, no automatic send and the next weekday
deadline.

The activation used an isolated owner-plane port because a separate quality-enforcer test plane held
the default local owner port. The schedule itself is stored in Aris's real cell database and survived
a daemon restart. The isolated daemon was stopped after verification rather than left as hidden test
infrastructure. A normal Aris daemon must be resident at 09:00 for an on-time wake; otherwise restart
reconciliation should coalesce the missed occurrence, which is useful recovery evidence but not an
on-time pass.
