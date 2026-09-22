# Launch readiness

This is the release acceptance record for the founder preview. A local demo or an
older test log does not close a check: the final run must identify the published
revision and exercise the installation a new user receives.

| Check | Required evidence | Current state |
| --- | --- | --- |
| Ship the demonstrated product | Reviewed collaboration/settings changes, migrations and assets committed; clean published checkout builds and runs the demonstrated flows | Accounts, documents, conversation routing, Company settings, provider-first setup, and collaboration owner workflow shipped in `cc933fc`; remaining model and independent-browser requirements stay open |
| First installation and intelligence | Fresh state and checkout; README commands; successful native Codex, native Claude and API-backed requests; actionable connection failures | Clean setup verified at `c88e29d`; Docker-to-Core connection blocked on this host; Claude/API requests deferred by owner |
| First useful outcome | Published starter exercise produces an actual business document, then accepts a human-requested revision | Native document CLI journey passes; model-produced proposal and revision remain open |
| Human multiplayer | Two independent authenticated humans; invitation, shared editing, reconnect, comments, permission boundaries and removal through the supported product path | Independent-account protocol journey passes; simultaneous independent browsers remain open |
| Completion and recovery | Real model delegation, produced output, review and revision; interrupted execution recovers preserved work and delivers a final owner notification without a prompting message | Durable delivery, recipient isolation and restart/replay tests pass at `932f5e4`; real model journey remains open |
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
- The native Documents service shipped in `32a4b28` after all 25 tests passed
  with real PostgreSQL and no skipped tests. It preserves a single initial live
  body, rejects writes from a restored document's old body, and durably applies
  scoped agent edits without replacing concurrent human changes.
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
- The working-tree self-hosted account service now passes the same real
  PostgreSQL/Core/Documents integration using its own authentication and
  invitation implementation. Removal during a stopped Core reports an incomplete
  removal, blocks fresh entry across an account-service restart, and completes
  when retried after Core returns. Cancelled invitations, cross-origin rejection
  and the single-use password-reset email path also pass. Mail was captured
  locally; external SMTP delivery has not been qualified.
- Browser checks of the working-tree account service now reach a usable workspace
  for the owner and an invited member, including a repeat sign-in with an active
  service worker. The Core handoff now lands on the verified company and allows
  navigation to its public page while retaining API and desktop origin checks.
  Public page responses disallow external framing. The daemon build and three
  cross-site boundary tests pass.
- The invited member edited a document, commented on the new paragraph, reloaded
  it and retained both changes. The owner then read and replied to that feedback.
  This exposed and fixed a comment-anchor bug: Core now persists the current live
  body before attaching a paragraph comment. Six real PostgreSQL document tests
  pass, including concurrent comments, replay and removed paragraphs. These
  document changes still depend on unpublished collaboration work.
- Those browser checks used distinct accounts sequentially in one browser
  context. Concurrent edits, reconnection and removal of an open connection
  passed with independent authenticated protocol clients. A later browser run
  verified the member-removal confirmation: Cancel retains the member; Remove
  access ends their existing company session. The owner still sees the removed
  person's feedback. The ended-session screen now explains how to sign in or
  request a new invitation instead of showing only “401 Unauthorized”.
- Core now accepts an optional verified account name and displays it on the
  bound human Actor. Browser comments show “Launch Colleague”; the name and
  feedback survive membership removal. Eleven real database tests and the
  signed-assertion test pass, including duplicate names, replay, stale updates
  and unchanged roles. Frontend checks report zero errors/warnings and its
  production build passes. The self-hosted service that supplies these names
  remains part of the unpublished setup work.
- Simultaneous independent browser contexts and the clean published setup
  journey remain open. External SMTP delivery remains unqualified.
- That browser investigation found a separate cockpit defect: a failed initial
  company-list request kept the page in its loading state and hid the error.
  The fix displays the failure and a retry action. Frontend checks reported zero
  errors/warnings, production builds passed, and the built UI showed the error
  on desktop/mobile and recovered after its test endpoint became available.
- Release provenance now refreshes after commits on the same branch, tracked
  edits, packed refs and linked-worktree changes (`e2d6948`). An isolated real
  Git/Cargo workspace verified those transitions and the explicit exported-source
  override. This fixes the stale embedded revision found during qualification;
  it does not qualify the currently dirty working tree as a release.

- The Core collaboration package was isolated from the other unpublished work
  and passed six real database history/comment tests, 16 document HTTP tests,
  three Runtime configuration tests and three local Documents tests. The latter
  includes an actual Docker start, reuse, recovery and cleanup. The normal daemon
  build passed. Its binary also passed the account-service integration: separate
  verified people, invitations, private document sharing, concurrent edits,
  reconnection, session removal and removal recovery after a Core outage. All
  test databases, roles and containers were removed. The editor and account
  service remain separate unpublished pieces; this is not yet a complete
  published-source multiplayer qualification.
- The editor was then qualified separately against that Core package. Its
  type checks report zero errors or warnings and its production build passes.
  Real browser checks cover editing, a comment on a newly added paragraph,
  automatic history, restoration, mobile actions, requesting review and an
  accepted version appearing in history. Two open editor tabs exposed a restore
  bug: the old editor stopped at “Reconnecting” after a late edit. The Documents
  service now identifies that closure, and the editor keeps the old draft visible
  for copying before loading the restored version. The browser recovery check
  and six server tests pass. This two-tab check used one browser identity; the
  independent-human protocol checks remain separate evidence. Account setup is
  still unpublished, and the complete fresh multiplayer installation remains open.

- The self-hosted account package now has a reproducible integration command.
  Its documented configuration CLI passed against a fresh company, followed by
  verified invitations, independent Core identities, private sharing, concurrent
  document edits, reconnection and access removal, including an account-service
  restart while Core was unavailable. The generator also rejects an accounts
  connection that points to the company database under an equivalent URL.
  A separate test ran the production entrypoint and delivered verification mail
  through Nodemailer to a local SMTP receiver before verified sign-in. All test
  containers, databases, roles and temporary private configuration were removed.
  These are candidate-package checks; the published-revision rerun remains open.
  The setup guide supports new, unused companies: existing local-owner document
  permissions do not automatically transfer to new authenticated identities.

- The clean source-install run at published revision `72d00ad` reached its
  60-minute deadline during the Godot export-template download, before exposing
  the cockpit. Its processes, containers, volumes and temporary checkout were
  removed. This is a failed installation check, not a successful cold start.

- The account package is now published. A clean checkout and daemon built at
  `1a4a4bd` passed its three configuration checks, production SMTP/sign-in test
  and full independent-account protocol journey, including reconnect and removal
  recovery. The runner verified cleanup. This qualifies the published account
  components; it does not substitute for simultaneous independent browsers or a
  complete source installation.
- Investigation of the source-install timeout found a slow Godot download mirror.
  A bounded 1 MiB transfer from the mirror averaged 0.17 MB/s; the official GitHub
  release asset averaged 3.25 MB/s and reported the same pinned SHA-256. `1bbaca0`
  switches the template download to that official asset with visible progress and
  stalled-transfer detection. The complete download passed the existing checksum
  and the company image built successfully; Windows export templates remain
  included. `c11a917` applies the same verified source to both engine architectures.
- The fresh `1bbaca0` install then provisioned PostgreSQL and Infisical but failed
  to start the owner gateway: `web/build` was absent. The launcher installed web
  dependencies but did not build the assets the gateway now serves. It now builds
  workspace assets on reconcile or when missing, before the expensive image build.
  The production web build and all nine launcher configuration checks pass.
  The failed installation's processes, containers, volumes and checkout were
  removed; the next complete published-source installation remains open.

- A fresh checkout at `c88e29d`, with empty company state, now completes the
  README source build and opens setup. Real browser checks passed through both
  Vite and Core's production web server. Setup offers Codex and Claude sign-in,
  has no default model or API provider, and reports secure key storage ready.
  Doctor confirms the running computer, desktop/browser, Documents, OrgIntel and
  web endpoints. It still reports degraded coordination: this host's Docker
  bridge cannot reach the test coordinator. No model was connected in this
  disposable company. Its processes, containers, volumes, database resources
  and checkout were removed and their absence verified.
- Conversation checks exposed a multiplayer defect: a verified human's message
  could receive its reply in the legacy local-owner conversation. `932f5e4`
  routes replies and working history to the exact human, keeps owner completion
  updates separate from member conversations, and refuses late replies after
  removal or an ownership change. Successful work now leaves a durable delivery
  obligation for Exec. A clean checkout of that published revision passed all
  11 real PostgreSQL execution tests, the per-human focus test and daemon
  prompt/final-response tests. These cover
  restart, exact retry, superseded results, separate recipients and access changes;
  a real model delegation and unsolicited final reply remain open.

- Native-document CLI commands now cover creating and reading shared documents,
  guarded live edits, history, sharing and comments. Core retains each prepared
  edit so retrying the same request does not duplicate text. A clean checkout of
  published revision `2ed32cb` built both binaries and its Documents image. Its
  real host CLI run against disposable Core, PostgreSQL and Documents passed creation,
  readback, live editing, exact retry and a comment anchored to the new paragraph.
  Separate tests passed edit revocation, regrant and result replay, plus Runtime
  actor attribution. The checkout, test image tag, containers, volumes, database
  and role resources were removed and their absence verified. This verifies the host CLI
  path; installed-agent invocation and the full proposal/revision exercise still
  depend on resolving the Docker-to-Core connection.

The feedback route is shipped. Component results do not close the other five
checks: their final evidence must name the published revision and include the
complete user journeys.


## Settings and process cleanup — 22 September 2026

- `49221e2` waits up to five seconds for an agent's Linux session to disappear
  before declaring cleanup a failure. Thirteen focused ACP tests passed. A real
  isolated Docker test verified both a departing process and a process that stayed
  alive beyond the deadline; the test container was removed. The published source
  matches the tested candidate exactly. This does not close the live-model
  completion and recovery journey.
- `3490aff` publishes editable company direction, version history, model spend
  limits, and Company settings recovery. The candidate daemon and production UI
  built successfully, frontend checks reported zero errors or warnings, and three
  real PostgreSQL identity tests passed. They cover preserved independent evidence,
  earlier versions, stale drafts and competing first promotions.
- CUA browser checks against a disposable real daemon/database verified identity
  edits, stale-edit rejection, budget save/reload and negative-value validation,
  company rename, charter save, and the unsaved-charter navigation guard. Stopping
  the daemon produced explicit Vault and Doctor failures. Restarting it preserved
  the edits, and Refresh/Retry cleared the connection errors. The identity form's
  internal “invalid Work graph” error prefix was replaced with a direct explanation
  and checked against the rebuilt daemon. All 21 published file blobs match the
  final candidate; final formatting changes do not alter behavior.
- Desktop help placement and Escape dismissal were checked. The browser ignored
  the requested mobile viewport and remained at 1280 × 720, including in a new tab,
  so this run does not add mobile evidence. Both editing tabs shared one local-owner
  identity; they do not close the independent-human browser requirement. The test
  daemon, private state and PostgreSQL resources were removed and verified absent.
- The normal Docker bridge still times out on the isolated model/coordination
  ports. Existing Codex admission remains verified; Claude/API reconnection is
  deferred as requested. No live model journey is inferred from the settings tests.
- Read-only migration inspection found a local prerelease database that applied
  migration 59 with three draft comments. Its schema matches the published SQL,
  but SQLx hashes the comments too. Published migration bytes remain unchanged.
  That known prerelease database needs a verified, exact-checksum metadata repair
  before a future upgrade; it was not changed or restarted during qualification.

## Explicit intelligence setup — 22 September 2026

- `2aa442b` lets a company be created before choosing an intelligence connection.
  New launcher configurations contain no placeholder model, and existing
  `unconfigured/pending` configurations remain readable. Native Codex/Claude
  routes no longer require a direct API model to pass the execution guard.
  Configuring only a worker does not hide Exec's setup requirement.
- The final candidate passed a real PostgreSQL company-creation test, eight
  focused routing/configuration regressions, nine launcher tests, a daemon build,
  and the frontend check and production build. Frontend checks reported zero
  errors or warnings. The merged development tree also passed daemon/CLI checks.
  All 12 published file blobs match the tested candidate exactly.
- A real API request created a disposable company with the model field omitted.
  Browser checks verified Codex and Claude sign-in choices, no selected default,
  Exec's connection prompt, an API form with no preselected provider, custom
  provider input, and unchanged setup after Cancel and reload. The create-company
  button itself was not exercised: its generated names do not carry the `_test`
  suffix required for these disposable runs. The API and browser results are
  recorded separately. No credentials or model requests were submitted.
- The fixture's daemon, state, database and role were removed; the browser tab
  was closed. Claude/API live requests remain deferred at the owner's request.

## Collaboration owner workflow — 22 September 2026

- `cc933fc` publishes the document collaboration owner workflow with migrations
  58 and 60–66. The 69 released blobs match the publication canonical manifest;
  migration bytes were retained exactly and recorded EOF-only normalization did
  not change behavior.
- A manual authenticated-owner smoke on the final candidate verified entry,
  saved document editing after reload, Done clearing Attention to All clear, and
  a room message displaying Live. The disposable smoke fixture remains separate
  from the owner's running company.
- This evidence does not close the real-model journey, Docker bridge/network
  issue, expired Claude sign-in, absent API-provider key, or the requirement for
  simultaneous independent browser contexts.
