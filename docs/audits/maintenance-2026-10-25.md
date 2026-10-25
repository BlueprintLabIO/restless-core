# Core and Cloud maintenance — 25 October 2026

This pass started from Core `2d425334` and Cloud `dfd2183`, fetched from their
respective `origin/main` branches. Existing primary checkouts and their dirty
files were preserved. Development used a claimed Core pool slot and an isolated
Cloud worktree; no live appliance or production deployment was changed.

## Reproduced and repaired

| Failure | Repair and evidence |
| --- | --- |
| CLI context saving followed a pre-existing `contexts.json.tmp` symlink and overwrote an unrelated file with bearer credentials. | Reuse the appliance atomic writer, create unique temporary files with private permissions from creation, and remove failed temporary writes. Symlink preservation, final 0600 mode and concurrent complete-file replacement pass. Core `8cf48a2e`. |
| Names beginning with `localhost` or `127.` selected plaintext HTTP even when they named a remote host. | Select the default scheme from the parsed host; exact localhost and loopback IPs use HTTP. Remote lookalikes use HTTPS; IPv6 loopback works. Core `8cf48a2e`. |
| CLI decisions succeeded and printed “Nothing needs you” after a 503 containing the text “404”; malformed lists also became empty results. | Preserve the actual HTTP status, reject incomplete lists and fail unavailable reads. Read the company catalog once. Executable CLI scenarios cover failed, malformed, empty and removed-company responses. Core `903d2fb6`. |
| The standalone identity service pinned a vulnerable mail address parser. | Upgrade Nodemailer 10.0.10 to 10.2.1. Sixteen identity tests pass; captured SMTP signup, email verification and sign-in pass against temporary PostgreSQL. Its npm audit has zero findings. Core `dd5e0d04`. |
| Cloud only read the first Hetzner API page, missing later stock and potentially a duplicate plane server. | Follow pagination while preserving filters; propagate later-page failures and refuse ambiguous plane servers. All 52 provider tests pass, including PostgreSQL and the pinned nginx image; GitHub CI passed. [Cloud #140](https://github.com/BlueprintLabIO/restless-cloud/pull/140). |
| SvelteKit resolved cookie 0.6.0 in the cockpit, Fleet Web and public site. | Override only SvelteKit's copy to 0.7.2. Session serialization/parsing and malformed name/path/domain rejection pass in all three installed apps. Core `bdba8fde`; [Cloud #141](https://github.com/BlueprintLabIO/restless-cloud/pull/141). |
| The resource reaper passed regexes through awk string unescaping, widening escaped dots and matching innocent lookalike commands. | Pass the regex through the environment. The exact fixture formerly matched four processes; now only the real `.bin/vite` and `http.server` match, excluding lookalikes and young processes. `bash -n` passes. Core `a5e8bd1b`. |

## Verification scope

- Core CLI: 31 unit tests and two executable CLI scenarios pass. The Docker
  appliance backup round-trip remains an explicitly ignored, unexecuted scenario.
- Core machine helper: 22 tests pass.
- Core cockpit: type and boundary checks, all 262 tests, and both production builds pass.
- Cloud provider: 52 tests pass with no skips; temporary database containers were removed.
- Fleet Web: check, 32 unit tests and production build pass.
- Public site: artifact checks, type checks, 11 waitlist tests, four presence tests,
  production build and all 2,084 internal links pass.
- Cloud release qualification and platform, security, operations and migration
  contract checks passed. These are local/CI checks, not a production rollout.

## Remaining dependency work

Every committed npm lockfile was audited against the registry. Counts below are
package findings, so one underlying advisory can appear through several parents.

| Lockfile | Remaining findings | Next work |
| --- | --- | --- |
| Core `infra/host-tools` | Four high package findings from the single Zod 3 constrained-collection denial-of-service advisory, through the pinned coding agent, Puppeteer and chromium-bidi. | Verify a compatible toolchain upgrade that removes Zod 3. Do not replace the pinned agent without checking its Responses namespace patch and broker behavior. The company's image has a separate, exact-path exception expiring 8 November in `.dagger/src/publish.ts`; this audit does not extend it. |
| Core `web` | Five low package findings: SvelteKit/adapter plus KaTeX through Mermaid/svelte-streamdown. | Migrate the framework and adapters to the patched major together; follow the renderer's compatible upstream fix. No compatible KaTeX fix was offered by npm audit for the current graph. |
| Cloud `apps/fleet-web` | Three low package findings in the framework dependency chain. | Verify the SvelteKit/adapter major migration with authentication and account-entry browser journeys. |
| Cloud `site` | Two low package findings in the framework dependency chain. | Apply the corresponding framework/adapter migration with site verification. |

The other eleven committed npm lockfiles have zero findings, including the
patched standalone identity service. This audit does not assert that Cargo
dependencies, OCI images or all product behavior have been cleared.

The pre-existing Chrome exception expires on 27 October. Google's
[Chrome for Testing stable manifest](https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json),
checked on 25 October, still lists 156.0.8078.47. Recheck for the patched stable
build before that exception expires.

## Resource check limitation

`restless-reap --check` reached Docker's disk-usage query and stalled there;
that read-only invocation was stopped. No purge ran and no resources from other
sessions were removed. Its regex defect was verified separately with the exact
awk program and controlled process rows. No whole-host clean bill is claimed.
