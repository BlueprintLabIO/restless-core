# Launch readiness

This is the release acceptance record for the founder preview. A local demo or an
older test log does not close a check: the final run must identify the published
revision and exercise the installation a new user receives.

| Check | Required evidence | Current state |
| --- | --- | --- |
| Ship the demonstrated product | Reviewed collaboration/settings changes, migrations and assets committed; clean published checkout builds and runs the demonstrated flows | In progress |
| First installation and intelligence | Fresh state and checkout; README commands; successful native Codex, native Claude and API-backed requests; actionable connection failures | In progress |
| First useful outcome | Published starter exercise produces an actual business document, then accepts a human-requested revision | In progress |
| Human multiplayer | Two independent authenticated humans; invitation, shared editing, reconnect, comments, permission boundaries and removal through the supported product path | In progress |
| Completion and recovery | Real model delegation, produced output, review and revision; interrupted execution recovers preserved work and delivers a final owner notification without a prompting message | In progress |
| Founder feedback | Visible, working route from the README to onboarding help and founder feedback | Shipped; links verified |

## Initial findings — 22 September 2026

- Public `dev` and local HEAD both identify `b0890d274483d3c07b7c028874f83874cf3517aa`.
  The working tree contains substantial unpublished collaboration, settings and
  recovery changes, including migrations 58–63. They must be qualified and shipped.
- The development launcher uses the local-owner identity adapter. Independent
  human sign-in must be tested through authenticated network entry, not inferred
  from two browsers sharing that local identity. The existing Sprint 45 report
  proves signature/identity/revocation composition but explicitly leaves the
  full HTTP and user onboarding journey open.
- There are seven open dependency alerts: six in the experimental harness and one
  in the cockpit's `devalue` dependency. Review the affected use and fixes as part
  of qualification.

Temporary companies use `_test` names and isolated state, ports and databases.
The owner's running company and other active development sessions are not test
fixtures. Each run must verify its own cleanup.

## Qualification in progress

- Dependency fixes shipped to `dev` in `eb62b77`: `devalue` 5.9.4, `fast-uri`
  3.1.8, `hono` 4.13.8 and `qs` 6.16.0. Cockpit checks reported zero errors or
  warnings, its production build passed, and the experimental harness passed all
  seven tests. GitHub's alert refresh is still pending. Local audits retain low
  severity findings in other dependencies; no forced major-version changes were made.
- A new-profile PostgreSQL test verified private configuration files, startup,
  persistence after restart, preservation of external configuration and cleanup.
- A real isolated Infisical test verified immediate authenticated writes after
  provisioning, preserved credentials and data after restart, and removal of its
  containers, volumes and network. Provisioning now waits for the HTTP API after
  removing temporary bootstrap configuration and recreating the backend.
- The native Documents service passed all 25 tests with real PostgreSQL and no
  skipped tests. These checks cover the pending source changes, not a published
  release yet.
- The first Rust workspace run found a stale permission expectation: replaying a
  document checkpoint after edit access is revoked must return unavailable. The
  corrected test passed, including replay after access is restored without
  applying the checkpoint again. The subsequent full workspace run passed 609 tests, with 10 ignored tests and no failures; its isolated database was removed. Changes after that run still require their own checks.
- The first source installation stopped at the image's missing `TARGETARCH`
  argument under the legacy Docker builder. A target-architecture fallback and
  reusable tool-installation layers reached the pinned tool downloads in a second
  source build; the build fixes shipped in `79e3bb6`. That run was stopped after discovering its test driver had selected
  offset 29000, outside the development profile range. The driver now selects a
  valid range, and the launcher rejects invalid offsets before building. A complete
  fresh source installation remains unverified. The base image also downloads a
  1.28 GB Godot export bundle, adding substantial cold-start time.
- Codex completed a real native `gpt-6-astra` request using the existing account
  in a disposable container; its copied credentials and container were removed.
  This proves provider admission, not the still-pending Restless delegation journey.
  Codex has an existing connected account. Claude's saved sign-in has expired,
  and no API-provider key is configured. The owner asked to use existing
  connections first and will reconnect later; successful Claude/API requests
  remain open requirements.
- A new empty profile provisioned its own PostgreSQL and Infisical, then opened
  provider setup with `unconfigured/pending` and no inherited connections. The
  live desktop and 390px mobile checks verified native Codex/Claude sign-in
  choices, an explicitly opened API form with no selected provider, and editable
  custom-provider input after clearing it. No credentials were submitted in that
  UI check. The test used existing computer and Documents images, so it is not
  the required clean published-source qualification.
- Nine launcher tests pass, including rejection of invalid profile values before
  provisioning. Frontend checks reported zero errors and warnings; the production
  build passed after the provider-selection fix.
- Runtime startup serialization and the CLI connection deadline shipped in
  `493b913`. A reachable local protocol fixture succeeded; a dropped TCP route
  failed in 3.01 seconds with a connection/firewall diagnostic.
- Startup testing found two races: the launcher could write before crash recovery
  opened admission, and automatic Doctor could create the same computer as the
  explicit startup request. Waiting for admission and serializing startup per
  company let the next live run reach a running computer.
- That run then exposed a host networking problem: Docker bridge traffic reaches
  the existing listener on port 8791, but times out on the isolated listener on
  port 23791. UFW is enabled. The owner was asked to allow only the test model
  and coordination ports from the Docker bridge. No host-network workaround is
  counted as evidence that a normal installation works.
- A real Better Auth foundation test passed sign-up, required email verification,
  independent sessions, invitation email matching, membership and removal against
  isolated PostgreSQL. A subsequent HTTP/WebSocket integration connected those
  accounts to Core: distinct human Actors, rejected handoff replay, private
  documents, explicit sharing, comments, live edits from both people, preserved
  edits after reconnecting, and removal ending both the existing Core session and
  its open document connection. The identity adapter in this check is a test
  driver; a shipped invitation/sign-in interface and the complete browser
  journey remain open.
- That integration exposed a deployment coupling: network authentication also
  selected hosted computers and required Fleet monitoring credentials. The
  working-tree fix adds an explicit local Runtime choice, retains signed network
  authentication and immutable company images, and lets local Documents fetch
  Core's public verification key over loopback while retaining the public token
  issuer. Existing local and hosted defaults pass three focused configuration
  tests. Live startup also rejected a mutable network image, a hosted Runtime
  without network authentication, and an invalid Runtime setting before serving.
  The integration's isolated databases, roles, containers and volumes were removed.
  These changes depend on the pending collaboration implementation and
  have not yet been qualified from a published checkout.
- Setup-help and founder-feedback issue forms, with README links, shipped in
  `7e535f0`. Anonymous navigation reaches GitHub sign-in with the intended form
  URL preserved; no public test issue was submitted.
- Authenticated human invitations still need a complete supported identity-adapter
  journey. Signed fixture assertions and two local-owner browser sessions do not
  close the multiplayer check.

The feedback route is shipped. Component results do not close the other five
checks: their final evidence must name the published revision and include the
complete user journeys.
