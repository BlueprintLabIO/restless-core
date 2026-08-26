# Experiment Sprint 06 — Codex versus Restless on a public site outcome

**Status:** Founder-approved; preflight in progress. No counted arm has started.

**Decision owner:** Founder.

**Date:** 26 August 2026

**Outcome:** Determine whether Restless's current accountable organisation produces a better complete
public website outcome than one strong Codex actor when both receive the exact same owner request,
model, reasoning effort, starting code and ordinary tool access.

## 1. Hypothesis and comparison

This is a system comparison, not a model bake-off.

```text
Arm C: owner request -> one Codex producer -> review-ready site

Arm R: owner request -> available Exec -> non-producing accountable lead
                                      -> Staff producer(s) -> lead judgement -> review-ready site
```

Arm R may choose its Staff count using current Restless judgement. Exec and the lead may not silently
produce or repair the site. Every productive contribution must be attributable through Work and an
Attempt. Arm C may plan, implement, inspect and repair its own outcome as one actor.

The primary hypothesis is that Restless wins product truth, breadth of judgement and final quality by
preserving portfolio direction and supervision. The credible null is that one strong, fast actor wins
because brief translation, commissioning and review add latency and information loss without useful
independence.

## 2. Frozen treatment

- Owner prompt: exact bytes of
  [`site-brief.md`](../coordination/experiments/EXP-06/site-brief.md).
- Starting ref: `06c114fc2ef2244777df78c8a754386f50faeeef`.
- Model: `gpt-5.6-sol` through authenticated ChatGPT subscription access, with no fallback.
- Reasoning effort: `medium` for every counted actor.
- Workspace: one fresh isolated Git worktree per arm; neither arm may read the other.
- Allowed scope: `site/` only, plus `site/evidence/` inside each arm.
- Tools: local files, Git, shell, package manager, network fetches and local Chromium/Playwright. No
  native search tool, image generation, external publishing, deployment or live company effect.
- Repo context: normal project `AGENTS.md` and files present at the frozen ref. No conversational
  history, private critique or prior site feedback is supplied beyond the frozen owner prompt.
- Completion: one committed production build candidate, evidence note and terminal actor report.
- Arm order: fixed by the frozen contract before either counted model call.

The Codex arm runs through the authenticated first-party Codex harness. The Restless arm must run
through the current Restless product path: owner request, OrgIntel responsibility, accountable lead,
Staff Work/Attempt, scoped actor sessions, Runtime files, terminal truth and native review. Calling a
model beside Restless and copying its files into a company does not count.

## 3. Validity gates

No arm starts until all gates pass:

1. **Route parity:** a non-productive probe proves both harnesses report `gpt-5.6-sol` at medium
   effort. Restless has exactly one active `openai-codex` broker credential and no model fallback.
2. **Tool parity:** each environment can read the frozen repo, modify only its isolated site scope,
   run the package checks and launch local Chromium. Any material difference is recorded before the
   run.
3. **Restless truth:** the `_test` company is running; Exec dispatch, accountable lead responsibility,
   claimed Staff Attempt, messages, artifact references and terminal review are observable through
   product APIs.
4. **Isolation:** worktrees start clean at the exact ref; repository search from either arm cannot
   resolve the peer path; generated state is excluded from the durable evidence commit.
5. **Evaluation freeze:** prompt hash, rubric hash, route list, arm order, measurement clock and stop
   rules are recorded in `frozen-contract.json`.

A failed gate is a harness or provider result. It is not evidence that either organisational shape
lost.

## 4. Run protocol

Run arms sequentially in the frozen order to avoid CPU, browser and provider contention. The operator
may repair infrastructure only before productive cognition. Once an arm has read the owner prompt,
its site output is counted and the treatment may not be tuned.

Each clock begins immediately before delivery of the frozen owner request and ends at the first
review-ready terminal report whose referenced commit passes the gate checks. There is no semantic
timeout. A 60-minute runaway safety envelope may terminate a live but non-progressing process; that
is reported as operator termination, not ordinary task failure. Provider callbacks, process exit,
Work/Attempt state and artifact existence determine completion. Polling is observation only.

After both arms finish:

1. rebuild each exact commit from a clean install state;
2. run the same objective checks and collect route, bundle and dependency facts;
3. capture equivalent 1440x1000 and 390x844 screenshots for every public route;
4. serve the two candidates behind neutral A/B labels;
5. lock blind rubric judgement before revealing process evidence; and
6. ask the founder to inspect both native review targets and make the final decision.

No losing arm is merged or deployed automatically. The comparison produces evidence and a selected
candidate; incorporation into `dev` is a separate founder decision.

## 5. Measures and decision

The primary outcome is the blind weighted rubric in
[`blind-rubric.md`](../coordination/experiments/EXP-06/blind-rubric.md). Product falsehood,
fabricated evidence, broken core navigation or a failed accessibility/build gate makes a candidate
unpublishable.

Secondary measures explain why an arm won or lost: elapsed closure time, number of model turns,
observable usage, productive actor count, supervision interventions, repairs, provider/runtime
failure, changed surface, dependencies and bundle size. Same-model review is not called independent;
the founder's blind native inspection is authoritative.

One run can choose the better implementation for this exact task. It cannot establish a universal
team-versus-solo law. The result updates the prior and names the next replication or crossover test.

## 6. Deliverables

The sprint closes with:

1. frozen prompt, contract and rubric;
2. route/tool/isolation preflight evidence;
3. one exact commit and runnable production build per valid arm;
4. neutral screenshots and live A/B review targets;
5. objective results, locked blind scores and owner decision;
6. causal process comparison with failures attributed to provider, harness, model or organisation;
7. an update to the coordination evidence/canon stating only what this run supports; and
8. deletion or quarantine of experiment-only adapters and credentials that do not earn product use.

## 7. Stop boundaries

Stop for founder direction before changing the Exec-always-delegates or lead-never-produces
invariants, changing the model or effort, exposing provider credentials to a company Runtime,
publishing either candidate, contacting a competitor, adding a new durable orchestration primitive,
or merging a winner into `dev`. A blocked first-party route may be repaired through the existing
host broker; it may not be disguised as an equivalent run through another provider.
