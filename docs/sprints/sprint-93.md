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
- [x] Integrate verified source and install the supported local stack.
- [x] Observe the hosted release identity and both company runtimes after recovering host disk
  headroom. Installation receipts and the remaining follow-ups are recorded below.

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
4. **Finish operational polish.** Hosted disk recovery and bounded image retention are complete
   (receipts below). Keep installed-versus-desired versions observable and the Installed alignment
   correction outstanding. Make publishing, app activation and schedules expose a useful latest
   outcome and recovery action. Measure navigation latency before claiming a performance gain.

Acceptance for these follow-ups must include a real owner journey, not only an implementation or
passing component checks. No one-click publishing, production support grant, automatic cost import
from unrelated developer chats or burn-rate protection is claimed implemented by this sprint's
Spend page. Disk recovery was a separate Cloud operations fix, verified below.

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

## Installation and recovery receipts — 17 October

- Core product source is `243528f5aa5fdd8be691ac8693c67b25999088b6`. The supported local
  appliance installed release `8f91e9fdddbb2dd62105`; a fresh status check reports ready, owner HTTP
  200, the correct source, and no drain or repair. The populated Sydney Resale Spend page renders
  recorded spend separately from the labelled API-equivalent estimate and exposes request/session
  identities and input/output/cache evidence. The local runtime subsequently exited cleanly at
  01:25 UTC with exit 0 and no OOM; verification preserves its existing Asleep state. A current
  doctor therefore reports that stopped runtime rather than claiming every component is running.
- [Immutable Core release 39010654627](https://github.com/BlueprintLabIO/restless-core/actions/runs/39010654627)
  passed qualification, platform publication, signing and contract verification. The sealed bundle
  is `ghcr.io/blueprintlabio/restless-core-release@sha256:20eb4718fe61eb3199833739bc3dc6bebc03241d5e5660b6441ae7bf98975936`;
  its release manifest digest is `sha256:7a919bbf1f85d5ca5864a958be339681567214c1d71157f748eec5afd12157f0`.
- [Cloud release 39013031451](https://github.com/BlueprintLabIO/restless-cloud/actions/runs/39013031451)
  passed all four image builds/scans, signed release composition and Northflank deployment/smoke
  checks from Cloud `061f5c4e105a53d26d0022c61d4d404e650168c1`. Live owner health reports the
  exact Core product source above. Both companies report current/running at the manifest's runtime
  digest `sha256:47978c7d3b7e9df3c5ea36856ff3e4a11e01c575799f3314464ad7c048f9c1a6`.
- The owner VM previously had 1.9 GiB free (98% used) and 158 images. Authorized image-only cleanup
  removed 137 old versions while preserving container references, six recent component versions
  and all company/database volumes. The previously failing runtime pull succeeded. After the Spend
  rollout, a fresh provider heartbeat reports desired/applied revision 53, healthy, no error and
  54,093,074,432 free bytes. [Cloud PR 90](https://github.com/BlueprintLabIO/restless-cloud/pull/90)
  adds bounded reclamation below 12 GiB, protected desired/current/previous references and a
  24-attempt limit. Its tested agent was checksum-verified and installed on the existing VM, with
  the prior script retained. New VMs also receive it through cloud-init.
- Verification used the installed local UI and hosted release/owner/heartbeat evidence. A signed-in
  visual review of the hosted company is still an owner review step; no developer session was
  fabricated from database access. The planned publishing, support-access, efficiency and alignment
  stories remain planned rather than being marked complete by this operational rollout.

Sources consulted: [OpenAI usage guidance](https://learn.chatgpt.com/docs/pricing),
[prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching), existing Core
publication contracts and Cloud owner-plane entry/lifecycle contract. API-equivalent prices cannot
be converted reliably into included Codex allowance usage.
