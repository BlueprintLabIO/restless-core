# S06-T5 · The lead runs, repairs, improves, and speaks for the team

**Layer:** OrgIntel (routing, context assembly, wakes) + Runtime (what a lead's turn is given).
**Serves:** `orgintel` §2.2, §6.2 (communication), §7.1 (self-running operation), §3.3 (self-repair).
**Depends on:** S06-T4.
**Makes deletable:** the assumption that the Exec is the only coordinator — concretely, the
Exec-shaped escalation default in staff context assembly.

**Implementation evidence:** tracked in the Sprint 06 checklist and
[`run-report.md`](run-report.md), not in a second ticket-status field.

---

## This is the ticket that justifies T4

T4 adds a table. This one does the four jobs the record says are unowned. If T5 cannot be demonstrated,
T4 should be deleted rather than kept.

## The four jobs

### 1. Run the team and absorb repair below the Exec

Today a blocked member's only upward path is the Exec, and a saturated Exec's only upward path is the
owner. Both hops are in the record as failures (S05-T7, S05-T8). With a lead:

- **A member's blocker reaches its lead first.** The lead resolves it inside its own team where it
  can — reassign or replace a member, revise a brief, change a dependency, open a new Work node,
  select a more suitable context, skill, model, or tool, or decide between contradictory outputs —
  and asks the Exec only when the fix is outside the team's charter or crosses team boundaries.
- **The Work graph runs ordinary execution.** Ready Work wakes its responsible member and completion
  releases dependants. The lead designs and repairs the graph; it does not manually narrate every
  kickoff and handoff. Messages remain free-form context, not a second assignment protocol.
- **A member's "am I finished" question is bounded by the team**, not by company-wide state. This is
  the direct fix for the run-report failure where a critic that had completed its own review marked
  itself blocked because *other* company work remained.
- **Fall-through is recorded, not silent.** If the lead is unavailable — provider exhausted, session
  dead, disbanded team — escalation falls through to the Exec and the record says it fell through.
  A lead that silently swallows escalations is worse than no lead, and the S05-T7 failure is exactly
  the shape this must not repeat one level down.

### 2. Ask the Exec for guidance, with a prepared path to the owner

- **The lead asks upward only when local repair is exhausted or the decision is outside its charter.**
  The request carries the exact blocker, evidence, attempted repairs, affected Work, and the smallest
  decision needed. "What should I do?" with no prepared state is not an escalation.
- **The guidance conversation remains free-form.** The reason and prepared state are context assembled
  from Work, messages, and artifacts, not a new form, command algebra, or three-state conversation
  protocol. If an answer is required before Work can resume, the question is linked to that Work so
  the existing graph has one deterministic condition to observe.
- **The Exec is the next judgement altitude.** It answers company-wide priority, cross-team resource,
  strategy, or charter questions. If the Exec can resolve the need, the owner never sees it.
- **The Exec brings irreducible judgement to the owner.** When owner taste, mandate, capital exposure,
  or another real owner decision is required, the Exec surfaces the bounded choice and preserves the
  prepared state. The answer returns Exec → lead → affected Work.
- **Irreducible human last miles remain separate.** Identity, CAPTCHA, MFA, legal attestation, and
  payment confirmation still reach the owner directly because no amount of organisational guidance
  can make an agent perform them. This ticket does not turn those into management questions.

### 3. Speak for the team to the owner, and change it

- **Speak for it.** A lead's context assembly carries its team's state — members, their Work, what is
  in motion, what is blocked, what changed since the owner last asked. An owner message to the lead is
  answered about the team, not about the lead's own single Work item.
- **Change it.** An owner instruction to a lead adjusts its team's Work observably in the graph. This
  is the criterion that separates a lead from a status endpoint: a lead that only summarises has not
  satisfied it (sprint success contract §8).

### 4. Improve from evidence, not self-description

- **A failed check or criticism changes the mechanism.** The lead diagnoses whether the miss came
  from composition, brief, context, skill, model, tool, dependency, or review target, changes the
  smallest relevant part, and starts a comparable next Attempt. Asking the same producer for another
  vague pass is not repair.
- **The real outcome is the review medium.** A website is reviewed as a running desktop/mobile page,
  a paper as a rendered PDF, and a game as a playable build. Code and process logs are supporting
  evidence.
- **Learning begins as an ordinary file or actor-evidence note.** The lead records what evidence
  caused the change and applies it to the next comparable Attempt. There is no actor-profile entity,
  learning engine, automatic prompt mutation, or permanent doctrine promotion in this ticket.

## Scope

1. **Work-driven operation.** Ready Work wakes its responsible actor and deterministic dependencies
   release the next node. The lead is woken for judgement-bearing team events — blockers, failed
   review, contradictory outputs, roster needs, addressed messages, and owner feedback — rather than
   polling or relaying every transition.
2. **Escalation routing:** member blocker → lead → Exec → owner where genuinely needed, with the actor,
   reason, prepared state, and resolution recorded at each hop. The return path reaches the affected
   Work rather than ending as an isolated answer.
3. **Lead context assembly:** the outcome charter and shared spine plus its team's durable actors and
   relevant evidence, Work graph, blockers, messages, recent changes, and real review artifacts. Per
   `orgintel` §5.2, this is *more* context than a member and *less* than the Exec.
4. **The lead's bounded coordination acts**, inside its own team only: reassign or replace members,
   request a revision, adjust a brief or dependency, open a new Work node, mark an assignment
   complete, and choose a different available context, skill, model, or tool for a new Attempt. These
   acts do not grant new effect authority, credentials, or budget.
5. **Owner ↔ lead conversation must first be made to exist.** S06-T2 found and confirmed by probe that
   owner mail reaches only the Exec: `restlessd/src/schedule.rs:139` gates its message handler on
   `to == "exec"`, so a message to any other actor is recorded, fires its NOTIFY, matches nothing, and
   is never read. Staff receive owner input only as Work-linked feedback (`staff.rs:173`).

   Sprint success criteria 7 and 8 are therefore **not reachable on the current delivery path**, and
   this ticket owns closing it: a message addressed to a team lead must wake that lead. Note the
   asymmetry to design around — the Exec has `fire_exec`, while staff run only through
   `dispatch_claimed_work`, so "wake a lead for a conversation turn" is a genuinely new path and not a
   one-line widening of the match arm. That is why delivery belongs to this OrgIntel/Runtime ticket,
   not only to T2's conversation surface.
6. **Evidence-backed local improvement.** After a meaningful failure or review, the lead records the
   smallest causal lesson in an ordinary team brief, actor-evidence note, skill/process file, or run
   report and applies it to the next comparable Attempt. Promotion into company-wide doctrine
   remains outside this ticket.
7. **Termination/status questions** are scoped to the assignment and the team, not the company.

**Not in scope:** the lead approving effects, spending, or holding credentials (S06-T4's invariant);
leads coordinating other leads; automatic team-formation heuristics; a capability ontology; an
automatic learning or prompt-rewriting engine; the §6.3 pattern library.

## Verification

This ticket is behavioural, so it is verified by a run, not by a unit test
(`ARCHITECTURE.md` §16.8: OrgIntel gets behavioural and recovery scenarios).

`_test` company, scripted:

1. Give member A ready Work. Assert A starts from graph readiness without the Exec or lead manually
   sending a second kickoff message.
2. Member A blocks. Assert the blocker reaches the lead and **not** the Exec, and that the lead's
   local resolution appears in the record with member A resuming.
3. Fail the real review target or return a specific critic rejection. Assert the lead changes one
   relevant mechanism — not merely the wording of "try again" — records the evidence, and a new
   comparable Attempt uses the change.
4. Give the lead a real cross-team or charter question. Assert it reaches the Exec with a reason,
   evidence, attempted repairs, affected Work, and bounded decision. Assert the Exec's answer resumes
   the team without owner attention.
5. Give the Exec a question that genuinely needs owner judgement. Assert the owner sees one prepared
   decision from the Exec, not raw member context; the answer returns through the lead to the affected
   Work. Separately assert the five irreducible human categories still bypass the guidance chain and
   reach the owner directly.
6. Kill the lead's session mid-escalation. Assert fall-through to the Exec **and** an explicit
   fall-through record. A silent swallow fails the ticket.
7. Owner sends one message to the lead. Assert the answer references at least two members' current
   state, with the owner having messaged nobody else.
8. Owner instructs a change. Assert the Work graph differs afterwards in the way instructed.
9. Member A asks whether it is finished while unrelated company work remains open. Assert A reaches
   `completed`.

Against live Aris, the sprint's success contract §9 additionally asks: on the run's coordinating
turns, the Exec is not the actor assembling the roster, relaying ready Work, or answering member
blockers inside a led team.

## Risk disposition

- **The lead becomes a relay that adds a hop and no judgement** — *pending fix*: sprint criteria 4,
  5, 7, and 8 require local repair, improvement, a team-level answer, and changed Work. A relay fails.
- **A saturated lead becomes the new single point of failure** — *guarded* by the recorded
  fall-through in verification criterion 6. This is S05-T7 repeating one level down and is the failure mode to
  watch hardest.
- **The lead inflates the roster instead of repairing the work** — *guarded*: every added member needs
  a stated difference and bounded output; the run compares accepted outcome, owner intervention,
  elapsed time, and cost.
- **"Self-improvement" becomes prompt drift** — *guarded*: a change names the failed evidence, changes
  the smallest mechanism, lives in a reviewable file or evidence note, and must improve a comparable
  Attempt before it is treated as useful.
- **Two coordinators disagree** (lead adjusts what the Exec just set) — *accepted* and made visible:
  both acts are in the record with their actor. `orgintel` §6.3 says warn rather than freeze work.
- **Lead context assembly grows toward Exec-sized** — *pending fix*: if a lead needs the whole company
  to answer for its team, the team boundary is wrong, and that is evidence about the boundary rather
  than a reason to widen context.


---

## Contract change after founder alignment — 17 August 2026

The landed routing proved that an ordinary member judgement can be owned below the owner, but its
second hop is now too short. The current implementation allows a lead to escalate an
`owner_judgement` directly to the owner. The agreed target is:

```text
member → lead → Exec → owner, only if the Exec cannot resolve it
```

Member → lead remains correct. Lead → owner for ordinary organisational guidance does not. The next
implementation slice must make the Exec an addressable judgement destination, preserve the reason and
prepared state, and return the answer to the affected team Work. Until that exists, the guidance
chain in this ticket is not landed.

The five irreducibly human categories are unchanged. They still reach the owner directly because
identity, CAPTCHA, MFA, legal attestation, and payment confirmation are last-mile participation, not
questions the Exec could resolve.

## Historical intermediate slice: member judgement gained a second destination

**This subsection records the intermediate implementation that motivated the aligned chain. Its
direct lead → owner mismatches are now superseded by the implementation evidence below.**

The audit behind this is in [`audit-what-reaches-the-owner.md`](audit-what-reaches-the-owner.md).
Its conclusion set the scope precisely: `owner_judgement` is the only category a lead may absorb, and
in practice it is 100% of what OrgIntel sends the owner.

`owner_handoffs` gained `assigned_to`, `escalated_from`, `escalated_at`. `NULL` on `assigned_to` means
the owner — which is what every pre-existing row means, and why it is nullable rather than defaulted.

- **`request_owner_handoff` routes.** An `owner_judgement` raised by a team member is assigned to that
  member's lead. `team_lead_for` returns `None` when the actor has no team *or is the lead*, so a lead
  never escalates to itself. **Current mismatch:** a lead's own judgement still reaches the owner;
  under the aligned contract it must reach the Exec first.
- **The owner queue narrowed.** `attention::project` filters to `assigned_to.is_none()`. This is a
  filter on *whose queue*, not on what the owner may see.
- **The blocked Work names its resolver** — `awaiting offer-lead judgement, handoff <id>` rather than
  `awaiting owner handoff <id>`. A queue invisible to the person who could clear it is not delegated.
- **Escalation is explicit and reasoned.** `restless work escalate-handoff --as <lead> --reason <why>`.
  An empty reason is refused: the owner is being asked for time, and an unexplained handoff is exactly
  the cost this removes. Only the assignee may escalate. **Current mismatch:** the command escalates
  to the owner; ordinary lead guidance must first reassign to the Exec.
- **The owner sees the current chain.** An escalated item's `why_it_matters` reads *"Offer lead could not
  settle this and passed it up: needs the owner's pricing call"* instead of the generic judgement copy.
  The target chain must also say what the Exec tried and why the remaining decision is truly the
  owner's.
- **`restless judgement --as <actor>`** is the lead's queue — the same shape as the owner's
  `attention`, because it is the same job at a different altitude.

### The invariant, enforced and tested

The five irreducibly human categories — identity, CAPTCHA, MFA, legal attestation, payment
confirmation — **never** route to a lead, at any org size. A lead absorbing a payment confirmation
would be a lead exercising authority it does not have. `escalation.rs` asserts this for all five by
enumerating the enum, so adding a sixth category without deciding its routing fails the test.

## Current implementation

- Ready Work is claimed atomically with an Attempt and starts its durable owner. Initial dependencies
  declared by `work add --requires/--revises` commit with the node, so the scheduler never sees a
  half-built initial graph.
- Addressed owner/member messages and assigned judgements wake a non-Exec team lead through the same
  ACP, provider failover, spend, and process supervision used by Work. A successful coordination
  turn marks captured mail read; a crash leaves it owed.
- Lead context carries the team charter, roster, team-owned Work, addressed messages, and pending
  judgements. It now carries Goal identifiers and the team's exact `requires`/`revises` edges, so a
  lead cannot mistake a prose claim for graph completion. Members get bounded assignment context
  rather than the company's Exec termination question.
- Ordinary routing is member → lead → Exec → owner only when still unresolved. The five irreducible
  human last-mile categories remain direct owner handoffs. A resolution is written back as exact
  Work feedback before the affected Work resumes.
- Critic text is preserved as Work feedback. Failed and rejected Work blocks instead of blindly
  retrying; `work resume` requires the accountable actor to name the changed mechanism.
- A lead may repair dependencies only inside its own team, with attribution and a reason. Provider or
  session failure reassigns owed judgements to the Exec and records the fall-through.
- The scheduler excludes actors already supervised in a conversation before claiming Work, so a
  busy lead consumes no false Attempt. A configured-Postgres regression locks this in.
- Staff usage accounting treats ACP updates as cumulative session snapshots and writes one final
  spend record per provider session. Re-prompts no longer charge every prefix again; provider
  failover still records each provider session separately.
- `changes_requested` invalidates a producer only from Work with an explicit outgoing `revises`
  edge. Evidence/research Work without review power completes after the unchanged artifact and gate
  checks, leaving its findings for the downstream critic.
- Superseded coordination can be retired through attributed `work abandon` without deleting Work,
  Attempts, artifacts, or history; a running Attempt is always refused. The per-company process
  ceiling is a resource guardrail and is now 100 rather than an organisational design assumption of
  two concurrent people.

## Observed Aris evidence and remaining proof

The live run in [`run-report.md`](run-report.md) proved Exec commission, lead-owned roster assembly,
graph-driven kickoff, a real member report reaching the lead, owner → lead delivery, direct lead →
owner reply, owner-directed graph repair, explicit resume after mechanism change, provider failover,
and reduced ordinary Exec relay load. It also exposed and fixed atomic initial graph construction,
supervisor restart recovery, busy-actor attempt consumption, and cumulative Staff usage accounting.

The run also disproved the first completion claim: an evidence-only researcher returning
`changes_requested` stayed blocked without a revises edge; the lead then declared acceptance before
that dependency and the independent critic completed, while leaving a documented localized
overclaim in place. The inflated pre-fix spend history closed the original `$100` fuse before the
Exec could repair the chain. These were live T5 failures, not green implementation details. The
accounting was subsequently corrected append-only, the owner raised the ceiling to `$200`, and a
Goal-linked replacement graph completed exact-commit gates, rendered-site evidence, and
primary-source claim verification at `148efbf`.

Still open as evidence, not hidden as implementation green:

1. an authenticated People-page click/send and visual tree check (the same backend path is proven by
   Aris messages 92–93, but no owner browser was controllable during the run);
2. one genuine outside-charter question traversing lead → Exec → owner and back in a live company
   (message 98 reached Exec, but budget prevented the answer; the configured-Postgres scenario covers
   the route and return semantics); and
3. atomic **batch** replacement of several live graph edges. Initial graph creation is atomic, but
   the Aris repair briefly exposed downstream Work between separate remove/add transactions.

The exact-commit N4 → N5 → debrief chain is now complete. Debrief Work `1fbe0d1f` completed with a
linked digest, and bounded correction Work `e113cfdc` then fixed one append-only-ledger sentence and
linked the final digest `770bd0c2…f9b4`. The correction also reproduced a narrower scheduling fact:
an owner message wakes the lead as a conversation before ready Work can claim that actor. The busy
actor exclusion prevented a false Attempt, then Work dispatched after the conversation released the
lead. That preserves correctness but adds latency and should be made one deterministic per-actor
queue rather than two competing wake paths.

## Verification

`RESTLESS_TEST_DATABASE_URL=postgresql:///restless cargo test --workspace -- --nocapture` exercises
the actor/team, escalation, feedback/revision, recovery, atomic-creation, and busy-actor scheduling
scenarios against PostgreSQL. The test was separately confirmed to fail when member → lead routing
was disabled, so its green is evidence rather than coincidence. The real-company observations and
their exact Work, message, event, and artifact identifiers are recorded in the run report.
