# S05-T8 · One Work graph, deterministic handover, ordinary governed tools

**Status:** Complete — all thirteen acceptance checks passed; the four real emails remain held  
**Layer:** OrgIntel + Runtime Bridge + Authority + owner projection  
**Observed run:** Sprint 05 Aris landing-page/PDF/outreach continuation, 16 August 2026

## Outcome

> Restless carries one piece of company work from author to independent review,
> correction, executable verification and a held external send without an Exec
> manually relaying every step, inventing timers, or asking the owner to perform
> machine-doable work.

This ticket is justified by repeated live friction, not a speculative workflow
ontology. In the Aris run:

- a verifier started before the author's artifact was complete;
- a verifier adopted an author's incorrect cube-net answer, and later report
  edits left contradictory versions behind;
- Exec repeatedly asked the owner to run builds, render a PDF, solve arithmetic
  and inspect machine-checkable output;
- work split across messages, commitments, output files, ad-hoc branches,
  worktrees and in-memory process state with no canonical input version;
- a known-exhausted primary model was retried on every Exec and Staff wake;
- an arbitrary 25-minute continuation was scheduled with no external trigger;
- direct `spawn`, end-of-turn `spawn`, event wakes and timer wakes could all
  initiate overlapping interpretations of the same work;
- one owner Request changes transaction emitted both message and Work hints,
  and those hints started two Exec sessions against the same worktree;
- the isolated governed-effect UID could receive a GitHub credential but could
  not read the persistent Git metadata created under the actor's `077` umask;
- the explicit `email.send` adapter could not attach the PDF even though the
  provider's ordinary open-source CLI supports attachments, JSON output,
  dry-run and idempotency.

The repeated failure has crossed `orgintel` §6.1's trigger for stronger
coordination. This remains recoverable OrgIntel state, not a Kernel workflow
engine.

## One canon

### Work evolves Commitment

There is no parallel `Work` and `Commitment` truth. The existing commitment row
is migrated into a Work node and the old table, CLI spelling, spawn path and
termination-envelope spawn field are removed after migration.

A Work node carries only what the observed handoff needs:

```text
work_id
goal_id
outcome contract
accountable actor
status
priority
expected artifact or decision
repo + base + integration branch + worktree where applicable
revision
attempt limit where explicitly chosen
blocker / resolution
```

### Graph semantics

The graph is a directed multigraph with two edge meanings:

- `requires`: the downstream node may start only when the current revision of
  every upstream requirement has an accepted result. The `requires` subgraph
  must be acyclic; an attempted cycle is reported as a deadlock.
- `revises`: a reviewer or verifier may request a new revision of an upstream
  node. These edges may close cycles.

The reusable graph may therefore be cyclic. Its execution history is not:

```text
author attempt 1
→ verifier attempt 1 requests changes
→ author attempt 2
→ verifier attempt 2 accepts
```

Every Attempt records the exact Work revision and artifact inputs it consumed.
When an upstream result changes, descendants built from the prior version
become `superseded`; they cannot remain silently accepted.

### Deterministic kickoff and handover

Readiness is computed from graph state, not from model prose or a remembered
message. OrgIntel atomically claims one ready node and asks the Runtime Bridge
to start its assigned actor with a generated kickoff package:

```text
work + attempt identity
outcome and acceptance contract
exact upstream artifact references and revisions
repo/base/integration branch/worktree
owner feedback cursor
gates
authority and runtime pointers
completion/blocker contract
```

An attempt ends as one of:

```text
produced | changes_requested | blocked | failed | abandoned | superseded
```

The result, not the actor's message, advances the graph. Messages remain
ordinary free-form organisational conversation.

### Evidence and artifacts

Files remain ordinary Runtime files. OrgIntel records references only:

```text
path/repo/URL/receipt locator
runtime generation where relevant
digest or source commit where useful
producer attempt
available/stale/missing/superseded/unknown
```

There is no custody, export/import or content-addressed asset lifecycle.
Independent verification is tied to the exact artifact version inspected.

### Executable gates

An outcome contract may name ordinary Runtime commands as gates. Runtime runs
them and returns exit status plus bounded output/digests. A model claim never
substitutes for a feasible build, test, render, exact-text or external probe.
Gates are evidence attached to an Attempt, not a workflow language.

### Owner handover

An owner handover is valid only for an irreducible category:

```text
identity | captcha | mfa | legal_attestation |
payment_confirmation | owner_judgement
```

It must name the prepared state, exact owner action, attached Work and, when a running Attempt is
handing over, that exact Attempt, plus an observable resume condition. An owner-judgement review has
separate **Accept outcome** and **Request changes** decisions; free-form chat with the accountable
lead never resolves it implicitly. Shell commands, builds, rendering, arithmetic and ordinary file
edits are invalid handovers. Browser controller release never resolves the Work by itself.

### Provider continuity

A classified provider refusal writes a bounded cooldown with reason and retry
time. Exec and Staff use the same availability reading and skip a known-down
candidate until expiry. Model order remains owner configuration; cooldown does
not invent a fallback.

### Ordinary governed tools

Restless does not own `email.send`, `repo.push`, or a provider command
catalogue. An actor prepares an ordinary non-interactive command such as
`resend emails send --json --attachment ...` or `git push ...`. For a material
consequence, the generic effect runner:

1. receives the effect class, party, purpose, idempotency key, executable,
   arguments, working directory and named secret bindings;
2. checks Authority grants/approval and prior/unknown intent;
3. injects each resolved secret only into that process environment;
4. runs the command in the Company Runtime without a shell;
5. records exit status and parsed/opaque JSON as the provider result;
6. writes one generic receipt with redacted invocation and attachment/artifact
   metadata.

Credential material is never placed in argv, persisted CLI login state,
Runtime-wide environment or the receipt. `_test` companies cannot receive
live secret bindings and therefore cannot reach a live provider.

Work-linked feedback that activates a revision is one deterministic Attempt
input. Its database message notification must not also start a free-form actor
session. Productive files are group-private to the company actor and isolated
effect UID; the credential remains visible only in the governed child.

## Acceptance

The ticket passes only when one `_test` run proves all of the following:

1. An author and independent verifier are created as one graph, with an executable gate attached to
   the producer Work rather than modelled as another actor or workflow node.
2. The verifier cannot start before the exact author artifact is ready.
3. A verifier `changes_requested` result creates author attempt 2; attempt 1
   and every dependent result built from it are visibly superseded.
4. The verifier's second attempt consumes author attempt 2's artifact digest,
   not a mutable-path assumption, and an executable gate passes against it.
5. A daemon restart between nodes does not lose readiness or start a duplicate
   Attempt.
6. An active actor crash preserves its worktree, closes the Attempt honestly,
   and makes recovery/reassignment deterministic.
7. A non-handoff category such as generic machine work is rejected; the actor performs machine work
   itself. A prepared browser identity handoff blocks only its Work and resumes from an observed condition.
8. A seeded quota refusal cools the primary provider; both Exec and Staff skip
   it until retry time and use only the explicitly configured fallback.
9. `resend emails send --dry-run --json` executes through the generic runner
   with a test PDF attachment and no live secret or external send.
10. A deterministic fake CLI exercises success, safe failure, replay,
    different-args/same-key refusal and unknown-outcome reconciliation through
    the same generic receipt writer.
11. The SPA/CLI project the same Work, Attempt, artifact, gate and handover
    truth from OrgIntel/Authority; neither is a writer.
12. Searches and mutation guards show no direct Staff spawn path, no
    end-of-turn spawn field, no `commitments` table/API, and no Resend/Git
    provider adapter remaining.
13. The four real Aris emails remain unsent throughout this ticket.

## Narrow debt carried into this ticket

- provider cooldown and quota-aware failover;
- canonical artifact versioning and invalidation;
- executable acceptance gates;
- automatic repo/base/branch/worktree inheritance;
- admissible prepared-last-mile owner handoffs;
- credential input normalization and live capability probes;
- generic governed CLI effects with attachment support;
- external delivery/status evidence after a send;
- stale/split source-of-truth removal across CLI, daemon, OrgIntel and SPA.

## Deletions required

- `SpawnRequest` and both spawn initiation paths;
- the in-memory Staff registry as work custody (process observation may remain
  in the Runtime Bridge);
- `commitments` as a second name/table/API for Work;
- event rows used as the canonical schedule store;
- hard-coded `Provider::{Resend, Git, SelfReported}` dispatch and provider
  credential resolution by capability;
- context text claiming every network interaction is an effect;
- mutable artifact reports treated as acceptance evidence without version.

## Risks and dispositions

| Risk | Disposition |
|---|---|
| The graph becomes a bespoke durable workflow engine | **Guarded:** two edge meanings, one Attempt contract, ordinary files/tools, mutable recoverable state, no action DSL |
| A hard dependency cycle deadlocks | **Guarded:** reject cycles in `requires`; show the path that closes it |
| Revision cycles run forever | **Guarded:** explicit attempt limit/budget or Exec stop decision; every traversal is visible |
| Stale OrgIntel state blocks useful files | **Accepted:** direct Runtime work continues; reconcile graph forward |
| Generic commands become a shell-injection boundary | **Invariant:** argv vector, validated workdir, no shell, scoped env injection, bounded output |
| A credentialed command bypasses Authority later | **Guarded:** no persistent material-effect login; consequential secret refs resolve only in the governed launcher |
| Provider JSON shape changes | **Accepted:** retain opaque result plus exit status; promote fields only after repeated use |

## Explicit non-goals

- a general workflow DSL, BPMN engine or universal command algebra;
- exactly-once internal messages or sessions;
- gating ordinary files, Git, shell, browsing or research through Work;
- provider-specific Restless APIs;
- a content-addressed custody machine;
- turning free-form owner/actor conversation into a state machine.
