# Sprint 48 — Judgement-aware collaboration

**Status:** Draft for founder alignment
**Programme:** [Core company collaboration](company-collaboration-programme.md)
**Paired Cloud sprint:** Cloud 18
**Depends on:** Sprints 45–47

## Outcome

A human asks `@exec` or a responsible specialist in a Thread, Core creates one focused recipient-
relative Attention request, the correct durable Actor answers or hands off in the same Thread, and the
accepted contribution resumes the exact dependent Work. Conflicting human direction reaches the
accountable scope owner instead of becoming last-message-wins.

## Scope

- Add explicit Actor Mentions, atomic Mention-to-Attention linkage and a durable wake request.
- Make Attention recipient-relative: exact question, why this Actor, recommendation, alternatives,
  evidence/uncertainty, affected Work, deadline/fallback, independent work and return path.
- Route first by named responsibility, Work owner/reviewer, project/department lead, Exec, then
  Authority owner only for mandate/capital/root authority.
- Treat `input`, `work_with` and `transfer` as semantic contribution routes, not global UI modes or a
  Chat/Attention toggle.
- Add Thread active-responder and handoff state for consult/work-with/transfer while preserving one
  accountable owner and explicit return condition.
- Promote a selected Message explicitly to an existing Directive, Decision, Work/commitment, review,
  Attention, Authority request or Doc, with attribution and backlink.
- Surface contradictory instructions, preserve both authors/positions and route one Decision request
  by organisational scope.
- Keep agent chat output to material progress, blockers, changed assumptions, requested judgement and
  accepted outcomes.

## Acceptance

1. `@exec` while Runtime sleeps persists Message, Mention and Attention atomically, wakes one Exec
   Actor later and returns its response to the same Thread.
2. Exec hands a task-level discussion to a Technical Lead without the human restating context; Exec
   later receives only the decision, evidence, risk and company implication.
3. A bounded input request times out according to its stated fallback while unrelated Work continues.
4. A sustained transfer changes the existing Work owner explicitly and produces a return handoff; it
   is not recorded as a generic notification.
5. A Message promotion creates exactly one target object and backlink under retry.
6. Conflicting human directions create a visible conflict/Decision route and never silently prefer the
   newest message or membership rank.
7. Repeated accepted judgement can be proposed as a Decision, Principle, example or playbook change,
   but no automatic policy promotion occurs.

## Ticket outline

- [ ] C48-T0 — recipient-relative Attention contract
- [ ] C48-T1 — Mentions, wakeup and `@exec`
- [ ] C48-T2 — contribution routes and active responder
- [ ] C48-T3 — promotion/backlinks and decision conflict
- [ ] C48-T4 — focused owner/mobile presentation
- [ ] C48-T5 — dormant Runtime and conflicting-human dogfood

## Deletion and exclusions

Delete owner-default routing, Chat/Attention mode toggles, last-message-wins assumptions and generic
Attention detached from affected Work. Do not add majority voting, consensus, opaque expertise ranking,
company-wide locks, automatic transcript-to-policy conversion or a new workflow engine.
