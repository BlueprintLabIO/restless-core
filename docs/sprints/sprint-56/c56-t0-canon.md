# C56-T0 — Canon: accept ADR 0012

**Layer:** docs (cross-plane)

**Friction served:** Two Better Auth issuers had drifted apart. No document said which one was
canonical, or where people are managed.

**Change:**
- Accept ADR 0012 with the founders' resolved questions.
- Link it from ADR 0009's consequences and the cross-layer contract §2.4.
- Rewrite the account-service paragraph in `docs/self-hosted-network-entry.md`.

**Cloud hostname plan:** Company planes live under the account site's registrable domain. Owner
published services go on a separate registrable domain (ADR 0012, resolved question 2). Recording
Cloud's production values belongs to the paired `restless-cloud` sprint (C56-T7).

**Makes deletable:** nothing by itself.
