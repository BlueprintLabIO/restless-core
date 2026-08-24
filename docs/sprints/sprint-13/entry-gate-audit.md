# Archived S13 product-affordance entry-gate audit — 24 August 2026

**Disposition:** the original TypeScript **coordination-affordance** build is not authorised.

**Scope note:** A separate test-only TypeScript ecosystem comparison was briefly drafted on 24 August,
then cancelled before implementation and superseded by Sprint 14’s Rust-consolidation decision. That
does not amend this product result.

## Question

Did the repository contain one EXP-02 coordination affordance that had earned the bounded TypeScript product pilot originally described by Sprint 13?

## Candidate evidence

[EXP-02's production recommendation](../../../experiment/coordination/experiments/EXP-02/recommendation.md) answers no: it recommends no new durable coordination mechanism. Its [exit audit](../../../experiment/coordination/experiments/EXP-02/exit-audit.md) records these final dispositions:

| Candidate | Evidence disposition | Product-pilot consequence |
| --- | --- | --- |
| A1 commission context | Rejected: a P3 component result did not replicate native quality in P1 | Cannot be the pilot |
| A2 shared-history fork | Not activated: exact artifact access left no residual causal-context loss | No evidence-backed hook |
| A3 material-event capsule | Not activated: no repeated reconstruction bottleneck was observed | No evidence-backed hook |
| A4 recovery capsule | Loss: provenance improved, targeted churn did not | Cannot be the pilot |
| A5 micro-audition | Not activated: no genuine actor/seam uncertainty occurred | No evidence-backed hook |

This fails the original product branch’s first two entry gates. It does not license a generic TypeScript runner, context envelope or plugin mechanism.

## Historical verification baseline attempted

The local scratch Postgres service accepted connections to the local `restless` database. This command was run:

~~~text
RESTLESS_TEST_DATABASE_URL='postgresql:///restless' cargo test --workspace --no-fail-fast -- --nocapture
~~~

All live OrgIntel behavioural scenarios passed, including schema round-trip, atomic claim/gates, recovery, messaging and handoff cases. The overall workspace did not pass:

- `restlessd`: 112 passed, 1 failed;
- failing test: `staff::tests::one_actor_keeps_one_organisational_posture_across_wake_types`;
- failure: its assertion expects the operating rules to contain “There is no required handoff template”, but the current context source no longer contains that exact wording.

Strict Clippy was also run:

~~~text
cargo clippy --workspace --all-targets -- -D warnings
~~~

It reports two mechanical findings in active dirty work:

- needless borrow in `crates/restlessd/src/staff.rs` line 585;
- unnecessary_map_or in `crates/restlessd/src/main.rs` line 2574.

These checks identify the old product-pilot baseline honestly. They were later superseded by Sprint
12’s recorded 145-test passing workspace run; this audit preserves the original decision-time evidence
rather than silently rewriting it.

## Result and retained gate

No TypeScript coordination process, context envelope, schema, configuration, actor capability or generic extension point was added.

The original product branch remains closed until founders have all of the following:

1. a new experiment result naming exactly one affordance and its measured bottleneck;
2. a qualifying native-outcome/churn win against its simpler control, including required replication;
3. a named low-consequence real-company internal outcome and frozen no-module control; and
4. a clean live-Postgres, formatting and strict-Clippy baseline for the branch that will own the hook.

This gate continues to block any claim that TypeScript itself helps coordination. Sprint 13 is now
archived; no separate OIDC spike remains active.
