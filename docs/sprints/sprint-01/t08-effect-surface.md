# T8 · Effect surface + simulated providers

**Layer:** Kernel — external effects are a kernel concern (§3.2), even when ungoverned this sprint.
**Serves:** **Aris and Thymelake *are* external effects** — send email, take payment, deploy a menu, process an order. Without this surface those two companies can only write documents to each other and the sprint learns nothing from them.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T1.

## This is a capability surface, not a gate

`request_effect(capability, args, idempotency_key) -> Receipt`

The interface the governed version will eventually have. **The grant check, capability check and approval gate are simply absent this sprint** — see the risk register in the sprint spec, where that is an *accepted* risk with a stated expiry condition.

## Build

- Provider trait with a `Simulated` impl now and `Http` later. Same company-side logic for both (§10.8).
- Capabilities driven by what the companies actually need: `email.send`, `payment.charge`, `web.deploy`, `order.receive`.
- `Receipt`: id, capability, args digest, outcome, timestamp, provider.
- **Only behavioural simulators this sprint.** The deterministic simulators exist for kernel correctness, and there is no kernel correctness to test yet.
- Simulators are **model-driven prompts, not systems**: *"you are a parent who received this email; reply in character, and object about price roughly a third of the time."*

## Out of scope

Grants, delegation, approvals, standing grants, outcome-unknown reconciliation. The legacy effect broker is the most proven component available, but its value is its governance — we take the interface shape only.

## Acceptance

Aris completes outreach → reply → purchase entirely through this surface, with receipts recorded, and the company code path is identical to what a real provider would take.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
