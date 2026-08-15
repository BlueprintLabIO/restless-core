# S05-T3 · Launch gate → first real centre outreach

**Layer:** One end-to-end slice across Owner surface, Authority, OrgIntel and Runtime  
**Serves:** Sprint 05's commercial outcome  
**Depends on:** S04-T4 (owner merge), S05-T1 (the bounded launch queue)  
**Makes deletable:** the zero-contact simulated panel as evidence for the tutoring-centre channel

---

## Observed starting state

Sprint 04 ended with machine-doable preparation complete:

- centre-offer branch `feat/tutoring-centre-offer` at `4eb3345`, pushed with a compare URL;
- a separate pricing-document correction at `4e18414`, also pushed with a compare URL;
- four public email parties verified independently by a Kimi researcher and Exec;
- four tailored first-contact drafts offering the real February 2026 sample;
- static sample PDF observed HTTP 200;
- centre page 404 until merge, and interactive booklet/health probes returning 500/503;
- no party approvals and no email sends.

This ticket starts from those files and receipts. It does not regenerate the offer or prospects.

## Scope

1. The owner opens/merges the prepared centre-offer compare link. This remains a human authority act;
   no forge API or PR lifecycle is added.
2. Restless observes the repository's existing deployment. The centre page must return 200 with the
   expected title before any outreach links to it.
3. The owner grants or declines first contact for each exact canonical party. The four current
   candidates are BrainTree, Global Education Academy, Pre-Uni New College and Matrix Education.
4. Exec re-probes the sample PDF, booklet route and health endpoint through ordinary runtime/browser
   tools. If the interactive QR path remains unhealthy, it removes the marked QR paragraph from the
   prepared emails and sends the useful static PDF offer; it does not claim a broken capability.
5. Each approved email leaves through `email.send`, with a unique stable idempotency key and provider
   receipt. Direct provider calls remain forbidden.
6. Replies enter through the governed inbound path. The company records the first concrete objection
   or buying signal and chooses the next offer change. If none arrive within five business days, it
   records bounded non-response and decides whether to change segment, subject or offer.

**Not in scope:** a deploy adapter, forge API, CRM, bulk-email system, sequence builder, invented lead
database, or custom durable workflow engine.

## Acceptance

1. Provider/external observation shows `/for-tutoring-centres` live after the merge; a receipt or agent
   statement alone is insufficient.
2. Every recipient is one of the double-verified canonical parties and has an owner approval grant
   before first contact.
3. At least four approved first-contact sends produce distinct `email.send` receipts; replaying any
   key does not send twice.
4. The actual body sent contains no claim contradicted by the final production probes. A red QR path
   removes the QR paragraph rather than blocking the static-PDF offer.
5. `restless attention`, `receipts`, `people` and `spend` let the owner follow the run without `psql`
   or searching ordinary mail for an outstanding action.
6. A reply/objection is recorded if observed. Otherwise, after five business days, a dated
   non-response finding and the next bounded experiment close the ticket honestly.

## Risks

| Risk | Disposition | Why |
|---|---|---|
| First contact becomes spam | **Guarded** | Four hand-verified relevant centres, one useful free sample, per-party owner authority, no sequence or follow-up automation |
| Broken production funnel is advertised | **Guarded** | Live probes gate the claim; static PDF remains a useful fallback and QR copy is removable |
| No centre replies | **Accepted** | Four sends are a channel probe, not proof of demand. Bounded non-response is a valid commercial finding |
| A send is duplicated | **Guarded** | Stable per-party idempotency keys and receipt replay |

---
Sprint spec: [`../sprint-05.md`](../sprint-05.md)
