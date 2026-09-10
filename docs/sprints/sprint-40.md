# Sprint 40 — Take control of the exact Codex work and return it

**Status:** Draft for founder alignment  
**Date:** 4 September 2026  
**Programme:** [Open Company Runtime](./open-company-runtime-programme.md)  
**Depends on:** Sprint 39's native Codex session identity and Attention work-through, the existing
Company Computer, and the current Work/Attempt/Git recovery contract. Sprint 38's bounded native-launch
machinery may be reused after its exact ownership and cleanup assumptions are revalidated.

## Why this sprint exists

Restless can run Codex as a certified coordination or worker harness, and the owner can discuss hard
work through Attention. But low-level intervention still requires leaving the product, rediscovering
the workspace and starting an unrelated session. That is an abstraction trap: the managed path works
until the moment the owner most needs the harness's genuine controls.

Codex is the right first vertical proof because Restless already owns the native App Server thread and
Codex supplies a real remote terminal client. The sprint should prove one complete transfer and return
before inventing a harness-neutral application platform.

## Outcome

From an eligible active Work or Attention item, the owner selects **Take the controls**. Restless stops
new autonomous input, reaches a truthful safe boundary, checkpoints the exact workspace and opens the
genuine Codex terminal UI against the same Restless-owned App Server thread.

The owner inspects context, changes files and redirects Codex. On **Return to Restless**, the native
client detaches, Restless observes the final thread/workspace state and presents one bounded change
account. The owner may resume, redirect, reassign or leave the Work paused. No takeover action implies
completion, approval or Attention resolution.

## Frozen product decisions

1. **Take the controls extends existing Work and Attention.** It is not a new focus-session object,
   developer dashboard or harness picker.
2. **The counted path uses the real Codex terminal client.** A Restless transcript skin is not native
   takeover.
3. **One controller at a time.** Restless does not send autonomous turns while the owner holds control.
4. **Safe boundary before transfer.** An active tool call is acknowledged, interrupted or allowed to
   finish under an explicit bound; it is never silently orphaned.
5. **The same session must be proved.** The UI may say “same Codex session” only after thread identity
   and App Server acknowledgement match.
6. **The same workspace must be proved.** The client starts in the exact attempt worktree and revision,
   not a repository default guessed from process cwd.
7. **Control state is recoverable OrgIntel/session state.** It is not an Authority grant over ordinary
   files and does not introduce constitutional file locks.
8. **Checkpoint precedes native access.** Abandonment, terminal death and Runtime restart retain a known
   pre-takeover state.
9. **Return is explicit.** Client exit or network loss marks control uncertain/abandoned until Restless
   inspects it; it does not automatically resume the actor.
10. **Reconciliation describes observations.** It does not invent command history, test success or
    external effects that Restless did not observe.
11. **Consequential effects remain governed.** Direct Codex access receives only the same scoped MCP
    and Authority paths admitted for the session.
12. **No reusable remote token appears in URL, argv, logs or shell history.** The local launch broker
    exchanges a short-lived handle and cleans it when control ends.

## Success contract

1. One hard-goal Attention journey reaches the correct accountable actor's current Codex session from
   the existing rail or Work context in at most one consequential owner action.
2. Takeover records exact company, actor, Work, attempt/conversation, thread, session, cwd, revision,
   model, control owner and checkpoint identity.
3. A running turn transfers only after the declared safe-boundary result; race, cancellation and
   already-terminal cases are deterministic.
4. The real `codex --remote` client connects to the exact Restless-owned App Server over a local or
   authenticated forwarded transport.
5. Restless sends no autonomous input and starts no replacement attempt while owner control is active
   or uncertain.
6. Changes made through the native client remain ordinary files and Git state in the expected
   worktree and are visible immediately to the owner.
7. Return distinguishes changed, unchanged, dirty, conflicted, externally affected and unobservable
   state without collapsing unknown into success.
8. The owner can resume, redirect, reassign or remain paused; only existing typed actions settle Work
   or Attention.
9. Terminal crash, App Server death, Runtime restart, duplicate Take control and abandoned client each
   recover to one truthful control state with no concurrent writer.
10. Cross-company, cross-actor, wrong-thread and expired-handle attempts fail before native control.
11. Process, socket, token, temporary checkpoint and control-state cleanup passes after normal return,
    cancellation, expiry and crash.
12. A source-blind owner can explain whether they are observing, controlling or returning work and
    whether the native session is exact.

## Slice per layer

**Authority and host.** Issue one short-lived local native-client handle and preserve current effect,
credential and budget boundaries. Authority does not own the editor, worktree or control workflow.

**OrgIntel.** Record current controller, transfer intent, safe-boundary result and return disposition on
the existing Session/Attempt/Work relationship. It does not store terminal keystrokes or file custody.

**Runtime Bridge.** Pause input, checkpoint the workspace, expose the owned App Server through a bounded
transport, verify exact session/cwd, observe detach and perform cleanup.

**Owner cockpit and machine client.** Add **Take the controls**, native launch, visible control state,
**Return to Restless** and the bounded change account inside existing Work, Attention and Company
Computer patterns.

## Salvage

- Reuse Sprint 39 Codex thread, usage, cancellation and MCP policy only after T0 proves the native
  client observes the same identifiers and cannot augment ambient configuration.
- Reuse Sprint 38's opaque native-launch handle only after its audience, expiry, argv/log secrecy and
  exact cleanup corpus passes for an interactive terminal client.
- No earlier raw shell or ad hoc port-forwarding path is admitted as product behavior.

## Out of scope

- Claude takeover;
- a generic application or protocol registry;
- full Codex/ChatGPT desktop installation inside Runtime;
- arbitrary SSH, port forwarding or shell commands from the owner API;
- simultaneous agent and owner writes;
- importing unrelated personal Codex sessions;
- granting personal Codex plugins, MCP servers or remembered approvals to autonomous work; and
- changing company harness defaults during takeover.

## Stop rules

Stop if the real Codex client cannot attach to the exact owned thread without exposing a reusable
credential, if App Server cannot enforce one controller, if return cannot distinguish known changes
from unknown state, or if the implementation introduces a general remote-shell surface.

Do not replace exact attachment with a new Codex session and retain “same session” copy. A
reconstruction is a future explicit fallback, not a hidden success in this sprint.

## Ticket index

Status lives only here.

| Status | Ticket | Outcome |
| --- | --- | --- |
| [ ] | [S40-T0](./sprint-40/t0-freeze-takeover-corpus.md) | Freeze exact-session, control-race and return evidence |
| [ ] | [S40-T1](./sprint-40/t1-control-and-checkpoint.md) | Add one recoverable controller and pre-takeover checkpoint |
| [ ] | [S40-T2](./sprint-40/t2-secure-codex-attach.md) | Attach the genuine Codex TUI to the owned App Server |
| [ ] | [S40-T3](./sprint-40/t3-owner-take-control-surface.md) | Extend Work, Attention and Company Computer with truthful controls |
| [ ] | [S40-T4](./sprint-40/t4-return-and-reconciliation.md) | Return control with one bounded observed change account |
| [ ] | [S40-T5](./sprint-40/t5-dogfood-recovery-and-purge.md) | Prove the complete journey, recovery and exact cleanup |

Expected order: **T0 → T1/T2 → T3 → T4 → T5**.

## Terminal decision

- **Pass:** exact Codex takeover and return improve one hard owner intervention without lost company
  state, concurrent mutation or credential residue.
- **Revise once:** repair one bounded attachment, control-race or reconciliation defect.
- **Stop negative:** retain managed Codex and ordinary Company Computer access, and do not generalise a
  native-takeover abstraction that cannot preserve the exact session safely.
