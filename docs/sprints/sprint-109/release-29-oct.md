# Document stability and production delivery — 29 October 2026

PRs [503](https://github.com/BlueprintLabIO/restless-core/pull/503) and
[504](https://github.com/BlueprintLabIO/restless-core/pull/504) landed on main. The released Core
source is `bec616403890cfb42e0fc5f8a672f6806be9b6e4`; it includes the earlier Sprint 109 batches
and the displayed-review version guard. This is a delivery checkpoint, not full-sprint completion.

## What changed and what ran

- The first signed release attempt on f14994a3 was refused by the artifact scanner on Better Auth
  1.7.5: GHSA-965c-763c-88jm and GHSA-r4xp-prcw-77qf. PR 503 updated the self-hosted identity
  service and lockfile to 1.7.12, matching Cloud. All 16 identity unit tests passed, the production
  dependency audit reported zero vulnerabilities, and the subsequent artifact gate passed.
  No scan exception or threshold was changed.
- PR 504 keeps Saved and the toolbar on the document header row, uses a skeleton during the live
  connection, styles the read-only fallback like the live body, and floats Archive/Back clear of
  the text. See [document-surface.md](document-surface.md) for the observed scenarios and limits.
- The real owner/admin/member journey passed with both changes: concurrent document edits,
  discussion, document management, refused permissions, role changes and live revocation.
  Evidence: `/tmp/s109-combined-roles/journey.log`.
- The delayed-connection, failed-connection, actual Archive/Undo and desktop/phone geometry journey
  passed (`/tmp/s109-docs-still-final/report.json`). All 12 Inbox preview journeys passed
  (`/tmp/s109-docs-previews-final/report.json`). Four document/Work captures and both chat motion
  journeys passed; streaming and settled reply widths were both 770px across 275 samples.

## Installed appliance

`restless-dev promote` activated release `bdf1e7a2a742666f7a0f`, previous `6ad49106b068c10f5b2d`.
A fresh status reported the exact source above, a live binary/service/socket, owner health 200,
ready, no drain, repair or rollback. Connections: 23 passed, zero failed, one separate skills
smoke skipped. Evidence: `/tmp/s109-docs-promote.log` and
`/home/yail/.cache/restless/promote/connections-smoke-bec616403890/evidence.json`.

Read-only installed checks on Aris passed at 1440/390: board default, Inbox All, and an existing
document reaching Saved with one editor header and its status on that row. No document edits or
simulated company actions were performed. Evidence: `/tmp/s109-docs-installed-smoke/report.json`,
`/tmp/s109-docs-installed-document/report.json`, `/tmp/s109-docs-installed-status.json`.

## Signed Core and Cloud

- Core qualification passed: Rust workspace/boundaries, cockpit diagnostics/build/tests/overlays,
  artifact consumer, Documents, Sheets, issuer and release contracts. Its independent connection
  smoke also passed 23/0 with the separate skills smoke skipped.
- Core bundle: `registry.restless.run/blueprintlabio/restless-core-release@sha256:5226bb7485aa48f46a7fdfecdb82ea19e89b81a5b47864903242fd44664a2d41`.
  The release verified bundle, handoff and image signatures from the public registry before
  advancing latest-amd64 and the installer. Log: `/tmp/s109-core-release-bec616.log`.
- Cloud source: `c4f703f3166fec7b1dcce539e65286ed139c4793`. Qualification passed
  (`/tmp/s109-cloud-qualify-final.log`, exit 0). The first wrapper had a trailing Windows newline
  error after qualification completed; the corrected wrapper was rerun successfully. Deployment
  then used --skip-qualify for that same unchanged source.
- Cloud admitted the exact Core bundle, built/scanned all four service images, signed and deployed
  `cloud-b91ab9b1adb76fd488e3d6f8e4b92fee4ddbc4cd`.
  Manifest digest: `sha256:89e14529baeab31839cfd3e6d0e5edd8f998681ddbfdeca9f83ce771975d04f4`.
  All public health checks passed: restless.run, provider.restless.run and relay.restless.run.
  The apex serves the site; app. and www. redirect to the apex. Log: `/tmp/s109-cloud-release.log`.
- The live owner plane advanced from desired/applied 119/119 on source 2b050f5a to 120/120 on
  the exact released Core source. Its public health returned 200, the provider reported healthy
  with no error and a fresh heartbeat, and all six company runtime outcomes reported current.
  Evidence: `/tmp/s109-cloud-before-release.json`, `/tmp/s109-cloud-after-release.json`.

## Remaining verification and cleanup

The built-in production browser still reaches the sign-in page at restless.run/account. The live
plane/source and public service checks above do not establish the private Reno browser journeys.
Those remain open, as do durable Undo (C1), the rest of A4 and the other unchecked sprint items.
No additional checklist items are checked from this narrower delivery evidence.

The document test company, sidecar, host servers, mock, isolated Vault/Postgres containers and
volumes, private database URL file and two broker profiles were removed. The clean temporary Cloud
worktree and its 4GB target were removed after deployment. Rendered deployment secrets were removed
by the release script. The reaper returned to 19 pre-existing debt items; no global purge or
removal of another task's resources was performed (`/tmp/s109-release-reaper.log`).

Resources pricing remains unverified: the source catalogue describes company counts and AI credit,
not storage/CPU/RAM add-ons. The release environment has no Stripe key for a live catalogue read.
The approved illustrative layouts do not supply commercial prices.
