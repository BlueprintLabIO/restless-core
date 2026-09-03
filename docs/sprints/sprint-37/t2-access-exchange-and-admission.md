# S37-T2 — Exchange invitations for bounded admission

**Layer:** Authority/account plane + protocol gateways

## Observed friction served

Sprint 36 invitations prove publication scope, but a reusable bearer placed in a browser URL leaks
through history, referrers and logs, while a raw game token does not prove a useful human-to-native-
client join flow.

## Outcome

Implement one account-plane invitation exchange with two released materialisations:

- an HTTPS exchange producing a short-lived HttpOnly browser session for HTTP/WSS; and
- an HTTPS join exchange producing a short-lived subject-, build- and protocol-bound native ticket for
  authoritative UDP admission.

Gateways and servers validate admission locally from publication-scoped verification material. The
account plane owns grant/revoke/expiry truth but does not proxy application traffic.

## Acceptance

- Valid named invitees complete browser and game admission without a reusable bearer in the visible
  URL, process list or application logs.
- Revoked, expired, tampered, cross-company, wrong-subject, wrong-build and wrong-protocol cases fail.
- Invite-only authority cannot issue a public session.
- New admission observes revocation within a frozen bounded interval; the drain policy for established
  connections is explicit and tested.
- Account-plane restart pauses new exchange honestly while already-admitted bounded sessions follow the
  frozen policy.
- Verification material is publication-scoped and cannot sign another publication.

## Makes deletable

Long-lived query-string bearers, manual token entry and provider-specific authentication at each demo.
