# EXP-12 run log

This log separates observed treatment behaviour from later repairs. Times in OrgIntel evidence are
UTC; the founder ran the experiment on 29 August 2026 AEST.

## Frozen treatment

- Company: `exp12_attio_test`, cloned with live authority and credentials stripped.
- Owner request, delivered unchanged: “Set up a CRM for our tutoring-centre sales team, load the
  supplied unsent prospect set and give the sales lead a useful pipeline with ownership and next
  actions. Do not send anything.”
- Fixture: `tutoring-centre-prospects.json`, 20 eligible prospects and three explicit exclusions.
- Fixture SHA-256: `508967eebb426e63bb56d64ccae00b30733097e7f710a70ec06d7dc8405a2211`.
- Provider path selected by the company: Attio official remote MCP at
  `https://mcp.attio.com/mcp`; live provider metadata advertised `openid`, `offline_access` and
  `mcp`.
- Connection assignment: only actor `crm-operations` for the frozen CRM Work.

## Organisational result before connection

Exec created one accountable sales-operations lead (`sales-operations`, Mira Chen). The lead
commissioned CRM production to Staff (`crm-operations`, Jordan Patel). The original Work
`ebc5691f-f865-402c-8b31-d8389b5a9ea7` exhausted three Attempts and was preserved as abandoned
failure evidence. After observed platform repair, the lead created successor Work
`e761efee-b67b-4cda-85a8-5dcf53e6132d`; its first Attempt is
`4e073c90-9c36-41da-8600-12eb84f369e2`.

The first configured model, `zai/glm-5.3`, refused the request with a provider quota response before
performing work. The disposable company was changed to `openai-codex/gpt-5.6-sol` and the exact
owner request was redelivered. Count this as one technical setup intervention and do not attribute
it to the connected-tool mechanism.

## Observed product friction and repair

1. The CLI emitted one `purpose` key into two flattened wire structs. Serde consumed it in the wrong
   struct, so every fully populated install request was rejected as missing its purpose. The wire
   contract now has one authority-owned purpose field. A focused decode test and the connected-tool
   tests passed after repair.
2. Runtime-local `restless doctor` probed the host owner gateway at container loopback and reported a
   false outage. This caused repeated, reasonable Staff refusal to consume another Attempt. Runtime
   doctor now marks host-only owner surfaces `not_observed`; the owner-side doctor retains the full
   browser-to-Runtime probe.
3. OAuth discovery accepted the first HTTPS text printed by the mature helper, which was the Attio
   origin rather than the complete authorization URL. The pending handoff was refreshed in place
   with the actual provider authorization state. Parsing now waits for the helper's explicit
   authorization prompt; a regression test proves discovery URLs are ignored.
4. A manual baseline probe still held the helper's fixed localhost callback port. Only that
   disposable probe was stopped; the treatment helper then acquired the port. Record this collision
   as baseline contamination and an owner-intervention risk for concurrent installs.

At this checkpoint, provider connection `attio` is `awaiting_owner` under handoff
`1935804a-e3af-4f34-b341-76a1faa8b1b3`. The exact Attio authorization page is prepared and the
callback observer is live. No authenticated Attio tool list, CRM record, or outreach has yet been
observed. The experiment is therefore still running.

## Verification run after the first repairs

```text
cargo test -p restlessd connected_tool -- --nocapture
4 passed; 0 failed

cargo build -p restlessd -p restless
finished successfully

cargo test -p restless --no-run
finished successfully

RESTLESS_HOME=/tmp/restless-exp12 restless doctor -c exp12_attio_test
status: live before the OAuth handoff
```

Build and unit checks prove the repaired local contracts only. They do not prove connection,
workspace identity, CRM usefulness, reconnect, removal, or founder acceptance; those remain open
until native evidence is attached below.

## Founder amendment: bounded Aris Academy test data

The founder later chose the existing Aris Academy workspace instead of creating a disposable Attio
workspace. The Company Runtime remained the isolated `exp12_attio_test` company. The amended provider
boundary allowed only unmistakably labelled EXP-12 test records and prohibited outreach, drafts,
sequences, purchases, upserts, merges, updates to pre-existing records and use of the separate
`attio-disposable` connection.

This is a deliberate deviation from the original fresh-provider-workspace risk treatment. It does
not invalidate the connected-tool observations, but it means this run cannot prove that the
fresh-workspace provision was obeyed. The provider-side test data must be judged in that disclosed
context.

## Duplicate OAuth and stale-Work incident

The founder completed Attio OAuth once. Restless nevertheless projected another consent action. Two
independent defects caused it:

1. forced reconnect replaced the working credential directory before the replacement flow succeeded;
2. abandoning superseded Work did not withdraw its still-pending owner handoff.

The connection was also attached by durable actor rather than exact Work/Attempt, leaving unrelated
or stale Work for the same actor eligible to receive it. During the repair, the working Runtime token
was recovered into the isolated host credential directory; no token content was printed or committed.
The duplicate handoffs were explicitly withdrawn and Attention returned to zero items before CRM
work resumed.

The repaired implementation now:

- stages the exact assigned Work and Attempt beside the actor;
- attaches an enabled MCP only when actor and Work match, then binds it to the exact launching
  Attempt;
- gives conversational, Exec and unrelated Work sessions no Work-scoped connection;
- makes reconnect transactional by restoring the prior credential after a failed or interrupted
  replacement; and
- withdraws pending owner handoffs in the same transaction that abandons their Work.

This is a material execution fence, but not an absolute per-provider-call direction epoch. A remote
call already accepted by a provider may still finish while local cancellation propagates. Record a
call-time epoch/precondition proxy as the remaining invariant before claiming that stale Work can
never cause an external effect.

Focused verification after the repair:

```text
cargo test -p restlessd connected_tool -- --nocapture
7 passed; 0 failed

cargo check -p restlessd -p restless-orgintel
finished successfully

cargo build -p restlessd
finished successfully

RESTLESS_TEST_DATABASE_URL=<isolated live test database> \
  cargo test -p restless-orgintel \
  superseded_work_is_abandoned_with_attribution_but_never_while_running -- --nocapture
1 passed; 0 failed
```

The first attempted DB run used the repository's stale `DATABASE_URL` and failed authentication. It
is not counted as evidence. The stated pass used the isolated EXP-12 daemon's actual database without
printing its credential.

## Native Attio outcome

Work `889c6e5b-e665-47f9-a3f0-ff677fe73237`, Attempt
`3a67f7dc-87e6-4411-b597-0d84b7e70776`, ran with the recovered connection. Its first native call was
`whoami`, which observed workspace `Aris Academy`, authenticated email `yao@aris-academy.com`, member
`c44dbba4-1584-46be-b063-5baeacfa5fc2` and admin access. Restless recorded 39 provider tools and the
workspace reference before disabling the connection at the end of lifecycle verification.

The provider result is one native list:

- name: `EXP-12 TEST — Tutoring Centre Prospects — 2026-08-29`;
- list ID: `08268d90-cccd-4060-8d44-806e5bed7335`;
- native target: `https://app.attio.com/aris-academy/list/08268d90-cccd-4060-8d44-806e5bed7335`;
- 20 new People records, all visibly prefixed `EXP-12 TEST —`;
- 20 linked incomplete tasks, one per record, assigned to authenticated admin Yao Ke;
- exact task text `Review fit and prepare an unsent first-contact draft` and deadline
  `2026-09-04`; and
- the fixture's `Sales lead`, `Qualified — uncontacted`, next action, due date and incomplete state
  preserved in visible record descriptions.

Attio exposed no writable People owner field and no workspace member named `Sales lead`. The run did
not invent a second owner schema: it preserved `Sales lead` visibly and assigned executable tasks to
Yao Ke. The founder accepted this disclosed ownership representation on 29 August 2026 through
handoff `99f9d552-88d0-47e5-8ed5-e09f31d52ece`.

The attributable ReviewTarget is
`/company/outputs/exp12-aris-native-review-target.json`, SHA-256
`bed8f4ede2e251903de38e26c0a46374e3d32e85f12d36fb1fcdd808d4bb5af3`. The Work artifact records the
same digest. Independent transcript reconciliation observed:

```text
fixture SHA-256: 508967eebb426e63bb56d64ccae00b30733097e7f710a70ec06d7dc8405a2211
fixture prospects / artifact records / unique record IDs / unique task IDs: 20 / 20 / 20 / 20
fixture-to-artifact email differences: 0
excluded-address overlap: 0
record readback: 20 owner, next-action, due-date and incomplete markers
task readback: 20 IDs, exact actions, deadlines and incomplete states
native list readback: 20 entries, has_more false
native excluded-address search: 0 records, has_more false
native post-create email search across all 20 addresses: 0 emails, has_more false
```

The observed MCP surface contains no send-email, create-email-draft or sequence-enrolment tool. This
proves no such tool was available or called and that native post-create email activity was empty; it
does not prove absence of provider object types that the MCP cannot query. Attio automatically added
some company associations based on domains without a company write call. No outreach was sent.

Two governed `create-list` transports returned `Connection closed` and therefore have failed effect
receipts. Native reconciliation after the second failure observed exactly one list created during
that call's window. This is a real ambiguous-effect defect: provider completion outran the transport
receipt. The worker correctly reconciled before continuing, but future provider writes need a
provider idempotency key or a first-class interrupted-effect reconciliation path rather than a retry
whose local receipt says failure.

## Attachment and removal observations

The productive CRM Attempt received the Attio MCP. Fresh sales-lead and Exec review sessions launched
while the connection was enabled contained no Attio MCP tool calls, proving the actor/Work negative
scope in the live run.

After the CRM result was preserved, the connection was disabled. The accountable lead commissioned a
new bounded CRM lifecycle probe as Work `a735145c-23af-41ba-9a0b-32d5e9d8ff34`; its fresh Attempt
`96828cf1-4182-4cbc-b114-00ae0212e240` observed:

```json
{
  "attio_tool_count": 0,
  "attached_attio_tools": [],
  "attio_capability_usable": false,
  "external_calls_performed": 0,
  "crm_mutations_performed": 0
}
```

The evidence file is `/company/outputs/exp12-disabled-connection-fresh-session.json`, SHA-256
`b7636297b23de5eb8d98833c7d4400a34302514737b2c6185e9663fb7cf2f87a`. The actor linked the artifact
without its digest even though its resolution claimed a digest; the hash above was independently
observed and the missing artifact metadata is recorded rather than silently repaired.

## Treatment result and Nango disposition

The generic connected-tool mechanism is functionally promising: the official remote MCP exposed a
useful native CRM, exact Work-scoped attachment worked after repair, cached reconnect avoided a second
consent, the provider stayed source truth, and disable removed the capability from a fresh session.

The autonomous-installation treatment **did not pass**. The owner performed one legitimate provider
OAuth action and no CLI/config work, but a technical operator repaired four defects, stopped a
contaminating helper and manually restored the already-authorised credential before the productive
Attempt. The run therefore cannot support the central claim that one plain-language request reaches a
useful connected tool without developer access. Preserve it as a repaired mechanism win and a
treatment loss; rerun only after the remaining stale-effect and ambiguous-write boundaries are fixed.

The conditional hosted-Nango probe is not activated. The direct local treatment did not pass its
no-developer-access gate, and Cloud's own current entry rule keeps Nango deferred until repeated
provider-connection maintenance demonstrates that a narrow Authority-owned adapter is insufficient.
Current Nango documentation shows potentially useful per-customer OAuth, token refresh, provider
proxying and MCP exposure, but there is no configured Nango environment, secret or Cloud host in this
workspace. Creating one now would add a second auth/setup experiment and would not repair the failed
local treatment.

## Requirement-by-requirement completion audit

This audit keeps the original contract visible even where the founder amendment made a safer but
different workload authoritative.

| Requirement | Authoritative evidence | Disposition |
| --- | --- | --- |
| One plain-language owner request starts installation | Frozen request and first Work graph | **Observed**, but later owner amendments and technical repair mean it did not remain a one-request treatment |
| Exec independently finds and validates the official path | Official endpoint discovery and staged `attio` connection | **Observed with intervention**; OAuth URL parsing and callback-port contamination required repair |
| One bounded identity/consent Attention item | One legitimate OAuth completed by the founder | **Failed**; stale Work and destructive reconnect generated duplicate consent requests before withdrawal |
| Automatic observation and resume | Authenticated tool probe and later fresh Attempt | **Partial**; callback observation worked, but the productive resume required manual credential recovery |
| Exact fresh-session attachment | Attempt `3a67f7dc-87e6-4411-b597-0d84b7e70776` received 39 Attio tools; lead/Exec sessions did not | **Passed after repair** |
| Intended workspace and identity | Native `whoami`: Aris Academy, `yao@aris-academy.com`, admin | **Passed** |
| Supplied source, qualification, tier, owner, status, next action and due date retained | Native record readback contains 20 source, qualification, tier, owner, next-action, due-date and incomplete markers | **Passed under create-only amendment** |
| Qualified accounts with incomplete contact and action due this week | 20 records and 20 incomplete tasks due `2026-09-04` | **Passed** |
| Controlled positive and not-interested outcomes | No provider mutation was authorised beyond creating the frozen test queue | **Not performed**; superseded by the stricter create-only amendment |
| Duplicate account/contact does not appear | Pre-write collision search returned zero; final IDs/emails are unique and exactly match the 20-row fixture | **Partial**; no deliberate duplicate write was attempted, so provider-side duplicate handling was not proved |
| Pipeline summary with direct native links | Exact list URL plus all 20 record/task IDs in the ReviewTarget | **Partial**; one native list link exists, but per-record URLs were not materialised |
| Native founder acceptance | Handoff `99f9d552-88d0-47e5-8ed5-e09f31d52ece`, resolved `2026-08-29T02:36:00.352782Z` | **Passed**; the founder accepted the exact native list and disclosed ownership representation |
| Disable prevents next fresh-session use | Work `a735145c-23af-41ba-9a0b-32d5e9d8ff34` observed zero tools/calls/mutations | **Passed** |
| Provider remains domain source truth | Core retains generic connection metadata, Work/effect references and Runtime evidence only | **Passed**; `attio` appears in production crates only in generic wire/parser test fixtures, not provider semantics |
| No outreach | MCP surface had no sending/draft/sequence create tool; zero calls; post-create native email search returned zero | **Passed within the observable boundary** |

## Founder judgement

The founder accepted the native 20-record Attio outcome after opening the exact prepared list. This
acceptance closes the native-usability judgement; it does not change the experiment result. EXP-12
remains a **provisional loss** because the autonomous-installation treatment required technical
repair, helper cleanup and manual credential recovery. The repaired mechanism is useful evidence,
not proof of the one-request/no-developer-access hypothesis. The stable review record is
[`FOUNDER_REVIEW.md`](FOUNDER_REVIEW.md).

## Post-review operating reconciliation — excluded from EXP-12 metrics

After accepting the isolated test outcome, the founder asked Restless to record the three tutoring
centres that had already been contacted. This was separate operating Work, not a continuation or
rerun of the frozen experiment. Restless used the cached Work-scoped Attio connection and created one
native list, `Tutoring Centre Outreach — Operating`, containing exactly:

- BrainTree Coaching Australia `<hello@braintreecoaching.com.au>`;
- Pre-Uni New College `<info@newcollege.com.au>`; and
- Matrix Education `<info@matrix.edu.au>`.

Each record visibly preserves `Sales lead`, `Contacted — awaiting reply`, two of three allowed email
touches, one remaining allowance, the no-automatic-send next action and due date `2026-09-04`. Each
has one internal note containing the two observed send events and one incomplete reply-check task
assigned to Yao Ke. Provider-accepted events were not upgraded to delivered. Routing-test traffic
from `yaillives@gmail.com` was excluded.

Native reconciliation observed one exact operating list, three entries, three notes, six history
events, three incomplete tasks, twenty unchanged EXP-12 test-list entries and zero record overlap.
The live Aris Authority receipt count for `customer-contact.email` remained 25 before and after the
reconciliation, so no new email was sent. Attio was disabled afterward; a fresh Attempt observed zero
attached Attio tools and made zero provider, browser, network, effect or CRM calls. The declared gate
passed in run `5f3adc32-97f4-4afa-bf9f-1439b83e799f`.

The attributable ReviewTarget is
`/company/outputs/exp12-contacted-centres-operating-review-target.json`, SHA-256
`f9b9c294b600416196c68f7169e91d5248c5b21f2e958963c482cbe0d5dffe24`. Its native list is
`https://app.attio.com/aris-academy/list/7fa7f999-7506-4556-87a7-905e21079799`. Owner review remains
open under handoff `1570b4f7-16d8-422a-9a3d-47d6b6428685`; this later operating judgement is not an
EXP-12 acceptance criterion and must not be folded into its treatment measurements.
| No live-company test pollution | Isolated Restless company was used, but founder explicitly chose the real Aris provider workspace for labelled test data | **Amended, not passed as originally written** |
| Hosted Nango implementation probe after a passing local treatment | No Nango environment or Cloud host; local autonomous treatment failed | **Not activated** under the decision rule |

Original Treatment E tasks to upsert, mutate controlled outcomes and exercise an intentional duplicate
were not silently reinterpreted as complete. The founder's later create-only/no-existing-record-change
boundary took precedence. The 20-record provider result is useful, but it is not evidence for those
three omitted mutation behaviours.

## Measurements

- Full observed run window: 41,346 seconds (11h 29m 6s), including founder pauses, repairs and blocked
  Attempts.
- Productive CRM Attempt wall time: 1,493 seconds (24m 53s).
- Across the full observed window, persisted model transcripts record 436 assistant turns,
  3,510,258 uncached input tokens, 111,001 output tokens, 22,162,688 cache-read tokens and a nominal
  transcript cost of USD 31.962664. Recorded assistant execution duration totals about 3,790 seconds.
- The productive CRM Attempt records 98 assistant turns, 341,956 uncached input tokens, 23,904 output
  tokens, 9,218,560 cache-read tokens, 9,584,420 total processed tokens, about 865 seconds of assistant
  execution and a nominal transcript cost of USD 7.03618.
- Restless accounted spend is USD 0 because the selected Codex OAuth/subscription path is not a
  metered-API charge. Nominal transcript cost and Authority-accounted spend are different measures;
  neither should be substituted for the other.
- Provider charge: no plan purchase or charge observed; billing itself was not probed.
- Owner actions: one real Attio OAuth, one explicit amendment authorising labelled Aris test records,
  and one rejection of the duplicate OAuth request are attributable. Active owner minutes and exact
  provider clicks were not instrumented and remain unknown.
- Technical interventions: model replacement after provider quota refusal, four connected-tool/doctor
  repairs, callback-helper cleanup, isolated-daemon restart, manual recovery of the existing token,
  stale-handoff withdrawal, Work feedback/resume and final connection disable. The count is reported
  categorically because shell-level operator actions were not instrumented as one stable event type.

## Promotion and deletion decision

No provider catalogue, CRM schema, Nango dependency or Attio domain model is promoted into Core. The
generic connection record, exact Work/Attempt attachment, non-destructive reconnect and stale-handoff
withdrawal remain candidate repairs because each serves an observed failure and passed focused
verification. They are not release evidence until integrated through the founders' normal sprint
path.

The run strengthens the case for deleting or avoiding local CRM/domain-state machinery: Attio owns
the list, People, tasks and email activity, while Restless needs only a generic connection reference,
authority/effect evidence and native ReviewTarget. The old actor-wide attachment rule and destructive
credential replacement are made deletable by the repaired exact-scope and transactional-replacement
paths. The duplicate-consent handoffs remain historical evidence but no longer project owner action.

## Remaining founder judgement

The lead and Exec have prepared owner handoff `99f9d552-88d0-47e5-8ed5-e09f31d52ece`. The exact
irreducible question is whether the visible native list is acceptable with `Sales lead` represented in
record descriptions and Yao Ke owning the native tasks. A controllable signed-in browser was not
available during the final preparation, so native visual acceptance remains pending. Provider state,
the Work outcome and experiment status must not be called accepted until that judgement is recorded.
