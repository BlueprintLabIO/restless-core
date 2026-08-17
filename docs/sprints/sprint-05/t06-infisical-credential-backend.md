# S05-T6 · Import the real credential backend before real outreach

**Layer:** Authority credential plane
**Serves:** the first real provider run without making an ignored plaintext file the durable secret
store
**Depends on:** S05-T2's credential-reference CLI
**Makes deletable:** daemon startup folklore that manually exports `.env`, and the stale claim that
`env:` is the completed credential backend

---

## Observed trigger

Sprint 3 deferred Infisical while the workload was one operator and one provider key. Sprint 5 now
needs Kimi, Resend send, Resend ingress and Git custody. More importantly, the current architecture
and Authority implementation sequence name Infisical as the default imported backend before a mock
adapter becomes a controlled real provider.

The 16 August run misdiagnosed this as missing credentials. The Kimi, Resend and webhook keys were in
the ignored repository `.env`; `restlessd` simply did not load that file. `credential.rs` then quoted
Authority §8.1 as saying to defer a secret manager, but the current §8.1 specifies machine identities.
Both statements were stale.

## Boundary

1. `env:` remains a local bootstrap and migration source, not the durable backend.
2. `infisical:/absolute/path/SECRET_NAME` resolves through a kernel-side adapter. Project,
   environment and Universal Auth machine-identity bootstrap are service configuration, never company
   Runtime configuration.
3. Restless stores only the reference. `restless credential set ... --value @file` or `--value -`
   forwards material to Infisical; raw values are refused on argv.
4. Kimi stays the configured model. An optional `model.inference` credential reference resolves
   host-side. The daemon syncs only providers named by configured companies into OMP's imported auth
   broker/gateway; the ACP process receives a narrow gateway bearer and a credential-free
   `pi-native` route, never the provider key. No other key present in `.env` is imported as fallback.
5. Resend ingress resolves `RESTLESS_RESEND_WEBHOOK_CREDENTIAL`; its local migration default remains
   `env:RESEND_WEBHOOK_SECRET`.
6. Infisical owns source storage and machine authentication; OMP's host broker/gateway applies the
   model credential without exposing it to the Runtime. Restless still owns grants, approvals,
   idempotency, budgets and receipts. The Runtime receives neither provider-root nor Infisical
   credentials.

The Agent Proxy is a separate second slice for ordinary authenticated Runtime traffic such as
read-only GitHub CLI work. A consequential `git push` runs through the generic governed-process
boundary with an effect class chosen for the business consequence; Restless does not own a
`repo.push` command or let the operation move behind an unaccounted proxy request.

The OMP model gateway is not that general API slice. It is the already-installed ACP runtime's
provider-compatible credential boundary and preserves the model usage updates Restless meters.

## Acceptance

1. The daemon loads the ignored checkout `.env` without overriding service-manager environment, and
   a real Kimi wake plus `restless credential check -c aris` prove the existing local bootstrap path.
2. A live HTTP contract probe exchanges Universal Auth for a short-lived token, reads one scoped
   secret and upserts one value. Missing secret is `absent`; malformed reference, auth failure and
   outage are not collapsed into absence.
3. A deliberately malicious backend response containing a sentinel secret cannot put that value in
   the returned error or logs.
4. The owner provisions an Infisical project and scoped Authority service identity. Kimi, Resend and
   webhook material are imported without printing; Aris references and ingress switch to
   `infisical:` and probe `present`.
5. Exact-value scans find none of the imported values in company config, Authority/OrgIntel rows,
   Runtime environment, `/company`, SPA JSON, logs or receipts.
6. With Infisical unavailable, authenticated provider work fails clearly while local files, Git,
   browser inspection and OrgIntel reads continue.
7. The four centre emails remain unsent through all credential validation.

## Evidence checkpoint — 16 August 2026

- AC1–3 passed: `.env` loaded without override; the HTTP contract probe observed Universal
  Auth/read/upsert; absence stayed distinct from invalid; and a reflected sentinel could not enter an
  error.
- AC4 passed against a real loopback-only self-hosted Infisical v0.162.19 instance. Its dedicated
  `Restless Authority` project has `/companies/aris` and `/providers/moonshot` scopes plus a
  project-only Universal Auth identity. Access tokens last one hour. Kimi, Resend send and webhook
  values were imported through stdin, every live reference probes `present`, and the raw provider
  values were removed from the ignored checkout `.env` after migration.
- AC5 passed for all three imported values. An in-memory exact-value sweep found none in the ignored
  `.env`, any company config, Aris OrgIntel/Authority rows, receipts, the container configuration,
  accessible live process environments, container/supervisor or gateway audit logs, live SPA
  Attention JSON, or the complete `/company` volume. The ACP process receives only the gateway bearer.
- Broker persistence cannot create a fallback. Startup disables providers not named by a configured
  company and now also disables superseded credentials for a configured provider, then verifies that
  exactly one matching active row remains. The live gateway exposes Moonshot only.
- AC6 passed during a controlled outage: all three Infisical-backed Aris capabilities became
  `invalid` with an explicit connection-refused cause, while the six-item Attention queue and the
  persistent browser remained readable and `available`. After restart, all three returned to
  `present`.
- A daemon restarted without any raw provider key reached Kimi through the imported reference. Usage
  event 248 names exactly `moonshot/kimi-k3`, 60,763 tokens and `$1.0219914`. The wake subsequently
  terminated `blocked` because Kimi returned its billing-cycle usage-limit 403; this is provider quota
  evidence, not credential failure, and no fallback model was attempted.
- AC7 held throughout: none of the four parties has a grant and the exact centre send-receipt count is
  zero.

---
Sprint spec: [`../sprint-05.md`](../sprint-05.md)
