# Coordination lab v01–v20 — final evidence and recommendation

Completed: 22 August 2026
Scope: scratch evidence programme; no production architecture was changed

## Decision

Keep ACP. Build Restless's first-party harness by composing Pi behind ACP. Keep MCP as a thin optional
adapter for structured services. Put company coordination in OrgIntel, expressed through the seven
generic commands and ordinary Work/Attempt state.

Do **not** fork Pi, replace ACP, remove MCP, or port the v2 comparison coordinator into production yet.
The experiments found no protocol expressiveness failure that prevented control or coordination. The
dominant failures were missing context, conflated identity/provider selection, and model-authored Work
churn that prompt rules alone did not prevent.

```text
Alcanta / OrgIntel
  Actor · Goal · Work · Attempt · Message · Decision · Schedule · Event
  send · commission · redirect · report · request_judgement · decide · schedule
                         │
                        ACP
                         │
Restless first-party harness
  exact launch · context · model pool · tools · streaming · cancellation · result
                         │
                       Pi SDK
                         │
       model providers · native tools · MCP adapters · Company Runtime
```

## What the twenty mini-sprints proved

### First-party harness

The minimal ACP/Pi harness successfully demonstrated:

- exact system prompt, actor, model, cwd, tool set, MCP servers, write scope and limits in a hashed
  launch contract;
- live thought, text and tool lifecycle streaming in chronological order;
- native read/list/search/write/edit/run tools with workspace scoping;
- stdio MCP discovery and calls without teaching the harness organisational semantics;
- cancellation including child process groups;
- explicit `completed`, `cancelled`, `error`, `max_tokens` and `max_turns` outcomes;
- aggregate usage and zero-price proof from the live OpenRouter catalogue before every inference;
- bounded durable telemetry separated from token-live ACP updates; and
- one Work surviving repairs across Cohere, NVIDIA and Google launch attempts without hidden fallback.

ACP v1's fixed stop reasons remain awkward: Restless carries richer result metadata alongside the ACP
response. That is an adapter wart, not today's bottleneck. Reconsider ACP v2 when its session/result
contract removes enough local translation to justify migration.

### OrgIntel coordination

The seven commands were sufficient to express all observed coordination actions. No eighth command was
missing. What was missing was integrity around how the existing commands compose:

- a missing callback must remain `unknown`;
- a new Attempt must receive original Work, previous outcomes, failure evidence and persistent diff;
- messages are not assignments or implicit Work resumption;
- Actor identity is independent of model/provider identity;
- exact duplicate outcomes must not silently create parallel responsibilities;
- review must depend on a produced artifact; and
- only one accountable integrator advances a candidate.

These rules belong around mutable OrgIntel state and Runtime result ingestion. They do not require a
universal command algebra, immutable everything-ledger, governed artifact custody system, or bespoke
durable workflow engine.

### Workplace rules

The useful defaults remain:

1. Exec delegates substantial craft and quiesces after dispatch.
2. One actor owns one outcome.
3. Staff ends every Attempt with a terminal report linked to the artifact or blocker.
4. Provider/runtime failure repairs the same responsibility; reassignment is for capability changes.
5. Messages communicate changed context; Work and edges carry responsibility and dependency.
6. One integration owner converges producer artifacts.
7. An independent critic reviews the native outcome, not producer reasoning.
8. Reversible operating judgement stays with Exec; the owner receives only prepared irreducible
   judgement or authority handoffs.

Most can remain strong, overridable instructions. The exact-duplicate, terminal-result, one-live-actor,
and preserved-recovery rules need deterministic guards because v20 showed that prompt compliance is not
reliable enough to protect organisational truth.

## Score trajectory

Harness-only probes rose from **80** at v01 to **100** at v06/v08/v12, then held **90–96** under live
provider and recovery probes. These scores are not comparable to business outcomes.

Comparable outcome scores were:

| Version | Mode | Score | Dominant observation |
| --- | --- | ---: | --- |
| v05 | single agent | 7 | full mandate caused orientation without output |
| v07 | loose team | 15 | role labels and prose did not create accountable coordination |
| v09 | OrgIntel | 16 | read-only Exec lacked basic perception |
| v10 | OrgIntel | 16 | more perception made executive implementation churn easier |
| v11 | OrgIntel | 29 capped | real artifact, then unsafe automatic finalisation invented success |
| v17 | OrgIntel | 29 | useful Work, but actor identity was coupled to a failing provider |
| v18 | OrgIntel | 30 | Runtime error was durable but invisible at the recovery decision |
| v19 | OrgIntel | **33 peak** | stable multi-provider repair worked; recovery context was too thin |
| v20 | OrgIntel | 21 | exact recovery helped code, then Exec fragmented it into duplicate Work |

The regression is the important result. Harness maturity alone did not yield a commercial outcome.
The score did not rise merely because infrastructure tests passed or partial code existed.

Full dimension evidence is in [`SCORES.md`](./SCORES.md), with each run under [`runs/`](./runs/).

## Free-model evidence

Every live inference used an OpenRouter catalogue entry whose live prompt and completion price were
both zero and which advertised text plus tool support. No paid fallback was permitted.

Observed tendencies, not universal model rankings:

| Model | Observed strength | Observed weakness |
| --- | --- | --- |
| Nemotron 3 Super 120B | strongest free Exec framing and command use | provider overloads; repeatedly ignored ownership/recovery rules and created duplicate Work |
| Cohere North Mini Code | reliable tool calling; produced the only material code/document edits and positive callbacks | expensive repeated discovery; commonly exhausted 18 calls before test/commit/report |
| Nemotron 3 Nano 30B | low-latency cached continuation and correct partial-file observation | repeated searches/insertion-point discovery; no advancement in its full turn |
| Gemma 4 31B / 26B | no capability conclusion possible | Google AI Studio shared pool returned 429 before tokens in every tested production Attempt |
| Laguna XS / S | exercised smaller-model and loose-team paths | shared-pool errors and weak/no outcome in tested runs |

Model diversity helped expose contract fragility. Model-name diversity did not guarantee provider
diversity: both Gemma variants shared the same Google failure domain.

Pi's installed provider snapshot recognised 14 of 19 live zero-price tool-capable OpenRouter entries.
`models.refresh()` did not materialise newer catalogue IDs. This is a real limitation, but not enough to
justify owning a provider/model loop while multiple suitable free models remain usable.

## Retain, defer, and purge

Retain now:

- the thin ACP/Pi harness and explicit launch/result contract;
- live free-model verification and exact model/provider evidence;
- separate durable telemetry and owner-facing live streaming;
- the seven OrgIntel commands;
- Work/Attempt/revision/edge/message/decision/schedule semantics;
- exact recovery context and persistent workspaces;
- single integrator and artifact-bound critic defaults; and
- deterministic duplicate-Work and atomic terminal-result guards.

Defer until demonstrated:

- a Pi fork or custom model loop;
- ACP v2 migration;
- HTTP/SSE MCP support;
- automatic provider health optimisation beyond recorded bounded pools;
- reusable workflow templates as first-class machinery; and
- any marketplace/capability registry implementation.

Purge from the production proposal:

- v2's leases as a general organisational abstraction;
- per-Work Docker cells as the default Company Runtime model;
- its custom single-writer coordinator and outbox as a new workflow engine;
- automatic finalisation after a missing callback;
- hard-coded model identity on Actors;
- full growing model events as durable organisational history; and
- messages as a second kickoff, assignment or review path.

The v2 code remains in scratch as falsifiable evidence and a fault-test fixture, not as production
scaffolding.

## Recommended production slice

Implement one thin vertical slice in the real Runtime Bridge and OrgIntel:

1. Compose Pi behind ACP with the proved launch/result contract.
2. Assemble every wake from stable Actor identity, exact Work/Attempt state, current authoritative
   files, relevant messages and a bounded recovery diff.
3. Route a recorded free/provider pool at the session or Attempt boundary; never change Actor identity
   merely to change inference infrastructure.
4. Enforce one live cognitive process per actor, atomic result ingestion, and explicit unknown outcomes.
5. Reject or challenge exact duplicate open Work; `repair` preserves owner and workspace by default.
6. Run one real company outcome through producer → integrator → independent native-artifact critic.
7. Score it with the same rubric. Do not add a workflow engine unless repeated real runs still require
   one after these smaller controls exist.

A reasonable promotion bar is a reproducible **70+/100** outcome run with a committed artifact,
executable user-path evidence, integrated candidate, independent review, and no owner intervention for
machine-doable recovery. The current peak is 33, so the architecture is ready for a focused production
slice—not for claims that autonomous-company coordination is solved.
