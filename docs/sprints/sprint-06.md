# Sprint 06 — A self-running team with an accountable lead, and an owner surface that shows it

**Status:** Implementation and live evidence in progress. T1–T4 are complete. T5's runtime and
routing are implemented and the Aris run proved commissioning, lead-led assembly, Goal-linked graph
kickoff, owner → lead delivery, member → lead evidence delivery, local repair, provider failover, and
a direct lead reply. The run also exposed premature lead acceptance and cumulative-usage
over-accounting. Both mechanisms are repaired: graph state now outranks prose, the historical spend
is corrected append-only, and the owner-authorised ceiling is `$200`. The resumed exact-commit chain
has completed deterministic, rendered-page, primary-source, independent-critic, accountable-lead,
and corrected-debrief evidence at `148efbf`. T5 remains partial until a
genuine outside-charter question completes the live lead → Exec → owner return path. T6 is implemented and builds cleanly; its
authenticated browser rendering/send check remains open.
**Date:** 17 August 2026
**Spec refs:** `orgintel` §2.2 / §5.1 / §6.1 / §6.3 / §6.3.1 / §7.1,
`owner-cockpit` §2.7 / §5, `cross-layer-contract` §3.1,
`ARCHITECTURE.md` §4.4 / §16.1 / §16.2 / §16.5

---

## Research notes

The [skills, expertise, and powerful-teams research dossier](sprint-06/research/README.md) reviews the
current Agent Skills ecosystem, registries, skill trust, multi-agent evidence, human-team research,
and implications for OrgIntel. The dossier is evidence rather than a separate ticket; the outcome,
success contract, and founder decisions below define the Sprint 06 implementation scope.

## The audit behind T4/T5

[`sprint-06/audit-what-reaches-the-owner.md`](sprint-06/audit-what-reaches-the-owner.md) traces every
path that consumes owner attention. Its finding sets this sprint's scope: exactly two things reach the
owner, and of the six owner-handoff categories, **`owner_judgement` is 100% of what live Aris has ever
sent** — while the five irreducibly human categories have never once been used. A pending handoff also
*stops* its Work, so every judgement the company cannot make itself halts a Work node until one person
answers. That is the thing that does not survive headcount growth.

## Outcome

> **Aris runs its next revenue step through a named team with an accountable lead. The lead absorbs
> the outcome charter from the Exec, assembles and reshapes the smallest useful team, drives its Work,
> repairs failures, and applies evidence from review to the next attempt. The lead asks the Exec for
> guidance only when the need is outside the team's charter or crosses team boundaries; the Exec
> resolves it or brings a prepared judgement to the owner. The owner — opening the People surface —
> talks to that one lead, is answered on behalf of the whole team, and sees the lead change its own
> team's work.**

The organisational result is the sprint outcome. A `teams` table, a tree in a sidebar, or a passing
migration test does not complete Sprint 06 (`ARCHITECTURE.md` §16.2).

### Operating loop

```text
Exec commissions outcome + charter and appoints lead
→ lead assembles the smallest differentiated roster and Work graph
→ graph readiness starts members; artifacts and evidence return through Work
→ lead resolves blockers, revises the graph or roster, and verifies the real outcome
→ failed review changes the next Attempt and leaves a small evidence-backed learning note
→ need outside charter goes lead → Exec → prepared owner judgement only if still unresolved
→ owner may talk to the lead directly; the lead changes team Work
```

The deterministic part is readiness, dependency release, attribution, and resumption. Team assembly,
repair, and guidance remain model judgement over free-form messages and real artifacts. This is not a
new command algebra or fixed workflow.

### Success contract

The sprint passes when one observed run demonstrates all of the following:

1. Aris's staff actors are **durable**: the same actor takes a second and third Work assignment across
   revisions. No new actor is minted per revision, and the `-v9` / `-v7` / `2` suffix family stops
   appearing. The People list shrinks because identity became stable, not because rows were hidden.
2. The Exec commissions at least one outcome to a **named lead**, and that lead assembles at least two
   members from durable actors. The record explains what difference each member buys — role, model,
   context, prior evidence, skill, tool access, or independent capacity — rather than treating
   headcount as capability. Every member's Work is attributable to the team from OrgIntel alone.
3. Ready Work starts its responsible member through the Work graph; the lead does not manually relay
   every kickoff or handoff. The lead remains accountable for the graph, not for narrating it.
4. A member escalates a blocker and **the lead resolves it**, not the Exec and not the owner. The
   record shows the escalation reaching the lead, the lead's adjustment (reassignment, revision,
   brief or dependency change, roster change, or a new Work node inside its team), and the member
   resuming.
5. A real check, critic, or review finds an inadequate result and the lead **changes the mechanism** —
   team composition, brief, context, skill, model, tool choice, or Work graph — before the next
   attempt. A short evidence-based debrief records what changed; the same vague retry fails.
6. When the lead genuinely needs guidance outside its charter, the question reaches the **Exec with a
   reason and prepared state**. The Exec answers where it can; if owner judgement is genuinely needed,
   the Exec surfaces the bounded decision and returns the answer through the same chain. Ordinary
   team uncertainty does not jump member → owner or lead → owner. Irreducible human last miles remain
   direct owner handoffs under the existing authority contract.
7. The owner sends **one message to the lead** in the People surface and receives an answer that
   speaks for the team's state — what is in motion, what is blocked, what changed — without the owner
   messaging any member individually and without the Exec relaying.
8. The owner's instruction to the lead **changes the team's work**, observably, in the Work graph.
   A lead that only summarises has not satisfied this criterion.
9. The Exec's load measurably drops: on the run's coordinating turns, the Exec is not the actor that
   assembles the roster, relays ordinary handoffs, or answers member blockers inside a led team. Where
   the Exec still intervenes, the record says why.
10. The People surface renders the real team graph — lead at the top level, members indented — read
   from OrgIntel, with no client-side hierarchy invention. Actors with no team appear honestly as
   unassigned rather than being parented to something plausible.

## Why this is the next slice

Three runs have now produced the same shape of failure, and it is a span-of-control failure, not a
tooling one.

**The singleton Exec is a bottleneck and a single point of failure.** S05-T7's friction record: *Kimi
exhausted its allowance after 54 tool calls and stopped the singleton Exec.* One actor's provider
ceiling halted the company because one actor was doing all coordination.

**Machine-doable coordination is reaching the owner.** S05-T8's friction record includes *escalated
machine work to the owner*. The owner is currently the first escalation target above the Exec, because
there is nothing between a staff member and the Exec, and nothing between the Exec and the owner.

**Staff have no bounded coordinator, so they read company-wide state as their own.** From the Sprint 05
run report: *Staff reused the company-wide Exec termination question, so a critic that had completed
its own review marked itself blocked because later company work remained.* A member with a lead has a
bounded question to ask — "am I done with **my** assignment" — and someone bounded to ask it of.

**Actor identity is not durable, which makes any grouping meaningless.** `orgintel` §5.1 promises
"durable actors, replaceable sessions". The live Aris company contradicts it:

```
copy-critic · copy-critic2 · centre-critic · centre-critic-live · centre-offer-critic
staff-email-writer-v9 · staff-plain-english-critic-v7
```

Actors are minted per Work and per revision. Grouping throwaway actors under a lead organises
throwaways, so T3 lands before T4.

**And the owner cannot see any of it, or say anything to it.** The People surface showed thirteen flat
rows with `READY` beside most of them, and the primary surface for a person was a metrics panel rather
than a conversation. Building T2 then surfaced a harder fact, confirmed by probe: **owner mail reaches
only the Exec.** `restlessd/src/schedule.rs:139` gates its message handler on `to == "exec"`; a message
to `staff-email-writer` on the `aris_feedback2_test` company was recorded as id 19, left `read_at:
null`, and produced **no event at all**. There is no one else to talk to — not as a surface gap, but
as a delivery gap. T5 owns closing it.

### What this sprint is not

It is **not** a management hierarchy for its own sake, and not an org-chart feature. A lead exists for
four concrete jobs: **assemble the team around the outcome**, **run and repair it below the Exec**,
**improve the next Attempt from real review evidence**, and **give the owner one actor that can speak
for the team and change it**. `orgintel` §6.1 keeps the primitive set small on purpose; `Team` earns
its place by those behaviours and is judged on them.

It is also not `orgintel` §6.3's full teamwork-pattern library. General pattern recommendation, health
scoring, and a learning engine stay unbuilt this sprint. A lead choosing a team for one real outcome
and recording one evidence-backed improvement in ordinary files is the behaviour that library might
later learn from; building the recommender first would be modelling before observing (§16.1).

## Tickets

| ✓ | Ticket | Layer | Evidence (observed friction) | Depends |
|---|---|---|---|---|
| [x] | **[S06-T1 · Remove the situation strip and redundant eyebrows](sprint-06/t01-cockpit-chrome.md)** | Owner surface | A 54px strip on every page carrying five truncated values, four of which read "Goal not avai…", "2 Work items…", "$49.25 / $10…"; and mono eyebrows restating the heading directly beneath them | — |
| [x] | **[S06-T2 · People is a conversation surface](sprint-06/t02-people-conversation.md)** | Owner surface | Selecting a person yields a metrics panel; the only way to talk to anyone is the Exec rail, which the People page renders *twice* — once as the permanent rail and once when the Exec is the selected person | S06-T1 |
| [x] | **[S06-T3 · Durable specialist actors](sprint-06/t03-durable-actors.md)** | OrgIntel | `copy-critic2`, `centre-critic-live`, `staff-email-writer-v9`: actors minted per Work and per revision, contradicting `orgintel` §5.1 | — |
| [x] | **[S06-T4 · Teams and accountable leads](sprint-06/t04-teams-and-leads.md)** | OrgIntel | Nothing in OrgIntel relates one actor to another; the Exec is the only coordinator and the owner is the first escalation above it | S06-T3 |
| ~ | **[S06-T5 · The lead runs, repairs, improves, and speaks for the team](sprint-06/t05-lead-coordination.md)** | OrgIntel + Runtime | 54 tool calls exhausted the singleton Exec and stopped the company; machine work escalated to the owner; a completed critic blocked itself on company-wide state | S06-T4 |
| ~ | **[S06-T6 · People renders the real team graph](sprint-06/t06-people-team-graph.md)** | Owner surface | Twelve flat rows, eleven marked READY, no visible structure and no one accountable to talk to | S06-T2, S06-T4 |

`~` means partially evidenced and is not done. The exact observed and still-open behaviours are in
the [run report](sprint-06/run-report.md). T5's wake/routing substrate is implemented, and its live
run repaired incorrect evidence-node termination, graph-gated premature lead acceptance, and the
inflated historical spend projection without rewriting history. Its replacement critic and lead
verdict completed at exact commit `148efbf`; the final debrief and a bounded append-only wording
correction completed through Goal-linked Work with exact artifact digests. T6 is waiting on the authenticated
People-page browser check, not on an invented hierarchy or stale fixture.

The outcome path is **T3 → T4 → T5 → T6**. T1 and T2 are independent owner-surface work and are
already landed; they are in this sprint because T6 builds directly on T2's layout.

## Slice per layer

**OrgIntel.** Durable actor identity (T3); an Exec-commissioned `Team` whose accountable lead assembles
and reshapes its member actors (T4); Work-driven kickoff, escalation to the lead, local repair and
improvement, then reasoned guidance lead → Exec → owner only where needed (T5). The read model gains
team rows and regenerates its TypeScript bindings through the existing S05-T4 seam.

**Owner surface.** Cockpit chrome reduction (T1); People becomes a conversation surface with the
person's evidence beside it (T2); the People index becomes the real team tree and the lead becomes the
owner's addressable point of contact for a team (T6).

**Kernel / Authority.** Untouched. A team lead is coordination, not authority. It grants no effect
permission, no budget, and no approval right. A lead cannot approve what its members could not, and
the owner's approval boundary is unchanged. This is deliberate and is the main thing to hold onto if
the sprint drifts.

**Runtime.** Only what T5 needs for a lead's wake to carry its charter, team state, actor evidence,
Work, blockers, messages, and review artifacts. No new process class, supervisor tier, or container.

## Deliberately out of scope

- The `orgintel` §6.3 pattern library (recommend / explain / observe health / learn).
- Multi-level hierarchy. **One level: lead → members.** A lead does not lead leads. If a second level
  is needed, that is evidence from a run, not a design decision taken here.
- Teams as an authority, budget, or approval boundary (see Kernel slice above).
- Per-team spend ceilings, per-team credentials, team-scoped runtimes.
- Any team structure declared in `company.toml`. The Exec commissions an outcome and appoints its
  lead; the lead assembles and reshapes the one-level roster from actual Work. The structure is
  observed in the record and adjustable by the owner. A configured org chart is the speculative
  version.

## Deletion

Per §16.5, this sprint removes:

- `web/src/lib/components/SituationStrip.svelte` and its ~110 lines of CSS across two breakpoints,
  plus the `--situation-h` token and the `bridge-workspace` two-row grid it forced (T1).
- The `situation` snippet from `AppShell`'s required props (T1).
- The permanent Exec rail on the People route, which duplicated a conversation the page can hold
  directly (T2).
- Whatever per-revision actor-minting machinery T3 replaces, once durable actors carry the same runs.

## Founder decisions and sprint defaults

1. **Who assembles a team?** The Exec commissions the outcome, creates the team charter, and appoints
   the accountable lead. **The lead assembles and reshapes its own members.** This is still one level:
   a lead cannot create a sub-team or delegate its accountability to another lead.
2. **May an actor belong to two teams?** Sprint default: **no** — one team, so "who is accountable"
   has one answer. A lead may claim an unassigned durable actor; a cross-team staffing need goes to
   the Exec rather than silently poaching another team's member. Revisit if a real run needs a shared
   specialist.
3. **What happens to a team when its lead is unavailable** (provider exhausted, session dead)? The
   S05-T7 failure says this must have an answer before the lead becomes load-bearing. Sprint default:
   escalation falls through to the Exec and the fall-through is recorded, not silent.
4. **Is the lead's Work its own, or the team's?** Sprint default: a lead owns Work like any actor, and
   its coordination is not modelled as Work. Revisit if coordination becomes invisible.
5. **Who receives unresolved guidance?** A member asks its lead; the lead asks the Exec with a reason
   and prepared state; the Exec resolves it or surfaces a bounded judgement to the owner. The five
   irreducibly human handoff categories bypass this reasoning chain and still reach the owner
   directly because no actor can perform them.
