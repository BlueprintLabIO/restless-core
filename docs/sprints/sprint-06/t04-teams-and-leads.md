# S06-T4 · Teams and accountable leads

**Layer:** OrgIntel. A team is coordination state, not kernel truth — recoverable, overridable,
repairable (`ARCHITECTURE.md` §4.4, §12).
**Serves:** `orgintel` §2.2 (Exec responsibility), §6.1 (minimal primitives), §6.3 (teamwork patterns).
**Depends on:** S06-T3 — teams over non-durable actors are teams of ghosts.
**Makes deletable:** nothing yet. It adds a primitive; §16.5 is paid by T1/T2 this sprint and by T3's
minting paths.

**Implementation evidence:** tracked in the Sprint 06 checklist and
[`run-report.md`](run-report.md), not in a second ticket-status field.

---

## Why a new primitive, when §6.1 says keep the set small

`orgintel` §6.1 lists eleven primitives and does not include `Team`, and `ARCHITECTURE.md` §16.1 says
grow entities only after repeated real scenarios reveal the same need. That bar has to be met
explicitly, not assumed, so:

**Three runs, one unowned job.** Every escalation in the record goes staff → Exec → owner, because
there is nothing else to go to:

- S05-T7: *Kimi exhausted its allowance after 54 tool calls and stopped the singleton Exec.* All
  coordination sat on one actor, so one provider ceiling stopped the company.
- S05-T8: *escalated machine work to the owner.* With the Exec saturated, the next hop up is the human.
- Sprint 05 run report: *Staff reused the company-wide Exec termination question, so a critic that had
  completed its own review marked itself blocked because later company work remained.* A member with
  no bounded coordinator reads company-wide state as its own status.

**And an owner-side job with no owner.** The owner has one addressable actor for thirteen people. To
adjust a critic's brief they must either message the Exec and hope, or message the critic and bypass
whoever is accountable. There is no actor that can both *speak for* a group's state and *change* it.

A relation on `actors` alone (`reports_to`) would carry the first job and not the second: it names a
parent but not a thing with a brief, members, and a state to speak for. That is why this is a `Team`
with a lead rather than a self-referencing foreign key.

## Scope

1. **`Team`** — durable, minimal: id, name, brief (why this team exists and what it is accountable
   for), lead actor, created_by, created_at, disbanded_at. Nothing else until a run asks.
2. **Membership** — an actor belongs to at most one team. The lead is a member of the team it leads.
   Actors with no team are *unassigned*, and that is a normal, displayable state — not a default team.
3. **Work is attributable to a team** through its owner's membership. Work does not carry a team id:
   that would be a second writer of the same fact (`cross-layer-contract` §3.1) and would drift the
   moment a member changes team.
4. **The Exec commissions; the lead assembles.** The Exec turns a company outcome into a team charter
   and appoints its accountable lead. The lead inspects the Work and durable actors, then chooses the
   smallest roster that can achieve the outcome. For every member, the composition record or linked
   team brief states what difference that member buys: role, model, context, prior evidence, skill,
   tool access, or independent capacity. This refines `orgintel` §2.2 rather than contradicting it:
   the Exec still forms and remains accountable for the company's internal organisation while
   delegating bounded roster composition to the appointed lead.
5. **The lead reshapes its own one-level roster.** It may add or release an unassigned durable actor,
   create one stable specialist through T3's ordinary actor path when no existing actor fits, and
   replace a member when evidence shows a capability or availability gap. It may not take an actor
   already committed to another team, replace itself, create a sub-team, or silently widen the Exec's
   outcome charter. Releasing a member with active Work requires the lead to reassign or explicitly
   settle that Work first, so attribution cannot disappear as a side effect. Cross-team staffing and
   a changed charter go back to the Exec.
6. **The owner can override**: rename a team, change its brief, move a member, replace a lead,
   disband. Overriding is ordinary coordination, not an approval-gated act.
7. **Read model + bindings.** `TeamRow` derives `ts_rs::TS` and regenerates through the S05-T4 seam;
   the cockpit projection carries teams and each person's team and lead flag. `restless teams` lists
   them on the CLI, which remains the complete administrative surface (S05-T2).

The composition explanation begins as an ordinary team brief or Work-linked file, not a capability
ontology, staffing API, or permanent score. The run must make the lead's judgement inspectable; it
does not need a new entity to do so.

## Explicit non-authority

**A team lead is coordination, not authority.** It grants no effect permission, no budget, no credential
scope and no approval right. A lead cannot approve what its members could not approve; the owner
approval boundary is byte-identical before and after this ticket. `authority-plane` is untouched, and
no kernel record gains a team field.

This is the single most likely place for the sprint to drift, because "lead" carries managerial
connotations that map onto permission. It does not map. If a team ever needs a budget, that is a new
ticket with its own evidence, argued against `authority-plane` §2.2.

## One level, deliberately

A lead does not lead leads and cannot form a sub-team. It assembles only the direct members of the
team the Exec commissioned. If a run shows a lead saturating the way the Exec did, that is the
evidence for a second level — and it will be the same argument this ticket makes, made again with
data. Building two levels now is the speculative version.

## Verification

A `_test` company: the Exec commissions two outcome charters with distinct leads. Each lead sees the
durable actors before creation, assembles two direct members, and records why each member is present.
Assert a lead can claim and release an unassigned actor; cannot take a member of the other team or
replace itself; cannot release a member while leaving its active Work unattributed; and can create
one durable missing specialist without minting another on revision.
Assert Work is attributable to exactly one team via its owner; move a member through an authorised
override and assert attribution follows; disband and assert members become unassigned rather than
orphaned. `cargo test -p restless-orgintel` regenerates bindings clean. `restless teams` prints the
same structure the cockpit projection carries — one canon, two readers.

## Risk disposition

- **`Team` becomes an org-chart feature nobody's work needs** — *pending fix*: the sprint's success
  contract judges it by T5's behaviour (lead resolves an escalation, lead changes work on owner
  instruction), not by the table existing. If the run cannot demonstrate both, the primitive is
  deleted, not kept and grown.
- **"Lead" leaks into authority** — *invariant*. No kernel record gains a team field. This is the one
  thing in the sprint that does not get relaxed under schedule pressure.
- **Teams drift from the actual Work graph** — *accepted*. They are coordination state and are
  repairable and regenerable by design (§4.4); they are not governance truth.
- **A lead adds agents instead of improving the work** — *guarded*: every member needs an explicit
  difference and output, and the sprint compares accepted outcome, owner interventions, cost, and
  latency. Headcount without a useful difference is a failed composition.
- **Two leads compete for one specialist** — *guarded*: an actor remains in at most one team and a
  cross-team staffing need reaches the Exec for a company-level trade-off.
- **An actor needs to be in two teams** — *accepted for now*, with option 2 in the sprint's founder
  decisions naming the revisit condition.


---

## What landed

Migration `0007_teams_and_escalation.sql`, plus:

| Layer | Added |
|---|---|
| Schema | `teams` (id, name, brief, `lead_actor_id`, created_by, created_at, disbanded_at); `actors.team_id`; partial unique index on live team names |
| `OrgIntel` | `create_team`, `list_teams`, `set_actor_team`, `set_team_lead`, `disband_team`, `team_lead_for` |
| Types | `TeamRow` with `ts_rs::TS`; `ActorRow.team_id`; regenerated through the S05-T4 seam, so `web/` sees them without hand-editing |
| CLI | `restless teams list \| create \| update \| assign \| lead \| disband`, resolving a team by name *or* id |
| Daemon | `teams`, `team-create`, `team-assign`, `team-lead`, `team-disband`; `people` rows now carry `team_id` |

Three refusals are enforced in `OrgIntel`, not left to callers:

- **The owner cannot lead a team.** A team exists so judgement stops reaching the owner; a team led by
  the owner routes straight back to them and is a contradiction, not a configuration.
- **The owner is not staff** and cannot be assigned to a team.
- **A team name is unique among live teams only.** Disbanding releases the name and keeps the record.

`disband_team` returns the number of pending judgements reassigned to the Exec, so disbanding can
never silently swallow a queue or jump ordinary organisational guidance straight to the owner.

The roster paths are role-bounded: the Exec/owner commissions or overrides; an appointed lead may
claim only an unassigned actor into its own team or release its own direct member. Cross-team
poaching, nested leads, self-release, and release with unsettled Work are refused. Team rename and
brief changes are attributable owner/Exec operations.

Live Aris closes scope items 4–5. The Exec commissioned `centre-site-validation` with a concrete
charter and appointed `staff-site-validation-lead`. That lead inspected the durable actor pool,
selected two existing specialists, and recorded the distinct evidence/capability each bought in both
roster events and `team-graph.md`. The Exec did not assemble the roster.

## Verification

`cargo test -p restless-orgintel` against the live Postgres (`RESTLESS_TEST_DATABASE_URL=postgres://…`):
bindings, smoke round-trip, and the S06-T5 escalation scenario all pass. The migration was
additionally applied to a **populated** pre-0007 schema seeded from a real company's migration
history — 1 existing handoff and 3 actors preserved, `assigned_to`/`team_id` reading as NULL, which is
"the owner" and "unassigned" respectively.
