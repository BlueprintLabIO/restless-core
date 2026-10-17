# Sprint 93 — explain spend and make usage attributable

Owner request, 17 October: Company needs a best-effort Spend page that records known spend and token
consumption, explicitly labels estimates, and explains sudden usage growth. Existing Sydney Resale
records contain substantial subscription-backed token consumption while its dollar summary says $0.

- [x] Project the existing request ledger with its available token/billing evidence; retain unknown
  and interrupted subscription calls without changing charged-budget enforcement or its durable
  format. Fine-grained pricing and cache-write provenance remains an explicit measurement gap.
- [x] Company → Spend: recorded amounts, separately labelled API-equivalent estimates, tokens,
  employee/model/day breakdowns, paged request evidence, Work links and known payment currencies.
- [x] Verify cache-aware estimates, missing counts, corrected duplicates, restart/read failures,
  company isolation and owner/admin access. Review the populated page at desktop and phone widths.
- [ ] Integrate verified source and install the supported local stack. Hosted rollout remains a
  separate observed completion gate; the previous rollout's company image pull failed on disk space.

Reuse the company request spool, payment intents, shared public model catalogue and owner page/query
primitives. Do not add a parallel charging ledger, an accounting platform, model-generated telemetry
summaries or a second rate table. Historical unknowns remain unknown; estimates use the labelled
current catalogue snapshot and never rewrite historical charges or imply a subscription percentage.

## Follow-up plan requested in the same conversation

1. **Usage coverage and efficiency.** Backfill only attributable durable provider/harness records,
   deduplicating request identities and cumulative snapshots. Show unknown/unobserved sources. Add
   burn-rate and loop warnings, cache/input/output/reasoning visibility and owner-set budgets that
   cover subscription routes too. No new limit is silently enabled. Measure completed useful Work
   per token, redundant supervision/review turns, repeated tool reads, failed retries and context
   size; use deterministic collection and reporting. Extend durable provenance with a versioned,
   rollback-compatible format rather than making older binaries reject the spend spool. Retrieve
   focused context, preserve stable
   cacheable prefixes, reuse durable sessions, and choose effort/model for the actual job. Existing
   lead accountability remains; no repeated model wake merely to acknowledge passing evidence.
2. **One-click site publishing.** Build on the existing publication and isolated-service contracts.
   A site in Library should show Private preview / Published, its exact candidate, domain and known
   cost before Publish. Prepare build, assets, health check and destination automatically. An
   authorised click publishes that candidate, returns a working HTTPS URL, and retains rollback and
   Unpublish. Secrets and the company Runtime stay private. Dynamic sites must declare their build,
   start, storage and environment needs; a file is never falsely treated as a deployable site.
3. **Developer access to hosted review.** First use a dedicated invited developer account and normal
   sign-in against a populated test company. Plan an owner-granted, scoped, expiring support session
   for real company debugging, visibly identifying the developer with a revocable audit trail.
   Start read-only; additional effects require the existing authority. Reuse the Fleet assertion and
   plane membership boundaries. Neon access is for database operations, not session fabrication or
   an owner-impersonation login backdoor. Store any normal existing sign-in in the approved vault.
4. **Finish operational polish.** Recover hosted disk capacity safely, retain running/rollback images
   and company volumes, and report installed versus desired versions. Keep the Installed alignment
   correction outstanding. Make publishing, app activation and schedules expose a useful latest
   outcome and recovery action. Measure navigation latency before claiming a performance gain.

Acceptance for these follow-ups must include a real owner journey, not only an implementation or
passing component checks. No one-click publishing, production support grant, automatic cost import
from unrelated developer chats, burn-rate protection or hosted disk recovery is claimed implemented
by this sprint's Spend page.

Optional developer-usage import should read only consented local usage metadata, not prompts or
company credentials. Keep external chats outside company totals unless a real attribution exists;
linking the same provider request through two sources must count it once. Exact account allowance
percentages remain provider observations, not a conversion from dollar estimates.

Prioritise useful business outcomes over an endless sequence of synthetic infrastructure milestones.
Show the tokens and elapsed time spent producing, coordinating, repairing and reviewing an outcome.
The developer workflow also needs focused file reads, concise tool results, fewer model turns while
waiting for external builds, and no repeated broad verification after a bounded change has passed.

## Verification observed

- Focused reporting checks cover subscription value without a made-up invoice, unknown usage,
  invalid cache counts, externally billed API use and mixed-currency payment states.
- Existing ledger tests and correction/restart cases pass; interrupted subscription requests are
  recorded once without poisoning the charged-dollar ceiling. No charging spool format migration.
- Owner crate check, seven frontend role/access checks, frontend diagnostics/type/UI boundaries and
  production build pass. The new cache is partitioned by company and authenticated identity.
- Spend desktop/phone journeys pass populated/filter/paging, retained refresh failure, empty period
  and unavailable payment source. Serious/critical automated accessibility results are empty.
- The first wider run passed 18/20; its two configured-model scenarios had no provider in the fresh
  fixture. The test stack then used the existing local scripted provider rather than a real model.
- The configured wider run passed 19/20. Its directory-cache reuse check observed two reads during
  the concurrent frontend build; that one journey passed when repeated against the settled server.
  All twenty journeys have passing observations, with the earlier failure evidence retained.
- Final visual calibration uses Beautiful UI, Cult UI and Origin UI Svelte: aligned numeric columns,
  restrained hierarchy, bounded disclosure and the existing page primitives, with no imported UI
  framework or component runtime. Local/hosted installation receipts remain separate from tests.

Sources consulted: [OpenAI usage guidance](https://learn.chatgpt.com/docs/pricing),
[prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching), existing Core
publication contracts and Cloud owner-plane entry/lifecycle contract. API-equivalent prices cannot
be converted reliably into included Codex allowance usage.
