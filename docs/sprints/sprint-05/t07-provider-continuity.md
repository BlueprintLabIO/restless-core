# S05-T7 · Contextual owner handover and explicit provider continuity

**Layer:** OrgIntel + Company Runtime + Authority credential boundary + Owner surface
**Serves:** keeping the singleton Exec available through a real provider outage without hiding who
performed the work or handing the recovery workflow back to the owner
**Depends on:** S05-T5's controller lease and S05-T6's host OMP broker/gateway
**Makes deletable:** manual model edits during an outage, Runtime-held provider OAuth, and static
handover instructions that leave the owner to infer when control is safe

---

## Observed trigger

Aris reached Kimi through the real host credential path, worked for 54 tool calls, then exhausted the
provider's billing-cycle allowance. The singleton Exec stopped even though the company computer,
files, OrgIntel and Authority remained healthy. In the same live system the owner used the persistent
browser to authenticate a Claude subscription, returned the controller lease, and a bounded OMP prompt
returned the expected answer. Restless cannot yet use that proven second provider, and the SPA's only
handover explanation is a static footer.

This is no longer speculative multi-provider machinery. It is one observed outage and one observed
owner recovery path.

## Contract

1. `model` remains the primary provider-qualified model. `model_failover` is an ordered, owner-set list
   of provider-qualified models. No configured entry means no fallback; no provider is discovered from
   ambient keys or old broker state.
2. Automatic failover applies to the singleton Exec and to Staff that inherit the company policy. A
   Staff actor with an explicit model tries that model first, then the remaining configured company
   candidates without duplicates. The actor, Work, Attempt inputs, worktree and task remain the same across
   attempts. This extension was triggered by the live feedback-propagation run: the Exec survived the
   Kimi outage on Claude, then its newly spawned writer died on Kimi alone.
3. Credential, quota/rate, unavailable-model and model-session transport failures may advance once to
   the next configured candidate. Disk, Runtime, budget, ordinary business blockage and owner gates do
   not. Each candidate is attempted at most once per wake.
4. A fallback starts a fresh ACP session over the same durable company computer and the same assembled
   organisational context. Its context names the failed candidate and instructs Exec to reconcile any
   material external effect through existing Authority receipts/idempotency before repeating it. No new
   workflow engine or provider API is introduced.
5. Every attempt records the actual model and usage. A `model_failover` operational event and the wake
   report name `from`, `to`, failure class and a bounded reason. The final actor projection names the
   model that actually completed or blocked the wake.
6. API-key references continue through `env:` or `infisical:`. Subscription OAuth is named explicitly
   as `omp-oauth:<provider>` and remains in the host Restless OMP broker; the Runtime receives only the
   existing narrow gateway bearer. A stale broker credential that is not named by any company remains
   unroutable.
7. OAuth subscription telemetry is not actual API spend. Restless records tokens and the provider's
   reported/list-price estimate, but the charged-spend ledger receives zero dollars for a subscription
   turn. Provider allowance/quota is still classified as an availability failure.
8. Browser focus mode carries a free-form conversation with the actor that requested the intervention,
   whether Exec or Staff. The conversation is pre-grounded in the Attention envelope, prepared browser
   state, evidence, authority boundary and current controller status. Messages remain ordinary OrgIntel
   coordination; there is no handover conversation state machine. Take and Return remain deterministic
   and available with every model offline.
9. Message persistence is not model presence. A reply is addressed to the durable requesting actor and
   must enter that actor's context on its next run/resume. Until the Runtime can inject a new prompt into
   an already-running Staff ACP session, the surface must not imply live delivery or invent a scripted
   acknowledgement; the persisted message and controller hand-back remain independently true. When an
   Exec trigger arrives during an active wake, the scheduler coalesces it into one pending continuation
   and starts that continuation after the company slot is released. The message remains ordinary durable
   OrgIntel state; the pending reason is scheduling mechanics, not a second copy of the work.

## Acceptance

1. Config/CLI round-trip preserves the primary plus ordered fallbacks and rejects malformed or duplicate
   candidates. A daemon restart exposes only providers named by at least one configured policy.
2. The authenticated Claude credential is migrated from the isolated Runtime test profile into the
   host broker without printing credential material. Exact-value/process/volume checks show that the
   Runtime receives only the gateway bearer; the redundant Runtime credential is removed after proof.
3. In a `_test` company, the primary fails with a classified provider error and the next configured
   Claude model completes a bounded task. The report and OrgIntel events show both model names, the
   failure class and the final model.
4. A non-provider blockage does not advance the policy. Exhausting all candidates blocks once with an
   honest summary rather than scheduling a retry storm.
5. The subscription turn records tokens and an estimated provider cost separately, adds `$0` to charged
   API spend, and does not poison the ledger for missing billable cost.
   The same accounting and refusal semantics hold for inherited Staff attempts.
6. The actual SPA names the requesting actor and supports free-form owner messages with the source
   context already visible. Owner control excludes agent input; clean hand-back restores observation;
   neither transition resolves the item or grants an effect; both controls still work with model
   providers unavailable. The intended actor receives the persisted message on its next context
   assembly/resume; an already-running Staff turn is either proven to receive it or honestly shown as
   queued rather than live. A successful send visibly confirms durable delivery and automatic reply
   polling; it does not imply that the actor has read or acted on the message.
7. Aris keeps Kimi primary. Adding Claude to Aris is a visible owner configuration step after the
   `_test` proof, not a silent code default. The four centre emails remain unsent.

## Risks

| Risk | Disposition | Why |
|---|---|---|
| Fallback repeats work from the failed session | **Guarded** | One attempt per candidate, durable files/context, explicit reconciliation instruction, and existing effect idempotency; ordinary file work may still be repeated and is accepted |
| Fallback becomes an opaque router | **Invariant** | Ordered company config plus per-attempt report/event; OMP only applies credentials and transport |
| OAuth refresh token enters the Runtime or repo | **Invariant** | Host broker custody, narrow gateway bearer, exact-value scans and deletion of the isolated migration copy |
| Different model changes judgement | **Accepted** | Availability is the chosen priority; actual model identity and cause stay visible to the owner |
| A provider outage causes an unbounded retry loop | **Invariant** | Finite unique candidate list, one attempt each, then one blocked outcome |
| Owner mail arrives while Exec is closing its turn | **Guarded** | Busy triggers coalesce into one pending continuation. In `aris_feedback2_test`, mail sent during wake 158 started wake 165 automatically one second after wake 164 closed; the new context recorded `QUEUE-PROOF-20260816`, with no manual wake or effect |

---
Sprint spec: [`../sprint-05.md`](../sprint-05.md)
