# S06-T6 · People renders the real team graph

**Layer:** Owner surface, reading OrgIntel. No new owned concept — teams are S06-T4's.
**Serves:** `owner-cockpit` §5; `CLAUDE.md` — *probe, never guess*, applied to structure as well as
capability.
**Depends on:** S06-T2 (the three-column layout), S06-T4 (the relation to render).
**Makes deletable:** nothing.

**Implementation evidence:** tracked in the Sprint 06 checklist and
[`run-report.md`](run-report.md), not in a second ticket-status field.

---

## The friction

The People index is thirteen flat rows, eleven of them marked `READY`, sorted by creation. Nothing on
the surface says who is accountable for what, who works with whom, or who the owner should talk to.
The result is that the owner talks to the Exec about everyone — the load T5 exists to reduce.

## Scope

1. **The index becomes a two-level tree**: teams, each headed by its lead, with members indented one
   level beneath. One level only, matching S06-T4.
2. **The lead row is the team's addressable point of contact.** Selecting it opens the conversation
   with the lead — which, per S06-T5, answers for the team. This is the default selection when a team
   is expanded, because it is the conversation the owner most often wants.
3. **The team header carries the team's state**, not a decoration: member count, how many are in
   motion, how many blocked. Enough to choose which lead to talk to.
4. **Unassigned actors are shown as unassigned**, in their own group at the bottom. The Exec and
   non-person actors (`world`, `daemon`) are separate from teams and are never parented into one to
   make the tree look complete.
5. **No client-side hierarchy invention.** The tree renders `TeamRow` + membership from the cockpit
   projection. If OrgIntel has no teams, the list is flat and honest — it does not fall back to
   grouping by role string or guessing from the Work graph. An empty structure is a true statement
   about the company.

**Not in scope:** drag-to-reassign, org-chart layouts, collapsing/expanding persistence across
sessions, team creation or routine roster assembly from the SPA. The Exec commissions the outcome
and appoints its lead; the lead assembles and reshapes the roster through ordinary OrgIntel/Runtime
work (S06-T4). The owner's overrides land on the CLI first, which stays the complete administrative
surface (S05-T2).

## Shape

```
▾ CENTRE OFFER                          4 · 1 in motion
    centre-offer-lead        lead            working
    centre-offer-impl                        ready
    centre-offer-critic                      ready
    centre-pdf-content                       blocked
▾ PROSPECTING                            2 · 0 in motion
    prospect-lead            lead            ready
    prospect-researcher                      ready
  UNASSIGNED                             1
    claude-oauth-probe                       ready
  NOT PEOPLE
    The Exec                                 working
    The outside world                        ready
```

## Verification

Headless: against a `_test` company with two teams, assert the rendered tree matches
`GET /api/companies/{c}/cockpit`'s team rows exactly — same leads, same membership, same unassigned
set. Against a company with **zero** teams, assert the list renders flat with every actor unassigned
and no invented grouping.

The second check is the one that matters. A tree that looks right on data that has structure proves
little; the failure this guards against is a surface that produces plausible structure where the
company has none — the same defect class as `owner-cockpit` §2.7's *evidence before self-report*.

## Landed evidence and remaining proof

The cockpit projection now reads actors, teams, goals, and Work in one database snapshot. It carries
real `TeamRow` values and actor `team_id`s; if OrgIntel is unavailable it says so rather than
inventing partial structure. The People page renders team → lead → member, plus explicit Unassigned
and Not people groups. Selecting a member routes the conversation to that member's accountable lead;
the composer is enabled only for the Exec and live team leads. Rust bindings were regenerated rather
than hand-edited.

`cargo test -p restlessd`, the generated-binding check, `npm run check`, and `npm run build` pass.
The authenticated browser render/send check remains open because no controllable owner browser was
available during the run. Backend owner → lead delivery and the direct reply were observed as Aris
messages 92–93, but that is not represented as a click-through UI proof.
