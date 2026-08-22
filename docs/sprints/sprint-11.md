# Sprint 11 — Trustworthy delegated execution under real company work

**Status:** Draft for founder alignment. Implementation is explicitly paused; existing local
prototype changes and live observations are inputs to this spec, not accepted sprint evidence.

**Date:** 21 August 2026

**Spec refs:** `ARCHITECTURE.md` §2.1 / §2.4 / §2.6 / §3.2 / §3.4 / §4.4 / §4.5 / §5 / §6 / §9 / §16,
`orgintel` actors, context, Work, Attempt and wake semantics,
`authority-plane` effects, approvals, credentials and receipts,
`company-runtime` persistent processes, tools, skills and recovery,
`owner-cockpit` executive conversation and attention,
`cross-layer-contract` source ownership,
`evaluation-dogfood` real-company evidence rules

---

## Observed product gap

The Aris publication run crossed several boundaries at once: the owner asked Exec to publish a real
site; Exec had to understand the repository, delegate CI repair, use GitHub credentials, obtain
authority for publication, survive daemon replacement and keep the owner informed while Staff worked.
The run produced useful output, but exposed a single deeper failure:

> Restless can launch intelligent agents, but it does not yet prove that the complete owner intent,
> organisational role, Runtime capability, authority boundary, delegated acceptance contract and
> live owner explanation survive as one closed execution loop.

The symptoms were coupled:

- Exec performed substantial application and CI work itself instead of commissioning Staff Work;
- an agent equated the small ACP native-tool list with the Linux commands available through `bash`
  and concluded that `restless effect` and Git publication were unavailable;
- Git could read a working tree but could not consistently discover the governed credential path;
- temporary model failover could overwrite a durable actor model preference;
- daemon/runtime replacement could leave a wake or Attempt looking active without a terminal event;
- Exec used continuation wakes to poll delegated work that already had a durable completion event;
- Work could be claimed before deterministic acceptance gates were attached;
- acceptance written only in prose could not prevent a false completion;
- shell pipelines such as `pnpm test | tail` masked the failing command's exit status;
- gates with the same timestamp were ordered by random UUID even when one command generated state
  consumed by the next;
- the owner saw a generic waiting block while model text and tool calls accumulated elsewhere, so a
  live run looked stuck and its chronology was difficult to reconstruct;
- stop/reconcile could race an ACP process that had not yet released its in-memory claim;
- a Staff report claimed all four release gates passed while the authoritative run observed test
  exit 1 and build exit 134.

These are not eleven independent bugs. They are one missing structural guarantee: **a delegated
company outcome must remain truthful from intent through execution, authority, evidence and owner
explanation.**

## Value decision

> **Keep ACP as the replaceable agent transport and the persistent Company Runtime as the computer.
> Make Restless own the complete control package around them: role and context, model preference,
> installed capabilities, delegation semantics, ordered acceptance, external-effect authority,
> closed turn recovery and one chronological owner projection. Do not invent a custom agent protocol,
> workflow engine, universal capability API or marketplace to repair missing inputs and evidence.**

ACP need not own company semantics. It must faithfully carry the system prompt, selected model,
bounded native tools and session events Restless supplies. Linux commands, project CLIs and skills
remain ordinary Runtime capabilities reached through `bash`; they do not need to appear as one ACP
tool each.

## Outcome

An owner gives Exec a consequential, repository-backed company outcome. Exec acknowledges the
interpretation, performs only the executive inspection needed to frame it, and commissions the
smallest accountable Staff Work with exact repository coordinates, dependencies and deterministic
acceptance commands. Staff receives the authoritative company rules, assignment, current Attempt
workspace, chosen model, Runtime commands and relevant skills over ACP.

The owner sees prose and tool activity in chronological order as the run unfolds. Delegated waiting
does not poll. A restart closes or recovers interrupted work without duplicating a consequential
effect. Staff claims do not decide completion: deterministic gates run directly, in declared order,
inside the current Attempt workspace. Publication uses the installed Git client through a generic
Authority effect and retains a receipt. Exec reviews the resulting evidence, prepares the native
outcome and reports only what was observed.

## Success contract

Sprint 11 passes only when all of the following are observed:

1. **Restless owns every agent control input.** For Exec and every Staff Attempt, evidence identifies
   the exact authoritative company rules, role, assignment/context package, selected model, native
   tool allowlist, working directory and skill roots passed to ACP. Ambient host rules or extensions
   cannot silently join the session.
2. **ACP remains transport, not company ontology.** Replacing the ACP adapter remains possible behind
   the existing Runtime seam. No custom Restless agent protocol, per-provider agent class or model-
   specific execution branch becomes the source of organisational truth.
3. **Runtime capability discovery is truthful.** Agents distinguish ACP native tools from installed
   Linux commands, probe commands with `command -v`/help/status and read relevant skills before
   declaring a capability absent. A missing command or credential fails explicitly.
4. **Model preference and observed use are separate.** Owner or Exec may explicitly change an
   actor's next-wake model with a reason. Provider cooldown/failover records the actual attempted
   model on the wake/Attempt but does not rewrite that durable preference.
5. **Exec delegates production work.** Repository editing, dependency repair, multi-step CI work and
   domain production become accountable Staff Work. Exec may inspect enough to commission and later
   review, but does not privately implement the delegated outcome or relay ordinary Staff handoffs.
6. **Work enters the graph whole.** A repository-backed Work node, initial `requires`/`revises`
   edges, exact repo/base coordinates and deterministic gates commit atomically before the scheduler
   can claim it. A rejected transaction leaves no schedulable partial node.
7. **Acceptance is executable when it is enumerable.** Every named exit-code requirement is an argv
   gate, not merely prose. Gates run without a shell unless the command explicitly invokes one, so
   pipeline exit masking cannot redefine success.
8. **Gate execution is ordered and revision-correct.** Gates retain declaration order and run in the
   current Attempt workspace on every repair/revision. Generated state from one step is observed by
   the intended next step rather than an incidental timestamp/UUID order.
9. **Model claims cannot settle deterministic Work.** `outcome_met` with a failed or missing gate
   leaves the Attempt failed/blocked and wakes the accountable coordinator with exact evidence. A
   prose report saying “all gates pass” has no special authority.
10. **Delegated waiting is event-driven.** When Staff Work or an observable external process is in
    flight, Exec ends in a waiting posture with no timer. Its terminal Staff message, Work transition
    or external observation produces the next wake. A genuine immediate continuation remains bounded.
11. **Every wake and Attempt closes.** Success, refusal, timeout, transport loss, daemon shutdown and
    Runtime replacement produce or recover one terminal outcome. Restart reconciliation neither
    strands an active claim nor launches duplicate Exec/Staff work.
12. **Lifecycle replacement is race-safe.** The documented stop → reconcile path waits for supervised
    ACP work to release its claim or refuses with a bounded diagnosis. It preserves the company volume,
    browser profile, Git work and durable OrgIntel state.
13. **Git read and Git publish have different authority.** Public fetch/status and local commits are
    ordinary Runtime work. A credentialed fetch uses scoped credentials. A push is a consequential
    generic effect with party, purpose, idempotency key and receipt; the owner is never told that a
    machine-doable push is unavailable merely because it is not an ACP-native tool.
14. **Skills remain lightweight capability knowledge.** The first proved Git publication may yield a
    small skill/playbook plus credential helper. It does not introduce a capability registry,
    universal `execute_capability`, provider workflow model or Restless marketplace.
15. **The live conversation is chronological.** Assistant updates and tool start/update/completion
    events interleave in their observed order, stream while the turn runs, retain a compact expandable
    trace after completion and never expose hidden chain-of-thought, system prompts or secrets.
16. **Conversation history has one source.** Durable messages remain the canonical history; the live
    stream is an ephemeral projection attached to its trigger message. Reconnect, pagination, draft
    preservation and “sent message at top, response grows below” scrolling do not create a second
    transcript store.
17. **Owner status is honest and calm.** Thinking/waiting has a restrained glimmer and elapsed time;
    inactivity, completed work, failed work and delegated waiting are distinct. Generic instructional
    composer copy and permanently expanded raw traces are removed.
18. **One real outcome closes the loop.** After `_test` crash, gate and UI scenarios pass, Aris runs
    the repository-backed publication path with a real model and real GitHub account: Staff repairs,
    gates pass directly, Exec reviews, an authorised push receives a receipt, the deployed native site
    is probed and the owner receives the observed result. No simulated provider enters Aris evidence.

## End-to-end control loop

```text
owner intent
  → Exec ACP session (Restless-owned prompt/model/tools/skills)
  → atomic Work + edges + ordered gates
  → Staff ACP Attempt in the bound persistent worktree
  → direct deterministic gate evidence
  → Exec review and authority request
  → generic Git publication effect + receipt
  → live deployment probe
  → chronological owner explanation
```

Each arrow has one authoritative owner. The owner conversation does not become a second Work graph;
the Work graph does not become an effect ledger; the effect receipt does not claim the business
outcome; the cockpit does not become a second transcript database.

## Layer slices and ownership

| Concern | Authoritative owner | Sprint 11 responsibility |
| --- | --- | --- |
| Actor identity, durable model preference, Work/Attempt, dependencies, gates, messages and wakes | OrgIntel | Preserve delegation and executable acceptance as recoverable organisational truth |
| Approval, credential scope, consequential Git effect, idempotency and receipt | Authority Plane | Govern publication without governing ordinary reads, builds or local Git work |
| ACP adapter, persistent processes, worktrees, installed commands, skills and credential helper | Company Runtime | Deliver the complete control package and execute real tools in the company computer |
| Live event projection, chronology, history anchor, reconnect and disclosure | Owner gateway/cockpit | Explain the one underlying run without owning Work, effects or a second transcript |
| Business judgement about delegation, repair, provider use and outcome quality | Exec/Staff intelligence | Decide within the deterministic boundaries above; no static flow replaces judgement |

## Problem classification

**Deterministic and enumerable:** ACP launch arguments, tool allowlist, model selection record,
worktree binding, atomic graph transaction, argv gates, declaration order, exit status, wake closure,
claim release, effect approval/receipt and stream sequence offsets.

**Judgement and open-ended:** whether work deserves delegation, which durable specialist adds value,
how to repair a failed gate, whether an external capability should be built/bought/delegated, which
ReviewTarget best represents the result and whether the economic outcome is good enough.

The sprint must not use more prompting to enforce exit codes, nor a state machine to decide every
delegation. Intelligence chooses the work; deterministic substrate proves the bounded facts.

## Evidence already observed — not acceptance

The interrupted Aris run is diagnostic evidence only:

- Exec and `ci-engineer` ran through ACP using `anthropic/claude-sonnet-4-5`;
- Exec created Work `5d5d652b-c73d-4918-898a-0009a751d09b` with four initial release gates;
- Staff's first report claimed all gates passed after shell pipelines obscured failures;
- authoritative gate runs observed typecheck/lint exit 0, test exit 1 and build exit 134;
- OrgIntel blocked the Attempt and woke Exec rather than accepting the report;
- Exec independently inspected the evidence, resumed the same accountable Work and returned to
  event-driven waiting while the second Staff Attempt ran;
- a governed branch push earlier in the run produced receipt
  `8eea38cd-15fa-4ffd-a39a-3547a1d391e9` rather than pushing `main` directly;
- daemon replacement exposed an interrupted-wake and stopped-container reconciliation race;
- the owner surface previously separated accumulated prose from tool activity and could remain on a
  generic waiting state for over an hour.

These observations justify Sprint 11. They do not check any ticket: the current worktree is dirty,
some changes predate alignment, the second Aris Attempt was still running when implementation was
paused and the latest integrated Runtime has not completed a clean end-to-end rerun.

## Risks and dispositions

| Risk | Disposition | Why |
| --- | --- | --- |
| A capable model still makes incorrect claims | **Accepted** | Intelligence is fallible; direct evidence owns enumerable acceptance |
| Exec delegates trivial one-step work | **Accepted** | Delegation remains judgement; dogfood tunes the prompt without a static classifier |
| Exec implements substantial Staff work itself | **Guarded** | Context contract, Work evidence and dogfood make the boundary observable |
| Gate commands mutate shared Attempt state | **Guarded** | Declared order and revision-bound workspaces make the pipeline explicit |
| A failed gate is hidden by a shell pipeline | **Invariant** | Gates store argv and direct process exit; a shell must be explicitly chosen |
| Model failover changes organisational preference | **Invariant** | Preference changes are explicit; attempts record actual models separately |
| Restart duplicates a consequential effect | **Invariant** | Wake recovery reconciles durable intent/receipt before retrying |
| Live trace exposes chain-of-thought or secrets | **Invariant** | Only safe assistant updates and bounded tool metadata reach the cockpit |
| Git support grows into a speculative marketplace | **Accepted** | One skill/helper is enough until repeated external capabilities prove a shared shape |
| Existing uncommitted founder work is overwritten | **Invariant** | Sprint implementation must preserve unrelated dirty-worktree changes |

## Non-goals

- a bespoke replacement for ACP;
- a universal agent, tool or provider protocol;
- a capability registry or marketplace;
- a durable workflow engine;
- a kernel command for messages, Work, Git commits, tests or every Runtime mutation;
- raw chain-of-thought display;
- a second conversation/history database;
- making Restless merchant of record or holder of company operating funds;
- fixing every existing Aris application defect merely to increase a green-test count.

## Tickets

| Status | Ticket | Layer | Observed friction served | Depends on |
| --- | --- | --- | --- | --- |
| [ ] | [**S11-T0 · Freeze the trustworthy delegated-turn contract**](sprint-11/t00-control-contract.md) | Cross-layer | ACP, OrgIntel, Authority, Runtime and cockpit each held part of the run but no single end-to-end contract named their boundaries | — |
| [ ] | [**S11-T1 · Deliver a fully Restless-controlled ACP session**](sprint-11/t01-acp-control-package.md) | Runtime + OrgIntel | Agents misread available capabilities and failover rewrote durable model choice | S11-T0 |
| [ ] | [**S11-T2 · Commission repository Work with atomic ordered acceptance**](sprint-11/t02-delegation-and-gates.md) | OrgIntel + Runtime | Exec implemented Staff work, Work raced late gates and model prose could overrule failed checks | S11-T0, S11-T1 |
| [ ] | [**S11-T3 · Close and recover every wake without polling**](sprint-11/t03-wake-recovery.md) | OrgIntel + Runtime | interrupted turns remained active, restart raced claims and delegated waiting spent repeat wakes | S11-T0, S11-T1 |
| [ ] | [**S11-T4 · Prove Git as an acquired governed capability**](sprint-11/t04-governed-git.md) | Runtime + Authority | Staff could use Git locally but concluded governed publication was unavailable | S11-T0–T3 |
| [ ] | [**S11-T5 · Project one chronological executive conversation**](sprint-11/t05-conversation-timeline.md) | Owner gateway + cockpit | live prose and tool calls were separated, slow work looked stuck and history behaviour was implicit | S11-T0, S11-T3 |
| [ ] | [**S11-T6 · Dogfood Sonnet on Aris and purge false paths**](sprint-11/t06-aris-dogfood.md) | All touched layers | structural claims remain unproved until one real delegated publication closes under restart and failure pressure | S11-T1–T5 |

Ticket status lives only in this checklist. Existing code or one passing component test does not check
a ticket. Founders align on this spec and the ticket set before implementation resumes.

## Verification story

### 1. Deterministic `_test` company

Use a real local Git remote and ordinary commands, not a simulated provider outcome. Prove:

1. the exact ACP control package for Exec and two Staff actors;
2. explicit model preference followed by provider failover without preference mutation;
3. atomic Work creation with repository coordinates, dependencies and four ordered gates;
4. a gate whose first command generates state consumed by the second;
5. a deliberately misleading Staff `outcome_met` while one direct gate exits non-zero;
6. daemon termination during Exec text streaming, during a Staff tool call and after a material
   effect intent but before local acknowledgement;
7. one terminal recovery, no duplicate effect and no polling wake while Staff is active;
8. chronological reconnect to the same durable conversation and active ephemeral stream.

### 2. Owner-surface verification

At 390, 768 and 1440 CSS pixels, observe a real running turn with text → tool → text → tool order,
elapsed state, restrained glimmer, automatic post-send anchoring, history pagination/reconnect and a
collapsed completed trace. Confirm no system prompt, hidden reasoning, secret value or raw credential
appears. The final visual pass follows `docs/FRONTEND_DESIGN_REFERENCES.md` and the working agreement.

### 3. Real Aris publication

Only after the `_test` run passes:

1. pass `restless doctor -c aris`;
2. send one owner publication intent through the cockpit;
3. observe Exec commission repository-bound Staff Work rather than implement it;
4. make at least one real gate fail, observe repair and then obtain four direct passing gate runs;
5. review the exact branch/commit and native site candidate;
6. perform the authorised GitHub push through a generic effect and retain its receipt;
7. observe provider/CI/deployment state and probe the final public URLs;
8. report the deployed business result and remaining uncertainty without claiming more than the
   provider and live probes establish.

### 4. Purge

Remove the UI actor-id allowlist, late-gate race path, capability-unavailable prompt folklore,
failover-as-preference mutation, polling schedules for delegated Work, non-chronological live trace,
stale generic composer instruction and any duplicate Git publication wrapper that the retained path
makes unnecessary. Record every deletion and remaining accepted risk in the run report.
