# S61-T1 — Probe the two Cloud suspects

**Layer:** Gateway/Cloud. **Serves:** acceptance 3; the "callback may not route" and "hosted
Runtime reaches the gateway" frictions. Done first because a positive finding changes T3 and T5.

On a Cloud test plane with a `_test` company, behind `app.restless.run`:

1. Start a connection sign-in from the cockpit and read the `redirect_uri` it sends to the provider.
   Request that exact URL with a dummy `code` and `state`. Record which service answered (router,
   Fleet, plane) and the status. Expected if the suspicion holds: not the plane's callback handler.
2. From inside the company Runtime, as the `company` user, request the gateway address the launch
   contract hands an actor (`tool_gateway.rs`, `host.docker.internal:{port}/tools/{company}`).
   Record whether it reaches the plane's gateway.

For each that fails, choose the smallest fix, preferring one that adds no router rule. The likely
fix for (1) is a company-scoped callback path. Land it in Core, or file the paired
`restless-cloud` change. Record both observations, before and after, in this ticket.

**Deletes:** nothing.
