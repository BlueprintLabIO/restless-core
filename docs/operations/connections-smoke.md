# Connections smoke

`scripts/connections-smoke` answers one question before a build reaches an owner: can a company
still connect, govern and use tools, and supply itself with plugins and skills? It is Sprint 61's
Tier A. No model runs and no third-party account is used.

## What it runs

- An isolated `restlessd` with its own PostgreSQL container, state root, namespace and port block
  (`scripts/lib/smoke_plane.py`, shared with the skills smoke).
- A real company Runtime from the company image under test.
- The fixture provider in `tools/fixture-mcp-provider`. It is a real MCP server behind the official
  SDK's MCP authorization, so Restless connects it exactly as it connects any service.
- The owner HTTP API for everything an owner does, and the tool gateway reached from inside the
  company computer with a real tool-session capability for everything an actor does.

Each assertion records its input, observation, expectation and what a broken system would show
instead, in `evidence.json`. Every resource is `_test` and is removed, including after a failure;
assertion 20 fails the run if anything is left.

## Where it runs

| When | How | Blocks |
|---|---|---|
| Every push to `main` touching connections, and nightly | `.github/workflows/connections-smoke.yml` (with the skills smoke) | nothing; the failure is visible on the commit |
| Release qualification | a step in `immutable-core-release.yml` | the release |
| `restless-dev promote` | after the image build, on the exact release binaries and image | activation; `--skip-smoke` overrides loudly |

## Run it

```bash
npm --prefix infra/host-tools ci --no-audit --no-fund
scripts/restless-cargo build -p restlessd
RESTLESS_COMPANY_IMAGE=restless-company-image:<tag> scripts/connections-smoke --target core
```

Options: `--only 1,2,3` runs a subset (dependencies are skipped with a reason, never silently),
`--with-skills` adds `scripts/skill-compatibility-smoke` as assertion 17, and
`RESTLESS_UPGRADE_IMAGE=<tag>` makes the restart step replace the Runtime with a second image, which
proves that user-space installs survive an image upgrade.

## Cost

Observed on the WSL development host (20 cores) on 6 October 2026: about 3 minutes for the 20
assertions, most of it Runtime starts and the 45-second wait for a provider's access token to
expire. Building the company image dominates when it is not cached. There is no model spend.

## Reading a failure

Open `evidence.json`, find the failed assertion, and compare `observed` or `error` with
`broken_would_show`. `daemon.log` and `fixture.log` sit beside it. A flaky assertion is a bug in the
product or the smoke; fix it or remove it within the week, never retry it into green.

## Not covered here

The Cloud target (`--target cloud`, S61-T5) needs a signed-in test owner on a Cloud plane. Real
providers (Tier B canaries) and agent journeys (Tier C) are separate work.
