# T15 · Run report, deletion pass, friction backlog

**Layer:** Cross-cutting.
**Serves:** This is arguably the sprint's most valuable deliverable. It is what makes sprint 02 evidence-driven rather than imagined (§16.4, §16.5).
**Makes deletable:** **Everything the runs did not exercise.** This ticket is where the sprint's deletion pass happens.
**Depends on:** T11, T12, T13, T14.

## Build

### Recorded per company

Elapsed time, dollar cost, owner-intervention count.

### The questions this must answer

- Did the small §4.4 ontology survive three company shapes, or did each want its own vocabulary?
- Did event-driven wakeups actually fire, or did the Exec go silent after a dependent result landed?
- Where did owner attention get pulled in — and was it **genuine judgement, or missing machinery?**
- Did file + Git work survive agent crashes intact, without custody machinery?
- **What did companies 2 and 3 cost to add, relative to company 1?** This is the sprint's primary measurement.
- Which company is the strongest ongoing dogfood? (Answers ARCHITECTURE.md §14 open question 12 empirically rather than by argument.)
- How often did agents fail to report coordination state through the CLI, leaving OrgIntel blind? (T10 accepts this will happen; the rate is what tells us whether prompt, playbook or tooling is the right response in sprint 02.)
- Did the Exec terminate on its own judgement, or did runs end at the spend ceiling? (T4)

### Assess the negative claim explicitly

The clean-slate decision rests on a claim this sprint is the first real test of: **that a company runs without the legacy machinery** — no universal command enum, no append-everything ledger, no content-addressed custody, no bespoke workflow engine.

So answer directly: did we have to reinvent any of them? If artifact handoff quietly grew a custody protocol, or retries grew a workflow state machine, or coordination grew a universal command envelope, **say so plainly.** The rebuild premise is better falsified in six weeks than in six months, and a run report that only records what worked cannot do that.

### The deletion pass

**Which code paths did no company exercise?** That is a data lookup, not a matter of taste (LLM_CURE.md frame 3). Anything unexercised comes out before sprint 02 opens. Deletion is reversible — Git holds it — so the bar is low.

### Friction backlog

Concrete observed failures, in the §16.4 form. Platform work in sprint 02 should originate here.

## Acceptance

Sprint 02 can be specified from this document without re-running sprint 01.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
