# T13 · Thymelake — simulated restaurant world

**Layer:** All three.
**Serves:** §10.7.3. Thymelake is the strongest whole-company dogfood of the three — product, sales, onboarding, support and external effects must operate as one continuous loop.
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T11 (skeleton), T8 (effects).

## Build

- Company config and directive: acquire a restaurant, configure its menu, launch QR ordering, process orders, resolve issues.
- The simulated restaurant world: prospects, a messy real-shaped menu, pilot approval, test orders, an outage, a refund, a support incident.
- Capabilities exercised: `email.send`, `web.deploy`, `order.receive`.

## Pass bar

A simulated restaurant carried from prospect to a processed test order.

## Acceptance

Scripted run asserts: a menu artifact configured from simulated input; the QR deploy effect receipted; a simulated order processed and acknowledged.

## The real measurement

Same as T12 — **configuration or engineering?** Thymelake is the most likely of the three to demand bespoke work, which makes its answer the most informative.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
