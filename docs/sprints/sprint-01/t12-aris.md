# T12 · Aris — simulated sales world

**Layer:** All three.
**Serves:** §10.7.2. Aris tests **selling an existing product** — materially different work from building one, which is the point of running it in the same sprint (§17 step 5).
**Makes deletable:** Nothing yet — first sprint.
**Depends on:** T11 (skeleton), T8 (effects).

## Build

- Company config and directive: sell practice papers, identify the strongest segment/offer/channel, produce evidence for the next commercial decision.
- The simulated sales world behind T8's provider trait: prospects, replies, objections, conversion behaviour. **Model-driven personas, not scripted branches.**
- Capabilities exercised: `email.send`, `payment.charge`, `web.deploy`.

The operating loop (§10.7.2): choose segment → create offer → find prospects → outreach → close → deliver → collect objections → refine.

## Pass bar

A simulated prospect carried from segment choice to a simulated purchase, with the funnel legible.

## Acceptance

Scripted run asserts: an offer artifact exists; the outreach effect is requested and receipted; a simulated reply is handled; the purchase effect is receipted.

## The real measurement

**How much of this was configuration versus engineering?** Record it. That number is the sprint's primary result, not the funnel.

---
Sprint spec: [`../sprint-01-walking-skeleton.md`](../sprint-01-walking-skeleton.md)
