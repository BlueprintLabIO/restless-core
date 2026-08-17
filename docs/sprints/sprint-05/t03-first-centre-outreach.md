# S05-T3 · Browser-reviewed launch gate → first real centre outreach

**Layer:** One end-to-end slice across Owner surface, Authority, OrgIntel and Runtime  
**Serves:** Sprint 05's commercial outcome  
**Depends on:** S04-T4 (prepared change), S05-T1 (live SPA queue), S05-T5 (browser handover),
S05-T6 (imported credential backend)
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

1. The SPA presents the prepared centre-offer compare link as an outcome-review attention item with
   the commit, tests, recommendation, consequence and production gate. The owner reviews it in the
   real browser surface. A scoped company account may use the shared Runtime profile; a personal/root
   GitHub identity uses the direct owner-browser link instead. No forge API or PR lifecycle is added.
2. The owner merges or rejects the change. A merge is recorded through the existing generic effect
   process with arbitrary JSON evidence such as compare URL, observed remote head and merge commit;
   this remains an attestation until the remote repository is independently probed.
3. Restless observes the repository's existing deployment from outside the actor's account. The
   centre page must return 200 with the expected title before any outreach links to it. Closing the
   browser or returning control does not clear this gate.
4. The owner grants or declines first contact for each exact canonical party in the SPA. The four current
   candidates are BrainTree, Global Education Academy, Pre-Uni New College and Matrix Education.
5. Exec re-probes the sample PDF, booklet route and health endpoint through ordinary runtime/browser
   tools. If the interactive QR path remains unhealthy, it removes the marked QR paragraph from the
   prepared emails and sends the useful static PDF offer; it does not claim a broken capability.
6. Each approved email uses the installed Resend CLI through `restless effect --class
   customer-contact.email`, with the PDF declared as an artifact, a unique stable idempotency key and
   a generic receipt. The message is sent only after its exact party approval; Restless owns no email API.
7. Replies enter through the governed inbound path. The company records the first concrete objection
   or buying signal and chooses the next offer change. If none arrive within five business days, it
   records bounded non-response and decides whether to change segment, subject or offer.

**Not in scope:** a deploy adapter, forge API, site-specific browser command, CRM, bulk-email system,
sequence builder, invented lead database, or custom durable workflow engine.

## Acceptance

1. The real SPA item opens the real compare page and production surface. A clean owner hand-back wakes
   the requesting actor against the same browser state, but leaves the item open until its source
   condition changes.
2. Provider/external observation shows `/for-tutoring-centres` live after the merge; browser-close,
   lease-release, a self-attested receipt or an agent statement alone is insufficient.
3. Before the owner acts on the four exact approval items, all drafts remain unsent and the receipt
   query shows zero new live `customer-contact.email` effects.
4. Every recipient is one of the double-verified canonical parties and has an owner approval grant
   before first contact.
5. At least four approved first-contact sends produce distinct `customer-contact.email` receipts; replaying any
   key does not send twice.
6. The actual body sent contains no claim contradicted by the final production probes. A red QR path
   removes the QR paragraph rather than blocking the static-PDF offer.
7. The SPA Attention Inbox plus `restless attention`, `receipts`, `people` and `spend` let the owner
   follow the run without `psql`, raw logs or searching ordinary mail for an outstanding action.
8. A reply/objection is recorded if observed. Otherwise, after five business days, a dated
   non-response finding and the next bounded experiment close the ticket honestly.

## Risks

| Risk | Disposition | Why |
|---|---|---|
| First contact becomes spam | **Guarded** | Four hand-verified relevant centres, one useful free sample, per-party owner authority, no sequence or follow-up automation |
| Broken production funnel is advertised | **Guarded** | Live probes gate the claim; static PDF remains a useful fallback and QR copy is removable |
| Browser hand-back is mistaken for launch success | **Invariant** | The source item resolves only from independent Git/HTTP observation or an authoritative effect result |
| A personal GitHub credential remains in the company profile | **Guarded** | Personal/root identity uses the direct owner-browser path; only scoped company accounts may persist in Runtime |
| No centre replies | **Accepted** | Four sends are a channel probe, not proof of demand. Bounded non-response is a valid commercial finding |
| A send is duplicated | **Guarded** | Stable per-party idempotency keys and receipt replay |

---
Sprint spec: [`../sprint-05.md`](../sprint-05.md)
