# Collaboration feature closeout

Goal: finish native document and group-chat workflows for human owners and agents, including Attention, mentions, capability discovery and extensive verification. Existing implementation is the substrate; no external document service or parallel editor.

## Required outcomes and evidence

- [x] Actor-authenticated native document discovery, creation, read, safe live editing, comments, versions and sharing through installed harness tools. Verify the real installed CLI/session, permissions, retry identity and concurrent human edits.
- [x] Native documents open as editable shared surfaces from Attention. Requests for collaboration and named-version review carry real document coordinates, resolve correctly and preserve the document identity.
- [x] Document comment mentions wake the named actor with exact context; replies remain on that thread, with recovery, access revocation and duplicate prevention verified.
- [x] Room creation and participant management UI, plus agent discovery/read/post/thread/management commands. Verify human and agent membership enforcement, same-thread replies, mobile and keyboard interaction.
- [x] Capability discovery and Doctor distinguish service health from available collaboration operations. Probe end-to-end workflows in disposable companies; no fabricated company records.
- [x] Multi-client browser verification of concurrent co-editing, reconnect, review, comments and group chat; full targeted Rust, CLI, service and frontend checks.
- [x] Real harness smoke on the deployed stack and teardown of disposable processes/data.
- [x] Recover the owner's preserved positioning draft into a real shared document and surface it in Attention without losing existing text; verify with the owner company only after isolated smoke passes.

## Initial audit (historical)

Initial audit: native Doc APIs/editor and Room APIs/threaded UI exist. Installed `restless document` exposes only request-review. Rooms has no creation/membership UI or general actor CLI. Native Doc mentions persist but have no actor reply workflow. Attention renders runtime outcomes, not the native editor. Doctor reports runtime health without testing these feature paths.

## Work log

Started by inspecting current source and runtime state. Prior uncommitted naming, scrolling and stable Exec-button changes are retained.

### Room workflow implementation

Added native-dialog room creation and member management with retry-stable creation keys, owner-only membership actions and cache refresh. Added a typed, actor-pinned `room-operation` boundary and installed-CLI commands for list/create/members/add/remove/read/thread/send. Reuses OrgIntel's existing membership and message/creation receipts.

Evidence so far: frontend check and production build pass; browser-only verifier covers creation after a transient failure, membership changes, Escape and mobile bounds (`web/scripts/verify-room-management.mjs`). Rust compile check passes. Database-backed room command tests are being run before claiming the harness path verified. The CLI/backend source is not yet deployed to the runtime image.

### Additional deployment prerequisite

Live Docker inventory shows no native Docs collaboration sidecar on this local machine. The service exists at `services/native-documents-collaboration`, and each cell already has a narrowly scoped database credential, but local startup does not launch the service. The existing proxy resolves hosted per-cell DNS names. Must implement local lifecycle/address resolution and Doctor checks as part of full document verification; source-only editor presence is insufficient.

Room command tests now pass (2): nested actor spoof rejection, concurrent-safe creation replay, nonmember denial, member admission, message replay, exact thread replies, owner-only membership and post-removal denial. Scratch schema removed. Browser verifier exits successfully after draining/cancelling its own routes. Logs are in workspace work/room-tools-tests.log and work/room-management-browser.log. Room source still needs installed runtime smoke and broader agent-mention cases.

Built the existing collaboration service image as `restless-native-documents:local` (image 8f8674f1b7ae); no collaboration container has yet been started. Next: wire local sidecar lifecycle and proxy addressing; then expose actor-authenticated native Doc operations with live Yjs editing (do not substitute whole named-version replacement for concurrent edits). Sidecar Hocuspocus offers `openDirectConnection` with a shared Y.Doc and transactional persistence; its codec deliberately allows only the `default` root fragment. Keep CRDT limited to document bodies.

### Local Docs provisioning implementation

Added a local Docker lifecycle and loopback endpoint resolution, independent of the company desktop. Startup/new-company Doctor runs document provisioning alongside Runtime setup; hosted DNS stays unchanged. The service uses its existing narrow 0600 credential mount and Core signer, with per-installation ownership checks and resource limits. Test-company destruction removes its sidecar before cell storage. A Docker/PostgreSQL lifecycle test covers start, readiness, reuse, stopped-container recovery and cleanup; it is being built/run before deployment. No end-to-end document outcome is claimed yet.

Local Docs evidence: `work/local-documents-tests.log` records two binding/ownership tests and the real Docker lifecycle test passing. `work/local-documents-boundaries.log` records five signer tests plus exact proxy-route, readiness-contract and readiness-secret-rotation tests passing. Test containers, databases and roles are absent after teardown. The rebuilt daemon is deployed; real-company startup Doctor reports `documents.status=ready` and `setup_failed=false`, and its loopback service returns protocol/schema 1 readiness. The persistent company Docs container is intentionally left running. Agent editing, Attention and mention outcomes remain unproven and unfinished.

### Agent document command foundation

Added domain-scoped `document-operation` commands and installed CLI verbs for accessible discovery/search, creation from bounded Markdown/editor JSON, explicitly named snapshots/version history, owner-managed sharing, comments, exact-thread replies and resolution. Core pins the authenticated actor; nested caller-attribution fields are rejected. Existing named-version Work review retains its signed Work/Attempt boundary. New-document Markdown uses the existing restricted parser with deterministic block IDs for retries; it never replaces a live body.

Evidence: `work/document-commands-tests-2.log` records 3 document tests (including the real PostgreSQL access/retry/thread scenario), TCP actor-pinning tests and CLI-surface coverage passing. `work/document-cli-tests.log` records 2 CLI parsing/serialization checks. Host binaries built successfully. A Debian-compatible runtime CLI image and separate-plane installed CLI smoke are in progress; live editing, document mention wakes, Attention and the owner's draft recovery are still unfinished.

Installed command smoke passed (`work/document-cli-smoke-3.log`): Debian runtime image → signed actor TCP → a separate real daemon/PostgreSQL plane. Verified create/retry/conflicting-key refusal, private reads, share/discovery, exact comment/reply/retry, revocation, and Room creation/posting. All disposable databases/roles/processes were removed after the run (including failed verifier iterations). This is an installed CLI transport proof, not yet a model-driven harness proof.

Deployed runtime image `729417a666a0` as `restless-company-runtime:local-cpu-compatible` (also `:collaboration-local`), rebuilt/restarted the daemon, reconciled the real company runtime preserving its volume, and observed `restless document list` from inside that runtime succeed with an empty list. No test documents were inserted in the real company. Build/source logs: `work/document-commands-build.log`, `work/document-runtime-image-build-2.log`; runtime reconciliation: `work/document-runtime-reconcile-2.log`. The installed-image smoke script is `work/verification/document-cli-smoke.py`; it creates its own test plane with disabled scheduler, a zero-budget fake model and ephemeral signed actors, then tears it down.

### Guarded live body engine and service boundary

Added body reads with stable block hashes and guarded insert/replace/delete preparation using incremental Yjs changes. A scoped POST collaboration/body endpoint authenticates a fresh Core token, consumes its one-use session, distinguishes read/write access, shares the browser's live Y.Doc, validates the merged schema before mutation, binds application to the loaded named checkpoint, and explicitly awaits database persistence before acknowledging success. Hocuspocus disconnect hooks swallow store errors, so they are not used as proof of successful persistence.

Evidence: work/document-body-service-all-tests.log has 23 passing tests, with the PostgreSQL test skipped there and separately passed in work/document-body-postgres-test.log. The latter verifies the narrow real database capability and preparing an edit before a store restart then applying/persisting/replaying it after restart. The HTTP/browser-provider test covers concurrent typing, fan-out, read-only/wrong-document/replayed token rejection, stale block/checkpoint guards, invalid updates, persistence failure and retry, and headless reload/edit. Type checking passes. Scratch database/role counts are zero after cleanup.

Still required: Core must durably retain each prepared delta under authenticated actor/document/command identity before applying it; no live edit CLI is advertised yet. The service changes are not deployed. Attention, document mention wakes, feature Doctor probes, model-driven smoke and owner draft recovery remain open.

### Core live command integration

Added migration 0059 for company-local prepared edit records, keyed by command UUID and bound to actor/document/exact request. Concurrent prepares atomically select one delta; completion retains the first acknowledgement. Every lookup/completion rechecks edit access. Added `document read` and `document edit --operations-file ... --key ...` source CLI commands and daemon dispatch through the existing local/per-cell document routing and Core token signer. Core retains the selected delta before transmitting apply; retries return completed results or resend that exact delta. A checkpoint conflict is surfaced, never silently re-prepared under the same key.

`work/document-live-core-tests.log` records three passing command tests, including real PostgreSQL preparation races, conflicting-key rejection, private access, retained completion, and the existing document/comment/sharing scenario. Scratch doctools schemas were absent after teardown. The source daemon and CLI paths compile; the new CLI, daemon, migration and sidecar still need installed-stack smoke and coordinated deployment. This is not yet end-to-end completion evidence.

### Installed live editing smoke and deployment

Built the daemon/CLI and sidecar, then ran `work/verification/document-live-cli-smoke.py` through the actual Debian CLI image → signed actor TCP → isolated Core → authenticated sidecar → real PostgreSQL. `work/document-live-cli-smoke-2.log` passed live read/edit, stale-block rejection, named-checkpoint separation, a simulated lost Core acknowledgement followed by a later collaborator edit and exact-delta replay, persistence across sidecar restart, comment-only write denial, read revocation, and the previous document/Room commands. The initial run discovered the test scheduler-disable flag also suppresses automatic setup; that run cleaned up, and the passing run exercised ordinary installation with zero model budget and no provider credentials. Both runs removed their test planes/databases/roles; no disposable containers remain.

Fixed release schema identity from 57 to 59 and ran both release tests (`work/document-live-release-test.log`). Deployed sidecar image `5189f9f7a51b` and runtime image `c00ea15509b8`, with rebuilt daemon and standing live-edit instructions. Runtime reconciliation retained the company volume. Final startup Doctor reports schema 59, current reconciliation, Documents ready and no setup failure; sidecar readiness is successful. `work/document-live-doctor-final.log` reports the local stack live. The preserved positioning draft still exists in the company volume and no test documents were added to that company.

The resource audit found no orphaned test companies/containers or leaked long-running tests; its default socket path does not describe this custom-root daemon, and the script exits at its disk section on this machine, so direct teardown/Doctor evidence remains authoritative. Remaining work includes agent checkpoint/version creation as needed for named review, native Attention surfaces and collaboration requests, document mention wakes, feature-level Doctor probes, multi-client/model-driven workflow smoke, and the owner's draft recovery. No full feature checklist is marked complete.

### Native Attention document surfaces (source verification)

Added separate owner collaboration requests (migration 0060), an actor-pinned `document request-collaboration` CLI command, access-filtered Attention projection and an exact owner resolution endpoint. Attention reuses the native editor/inspector. Named-version review opens the exact requested snapshot and offers a separate live editing view. Finishing collaboration preserves the document and does not accept a version review.

`work/document-attention-core-tests.log` verifies PostgreSQL access, request retry identity, owner-only completion, review separation and revocation. `work/document-attention-check-final.log` and `work/document-attention-release-test.log` pass compilation and schema identity checks. `work/document-attention-web-check-5.log` and `work/document-attention-web-build-4.log` pass frontend validation/build. `work/document-attention-browser-9.log` runs two real editors with a real Hocuspocus server, concurrent fan-out, synced completion, exact requested-version display and desktop/mobile bounds; owner API responses in that browser fixture are mocked, so it does not establish installed Core behavior.

The browser run exposed and fixed a provider reconnect loop: synchronous provider callbacks were accidentally tracked as Svelte effect dependencies. It also exposed a clipped Save version control; editor layout now responds to its container width. Installed-stack Attention smoke is in progress before deployment. Native Attention-linked chat still uses a handoff-only backend admission path and is unfinished. Mentions, agent named checkpoints, feature Doctor probes, model-driven smoke and owner draft recovery remain open.

### Installed Attention smoke and deployment

`work/document-attention-installed-smoke.log` passes the installed Debian CLI → signed actor TCP → real Core/PostgreSQL → owner HTTP Attention path. It verifies refusal without an owner edit grant, exact request retries, document/request coordinates, wrong-document completion denial, idempotent owner completion, live-body preservation, and removal/denial after owner access revocation. The same run repeats guarded live editing, lost-ack replay, sidecar restart, comments/sharing and Room commands. Its teardown confirms all disposable databases/roles and the test plane are gone. This is transport/API evidence; model-driven execution and a browser attached to this exact isolated Core plane remain unverified.

Deployed runtime image `fa103235b24c` with schema 60 and rebuilt daemon/frontend. `work/document-attention-runtime-reconcile.log` confirms replacement with the company volume retained. `work/document-attention-doctor.log` reports live with all six checks available; startup Documents is ready and setup_failed is false. Live Attention and Documents GETs return 200. Existing owner document `c53f7952-3a8a-4ab9-80e7-24e2ad7af34a` (created before this smoke) remains present; no smoke writes targeted the owner company. People sidebar regression checks also pass at 180/240/360 widths and mobile. Source diff whitespace check passes. All test/build/browser tool handles completed; only intended local services remain.

Remaining scope is unchanged: native Attention chat admission, agent named checkpoints, document mention wake/reply/recovery, feature-level Doctor capability probes, broader real browser/installed integration and model-driven smoke, and preserving/recovering the owner's positioning draft. None of these is proven by the successful API smoke above.

### Agent named checkpoint command

Added `document checkpoint DOCUMENT --snapshot-file PATH --reason REASON --key UUID`, consuming the exact saved JSON from `document read`. It reuses Core's named-version transaction, observed checkpoint guard and matching-live-projection check. It does not replace the shared body or resolve a review. Existing named-version judgement authority (human, Exec, active team lead) is retained. Checkpoint retries now recheck current edit/judgement authority before returning their durable receipt, so revocation applies to retries too.

`work/document-checkpoint-core-tests.log` passes the database-backed command suite, including checkpoint replay, conflicting payload, stale version and revoked-editor replay. `work/document-checkpoint-cli-tests.log` passes CLI parsing/serialization and rejects non-live snapshot input or missing retry identity. Scratch command schemas are absent. Host/runtime builds pass (`work/document-checkpoint-installed-build.log`, `work/document-checkpoint-runtime-build.log`); installed CLI/body/history smoke is running before deployment. No broader closeout item is marked complete by these checks.

Checkpoint installed proof: `work/document-checkpoint-installed-smoke.log` passes the actual runtime CLI/actor/Core/sidecar/PostgreSQL path. It rejects an earlier observed body after a collaborator edit, saves the current body, returns the same checkpoint on exact retries, refuses a changed payload under the same key, retains the prior immutable version, reloads the unchanged live body under the new checkpoint ID, and denies a comment-only actor. Attention/comment/sharing/Room regressions also pass. Teardown confirms disposable database/role removal and stopped processes; direct Docker inventory contains only intended local services. This headless recovery check does not yet establish checkpoint behavior while multiple real browsers continue typing.

Deployed runtime `18dfa21a27a6` and rebuilt daemon. `work/document-checkpoint-runtime-reconcile.log` confirms the volume was retained; `work/document-checkpoint-live-help.log` confirms the command is installed in the real company runtime. `work/document-checkpoint-doctor.log` reports live with all six checks available after ordinary automatic startup Doctor. No owner document was modified by checkpoint verification. Remaining work includes document mention wake/reply/recovery, native Attention chat admission, feature-level Doctor probes, real browser/installed multi-client and model-driven proof, and positioning-draft recovery. Full checklist remains open.

### Document mention reply obligations (Core foundation)

Added migration 0061 and document-owned mention obligations created atomically with new comments/replies. Self-mentions and non-agent recipients do not create automatic reply obligations. Each obligation retains exact document/thread/triggering-comment/recipient coordinates. Claiming and resolving reuse the Actor-wide cognitive lease; document and Room focus are mutually exclusive. Replies are plain restricted-schema comments on the original thread, with actor attribution derived from the lease. Reply insertion and obligation completion are one transaction, with exact-content retry identity. Lease tokens are excluded from serialized context.

Common lease renewal checks document access/thread status. Removal or downgrade below comment access cancels pending obligations atomically, including when access is quickly restored. Other unavailable obligations are cancelled during recovery observation. Expired/replaced leases cannot post output; a current replacement can reclaim the same mention. Lock order follows Document -> Actor -> lease -> mention for document paths; the common heartbeat takes the focused Document lock before its Actor lock.

`work/document-mention-core-tests-2.log` passes the database-backed command suite with new assertions for mention enqueue/retry, exact triggering context, Actor lease exclusion, document/Room focus exclusion, lease expiry and replacement, stale release/output rejection, same-thread reply, exact retry/conflict, token omission, and revocation/regrant. Document test schemas are absent afterward. `work/document-mention-check.log` passes source compilation. Existing Room lease regression tests are being run in a dedicated disposable database.

This foundation is not yet wired into Exec/staff wake scheduling or model context and is not deployed. Next integration must cover schedule recovery, staff conversation admission, exact reply persistence for both Exec/staff, capacity protection, and a document-specific ActorContextFocus/return path so old general/Room context cannot substitute for the document thread. It also needs model-driven and installed restart/revocation smoke. Do not describe document mentions as automatically answered yet. Live installation remains schema 60/runtime 18dfa21a27a6.

`work/document-mention-room-regression.log` passes the existing cross-pool/expiry/Work-exclusion lease test and lifecycle-revocation/named-mention recovery test. Their dedicated database was removed in a finally block and absence verified. The fixture now uses `_test` company suffixes. `work/document-mention-release-test.log` verifies source schema 61 matches migrations on disk. No new long-running service or test plane remains; automatic document mention replies remain undeployed pending the runtime/context integration described above.

### Exec/staff document mention routing and context admission

Added one Runtime mention adapter for Room and native Document claims. Exec recovery, staff admission, scheduler scans, capacity protection, prompt assembly and final-answer persistence now recognize document mentions. A native answer returns to its exact comment thread and does not become an owner-conversation message. New document mentions emit a body-free PostgreSQL wake hint; ordinary scans retain recovery responsibility.

Added document-specific Actor context focus, source anchor, return path and checkpoint filtering. A document mention's saved working memory is excluded from general and Room context. Provider-call admission now has an explicit document_mention kind, exact lease/mention checks, budget accounting and abandoned-call reconciliation. Migration 0061 commits the enum extension before 0062 uses it in the admission constraint; source schema is 62. Existing invocation fingerprint shapes are retained for the earlier source kinds.

Evidence: `work/document-mention-runtime-core-tests-4.log` passes the database-backed document/mention suite, now including exact native return coordinates, scoped checkpoint persistence, general-context exclusion, document-specific provider admission and Runtime reply routing. Earlier iterations exposed a missing test source link and a SQL continuation spacing bug; both were corrected before this passing run. `work/document-mention-runtime-regression.log` passes all 3 existing Actor context and 6 model-invocation admission tests in a separate disposable database. `work/document-mention-runtime-unit-tests.log` passes 9 context tests, 8 scheduler tests, 3 conversation tests, the schema identity test and 2 exact execution tests. An initial execution-module filter matched zero tests, so those two were rerun by their actual exact names; the zero-test result is not evidence. All handles completed, the dedicated regression database was removed and document scratch schema count is zero. Diff whitespace check passes.

This integration is still source/test evidence, not installed or model-driven wake proof. It has not been deployed; live remains schema 60/runtime 18dfa21a27a6. Next: build the daemon/runtime at schema 62, prove actual Exec and staff document mention wake/reply/restart/revocation paths in disposable companies (including a real model-driven run), then deploy. Remaining full scope also includes native Attention chat admission, feature Doctor probes, multi-client browser/installed verification and positioning-draft recovery.

### Schema 62 installed regression

Host daemon/CLI builds completed successfully (`work/document-mention-installed-build.log`). Runtime image `81c4981ff39d`, tagged `restless-company-runtime:document-mentions-smoke`, built successfully (`work/document-mention-runtime-image.log`). `work/document-mention-installed-regression.log` passes the installed CLI against an isolated schema-62 daemon, PostgreSQL and Docs sidecar: live guarded edits, lost-ack replay preserving later edits, sidecar restart, named checkpoint/history/retry, real owner Attention request/resolve/access revocation, document comments and Room create/send. The test plane stopped and its disposable databases/roles were verified absent. This verifies installed compatibility, not automatic model-driven mention replies. The image remains a smoke tag; the owner installation has not been reconciled to it. Actual Exec/staff model-driven mention proof and the remaining checklist are still required.

### First native-model mention diagnostic

Created `work/verification/document-mention-model-smoke.py` using a disposable company and a private in-memory copy of the existing Codex login. The first fixture incorrectly called up while automatic Doctor was provisioning; cleanup completed and the fixture now waits for Doctor. The next run exposed a real creation gap: the new cell listener was absent until a manual schedule wake (periodic repair otherwise). `create_local_company` now notifies the scheduler after startup Doctor finishes. Compile check and daemon build pass (`work/document-mention-startup-check.log`, `work/document-mention-startup-build.log`); automatic delivery with this new binary remains unverified.

The diagnostic also exposed a fixture ownership error in parent auth directories, corrected in the script. After that correction and provider cooldown, Codex launched but failed coordination readiness before prompt. `work/document-mention-model-smoke.log` therefore fails honestly: no actual model reply was observed. Its finally cleanup stopped the test plane and verified database/role removal. Do not treat the manually awakened run as automatic mention proof. Next action: rerun corrected fixture with rebuilt daemon, diagnose coordination readiness, and prove exact author/thread replies for Exec and staff. No owner-company document was changed.

### Real Codex document replies verified

`work/document-mention-model-smoke.log` now passes actual Codex OAuth/gpt-6-astra replies for both Exec and a non-lead staff actor (`evidence-reader`). Each comment created through the installed CLI automatically wakes its named actor; the resulting single reply has the exact document thread, triggering parent comment, intended author, and computed answer 43. No manual schedule wake was used in the passing run. Daemon logs show startup Doctor immediately notifying the scheduler and registering the new company cell listener, verifying the creation fix. Cleanup stopped the disposable plane and removed its databases/roles.

Transport scope: the host blocks the disposable Docker bridge-to-host coordinator port (direct TCP probe timed out). The fixture therefore retains the installed image and seeded company volume but recreates only its disposable harness container with host networking and a sleeping entrypoint, avoiding desktop listeners. This is real model/CLI/Core reply proof, but is not proof of default bridge networking or desktop lifecycle. Earlier fixture failures are superseded only within that explicit scope. Installed restart/revocation/dedup recovery, native Attention chat admission, feature Doctor probes, multi-client browser verification, deployment and owner positioning-draft recovery remain outstanding.

### Installed completed-mention restart recovery

`work/document-mention-model-recovery.log` passes a fresh real Codex Exec/staff mention run, followed by daemon termination and restart against the same disposable company database/volume. Exact original comment commands return their original receipts; both thread histories remain identical. An explicit recovery scan creates no extra reply, and direct SQL verifies zero unresolved, uncancelled document mention obligations. This proves completed-mention replay/restart deduplication, not recovery of a model turn interrupted before its reply commits. The host-network fixture limitation above still applies. All test resources were torn down and database/role absence verified. The broader closeout checklist remains open.

### Native Attention conversation admission

Owner actor-conversation API now accepts an exact currently projected native document Attention request for its responsible actor, with no Work ID. It preserves existing runtime actor addressability and ordinary conversation authority; native requests do not acquire Work handoff authority. Existing Work-handoff admission remains separate. `work/document-attention-chat-check.log` and build pass. `work/document-attention-chat-installed.log` passes actual API admission, wrong actor and mixed Work refusal, same message ID on exact retry (created=false), request preservation, and rejection of a new command after resolution, alongside the complete installed document/Room regression. All disposable databases/roles and processes cleaned up. The first test assertion incorrectly expected the created flag to match on replay; corrected before the passing run. Frontend conversation entry/wiring and browser verification remain incomplete, and non-lead authors still use native comment mentions rather than becoming general conversation targets. This backend change is not yet deployed to the owner service.

### Document conversation UI wiring

Native Attention projection now offers a conversation action only for an active Exec/team-lead requester. The document workspace exposes Discuss document, and the existing focused conversation view renders the live native editor beside the chat instead of an unavailable desktop. Other requesters retain native comment mentions. Rust check, Svelte check/type ramp and frontend build pass (`work/document-chat-ui-rust.log`, `work/document-chat-ui-check.log`, `work/document-chat-ui-build.log`). Extended browser fixture passes opening the Exec composer beside the document, retaining live text through navigation, two-browser synchronization, named-review separation and desktop/mobile bounds (`work/document-chat-ui-browser.log`). This uses mocked owner API with real Hocuspocus; actual browser message submission still needs installed verification.

The fixture initially typed before the seed body loaded, so it now waits for both editors to show the seed. It then exposed a real narrow-editor Save clipping issue, fixed with a title row below 420px; the rerun passes. Mobile and desktop screenshots were inspected. Beautiful UI and Cult UI were consulted for calm chat/surface polish; shadcn-svelte Resizable source examples for restrained split-pane controls. Existing Restless primitives were reused, no library imported. These frontend/projection changes have not yet been deployed.

### Installed browser document conversation

`work/document-chat-real-browser.log` passes a real Chromium -> new owner frontend -> isolated current daemon -> PostgreSQL/Docs sidecar run, without mocked API responses. The browser follows Discuss document, sees the recovered live text beside the Exec composer, submits a multipart message carrying the exact native Attention ID, receives a newly created message, and verifies unchanged editor text and the still-pending request. Focused workspace bounds pass at 1440/1024/390 pixels. Existing document/Room API regression checks also pass; cleanup verifies no disposable databases/roles remain. Source script: `work/browser/verify-installed-document-chat.mjs`, enabled by RESTLESS_SMOKE_BROWSER=1 in the installed CLI fixture. This proves browser delivery and preserved request/body; the no-provider fixture does not prove a model-driven chat edit or response (real model mention replies have separate evidence). Owner-service deployment and other checklist items remain open.

### Doctor installed-command visibility

Runtime Doctor now independently invokes installed `restless document --help` and `restless room --help` with five-second bounds. Company Doctor exposes separate command checks and explicitly states that successful help does not verify actor permissions, editing, delivery or model replies. Missing command observations remain unavailable even when service checks pass. This is installation discovery, not a substitute for end-to-end feature probes; those remain outstanding as a product Doctor capability.

`work/collaboration-doctor-check.log` compiles. Five existing company tests pass in `work/collaboration-doctor-tests.log`; the new regression was added after that compilation began, so it was separately compiled/run by exact name and passes in `work/collaboration-doctor-missing-tools-test.log`. The exact two read-only help commands also succeed against the owner runtime (`work/doctor-live-document-help.log`, `work/doctor-live-room-help.log`). New Doctor integration remains undeployed; no synthetic owner data was written.

### Interrupted mention recovery diagnostic

Added `work/verification/document-mention-interrupted-smoke.py`. It observes an accepted unresolved staff mention before and after stopping the daemon, then restarts against the same state. The first run (`work/document-mention-interrupted-smoke.log`) timed out without a reply; read-only SQL showed the old cognitive lease was valid until 09:56:57 UTC while the fixture exhausted its wait and cleaned up at 09:56:44. Thus the run did not reach lease-expiry recovery and cannot establish a recovery defect or success. The fixture wait now covers the five-minute lease plus a full five-minute repair sweep. That corrected wait is not yet executed. Cleanup succeeded; exact disposable database/role absence was checked. Completed-mention restart/dedup proof remains valid separately.

### Accepted unanswered mention recovers after lease expiry

Corrected `work/document-mention-interrupted-smoke.log` passes: actual Exec reply, staff mention observed unresolved before and after daemon stop, restart on the same database/volume, then an actual Codex staff reply on the exact parent/thread with the expected author and answer. No lease rewrite or manual scheduler wake was used to obtain that recovered reply. Recovery waited for the existing five-minute lease and the ordinary sweep. A further completed-state restart/replay/scan retained one reply and zero pending obligations. Cleanup stopped the test plane and verified disposable database/role absence. This demonstrates recovery of an accepted unanswered mention across daemon termination; it does not assert that a provider request had already started when termination occurred. Host-network fixture limitation remains as documented.

Current host daemon/CLI build passes (`work/collaboration-closeout-current-build.log`) and final frontend check/type ramp passes (`work/collaboration-closeout-web-check.log`). Read-only owner-runtime check confirms the preserved positioning draft still exists at its original path and is 12,133 bytes. No draft mutation or deployment occurred in this turn.

### Live deployment and positioning draft recovery

Deployed schema-62 daemon, latest built frontend and runtime image 81c4981ff39d to the owner stack after observing no live cognitive leases or Codex process. Runtime reconciliation retained the company volume. Initial Doctor caught starting services; the completed check is live (`work/collaboration-closeout-live-doctor.log`), including installed document/Room commands.

The actual Exec import first failed on a relative Markdown link, leaving no partial document. The importer now preserves unsupported file-relative link notation as plain text while continuing to reject unsafe schemes; four Markdown regression tests pass (`work/positioning-relative-link-tests.log`). The full original file was separately parsed successfully, the fix rebuilt/deployed, and Exec retried through its real Codex harness.

Recovered document 8079b3e1-1b4b-41f7-8e29-c097ba876193 (Restless positioning — shared draft), Attention request document:collaboration:64383662-cdbc-48bc-a52d-f6d52d5f10bc. `work/positioning-native-recovery-verification.json` proves all 54 content blocks exactly equal a direct import of the preserved source, whose 12,133 bytes are unchanged (SHA256 2224e3fb6ff0d52412dcc5dca6509a56270c81cd1fa89d41e8bd65e84c62a26d). Real browser verification sees the document in Attention, an editable live body, original transfer-copy text and Discuss document. No browser edits were made. The temporary Rust import verifier was removed. This completes the specific draft-recovery requirement; broader feature probes and remaining multi-client/revocation/Room verification still need closeout.

### Real multi-client reconnect/checkpoint diagnostic

Extended the installed browser script with two independent browser contexts, concurrent inserts, disconnect/reconnect, a checkpoint while both editors stay open, and post-checkpoint editing/reload. The first real run exposed offline state permanently leaving the editor read-only when the network returned. DocumentEditor now disconnects/reconnects the existing provider and reauthenticates on the same Y.Doc; pending document state is retained. Svelte/type checks and build pass (`work/document-reconnect-check.log`, `work/document-reconnect-build.log`). The second installed run passes convergence and reconnect, then fails named checkpoint with HTTP409 (live projection mismatch). Do not claim the full multi-client path passed. Both runs cleaned up disposable company data/processes.

Next diagnostic now records the browser checkpoint payload and persisted projection on failure so the mismatch can be identified before changing checkpoint authority. Possible persistence timing vs content normalization is not yet established. The reconnect source fix is not yet copied to the live cockpit.

### Real multi-client checkpoint/reconnect passes and deployed

Captured browser request and persisted projection were structurally identical after the failed save: the failure came from delayed persistence catching up after the checkpoint attempt. Owner checkpoint API now flushes the current live body through the existing authenticated sidecar read on a Core conflict, then retries the exact guarded checkpoint once. It retains access, expected-version and exact-content checks; actual stale content remains a conflict. Prior committed receipts succeed without requiring a sidecar round trip.

`work/document-real-multiclient.log` passes the real current daemon/PostgreSQL/sidecar/installed CLI plus two independent Chromium contexts: simultaneous inserts converge, a disconnected peer catches up, named checkpoint succeeds with both clients open, a later edit propagates, and reload preserves all changes. Native Attention browser chat, permission/retry/resolution and Room regression also pass. Cleanup confirms disposable databases/roles absent. Build passes in `work/document-checkpoint-flush-build.log`; frontend check/build evidence remains `work/document-reconnect-check.log` and `work/document-reconnect-build.log`.

Deployed reconnect frontend and checkpoint-flush daemon after observing no active Codex process. `work/document-reconnect-live-doctor.log` reports live, and `work/document-reconnect-live-browser.log` verifies the real recovered positioning document still opens editable in Attention without modifying it. Full checklist remains open for remaining comment/review/Room/access and feature-probe coverage.

### Installed Room browser and permission verification

`work/room-real-browser.log` passes actual Chromium -> isolated current daemon/PostgreSQL with no mocked owner responses: Room creation, participant add/remove, two independent browser sessions observing a root message and an exact-thread reply, reload persistence, Escape dismissal and mobile dialog/page bounds. The same run exercises installed CLI thread reads/replies, refuses non-owner membership management, refuses reads/posts after removal, and restores access after readmission. Script: `work/browser/verify-installed-rooms.mjs`, enabled by `RESTLESS_SMOKE_ROOMS=1` in `work/verification/document-live-cli-smoke.py`. All test processes, databases and roles were removed by verified cleanup.

Two verifier assumptions were corrected before the passing run: actor display names must come from the selector, and the mobile thread-only view must be closed before opening Room membership. Neither required a product change. Remaining full-goal gaps include real browser document comments/review and access revocation, concurrent model/human editing, and shipped end-to-end Doctor feature probes. Room agent mention/recovery breadth still needs final audit; this passing run alone does not close the full Room requirement.

### Real browser document discussion refresh

The actual two-browser test exposed an open comment thread that never refreshed replies from another browser, even though its thread list refreshed. `documentCommentsQuery` now uses the existing 15-second document query refresh cadence, including background clients. It retains the same guarded reads and permission invalidation.

Svelte/type checks and production build pass (`work/document-comments-refresh-check.log`, `work/document-comments-refresh-build.log`). `work/document-real-comments.log` passes actual thread creation, second-client reply, reply visibility in the original client without reload, resolution visibility in the second client and persistence after reload. The complete installed document/Attention/Room permission regression and disposable teardown also pass. Verifier: `work/browser/verify-installed-document-comments.mjs`, enabled with `RESTLESS_SMOKE_COMMENTS=1`.

Deployed the frontend to the local cockpit. Read-only `work/document-comments-refresh-live.log` confirms the recovered owner document still opens editable with its preserved text. No owner comments or document edits were created by verification. Named-review browser lifecycle, browser access revocation, simultaneous model/human edits and product Doctor end-to-end probes remain open.

### Installed browser named-version review

The real browser review request initially failed because the frontend sent obsolete `work_dependency: null` while the strict owner request schema accepts only version coordinates and summary. Removed that unused argument/property from the frontend helper; Work-bound actor review remains on its separate authenticated path. Svelte/type checks and production build pass (`work/document-review-contract-check.log`, `work/document-review-contract-build.log`).

`work/document-real-review.log` passes real review creation, Attention's immutable requested-version surface, refusal to accept after a visible live edit with time for persistence, preservation of that edit after reload, exact acceptance after the test author removes their test insertion, review disappearance from Attention, and preservation of the independent collaboration request. Full installed regression and disposable teardown pass. Script: `work/browser/verify-installed-document-review.mjs`, enabled with `RESTLESS_SMOKE_REVIEW=1`. One fixture assumption was corrected: review Attention already opens its discussion panel.

A first immediate edit/accept run returned success before the test established that its edit was visible/persisted; the corrected test proves the persisted-body guard but does not close the immediate edit/accept race. That timing case still requires explicit body-preservation verification. Deployed the frontend request-contract fix; `work/document-review-contract-live.log` confirms the owner draft remains editable and unchanged by the readonly verifier.

### Immediate review acceptance exposes pending-edit loss

The no-delay variant (`RESTLESS_SMOKE_REVIEW_RACE=1`) fails twice in `work/document-review-race.log`. The second run captures `work/document-review-race-response.json`: HTTP200, review accepted, and the originating editor still visibly contains `LIVE_EDIT_AFTER_REVIEW_REQUEST` immediately after acceptance. A newly opened independent browser does not receive that text within 30 seconds. This is contrary evidence; the broader document integrity requirement is not complete. Both runs cleaned up their isolated planes/databases/roles.

Source trace: acceptance advances the named checkpoint after comparing the persisted projection. An in-memory edit can still await debounced persistence. `PostgresDocumentStore.store` treats a different returned checkpoint as `DocumentCheckpointChangedError`; `server.ts` then disconnects/unloads the live Y.Doc. This conflates a same-body checkpoint advance with explicit restore/replacement. A correct fix must preserve pending Yjs edits across same-lineage checkpoint advances while fencing stale edits across a replacement. Do not simply swallow checkpoint mismatch or globally merge across restore. Existing database `seeded_from_named_version_id` identifies the lineage; any protocol change must validate that identity atomically, cover replacement/retry semantics and rerun this actual browser race before deployment. No speculative product fix was made in this diagnostic turn.

### Pending edits survive ordinary checkpoint advancement

Fixed the sidecar persistence adapter to track the existing `seeded_from_named_version_id` alongside its checkpoint/revision. Its store query reads that seed through the existing narrow load capability in the same SQL statement/transaction as the store; Core's document locks prevent a replacement between those observations. On a same-seed checkpoint conflict it merges the authoritative Yjs state and retries using the returned checkpoint/revision. A changed seed still raises `DocumentCheckpointChangedError`, so explicit restore/replacement cannot resurrect old edits. No migration or widened database grant was needed.

`work/document-lineage-check.log` passes type checking; `work/document-lineage-tests.log` passes 24 service tests (one PostgreSQL test separately run). `work/document-lineage-postgres.log` passes actual least-privilege database persistence across a checkpoint advance, then rejects the old writer after a replacement has itself been persisted by another writer. That test compares content and Yjs clocks, since merged equivalent states can have different garbage-collected binary encodings.

`work/document-review-race-fixed.log` passes the previously failing immediate browser race with HTTP200: the accepted immutable content excludes the later insertion, while the insertion appears in a new client and survives original-client reload. Full installed document/Attention/Room regression and disposable cleanup pass.

Deployed Docs image `8c8d96c83ce3` as local/collaboration-smoke. The initial deployment inspection revealed CLI `restless doctor` neither reconciles Docs nor includes its health in the reported live result: it reported live with Docs stopped. Restarted the daemon to invoke actual startup provisioning; confirmed the new Docs image running and automatic startup Doctor completion. `work/document-lineage-live-browser.log` verifies the owner draft opens editable without mutations. This CLI Doctor gap belongs to the still-open feature-probe requirement; do not cite its initial live result as Docs health evidence. Browser access revocation, concurrent model/human editing, broader Room mention audit and end-to-end product Doctor probes remain open.

### CLI Doctor now observes Docs health

Daemon Doctor adds a fresh native Docs readiness observation using the existing bounded proxy contract (local endpoint or hosted cell DNS). It reports only generic unavailability to callers, keeps service health distinct from workflow proof, and performs no provisioning/content writes. Host and Runtime CLI Doctor require this observation before reporting live; missing/failed Docs health produces degraded status and a Documents-specific action.

`work/doctor-documents-unit.log` passes missing/unavailable health regression. Host build passes (`work/doctor-documents-build.log`). `work/doctor-documents-installed.log` and `work/doctor-documents-installed-image.log` pass actual isolated daemon plus host and Debian-image CLI checks with the Docs container stopped: unavailable Docs, degraded exit/report and action; restarting observes recovery. Both runs also pass installed document/Attention/Room regression and cleanup. Runtime image build: `work/doctor-documents-runtime-build.log`.

Deployed host daemon/CLI and updated runtime image, reconciled the company runtime while preserving its volume (`work/doctor-documents-runtime-reconcile.log`). `work/doctor-documents-live.log` now reports live with Docs available and current image reconciliation. Readonly browser verification confirms the owner document remains editable (`work/doctor-documents-live-browser.log`). This closes the false-live service-health bug; shipped disposable end-to-end workflow probes remain outstanding, as do browser revocation and simultaneous model/human editing verification.

### Browser document access revocation

`work/document-browser-revocation.log` passes two real independent browser contexts with an installed actor CLI revoking their shared owner access. The connected client removes document content/editability; the disconnected client does the same after reconnect; both receive HTTP404 for document reads and remain without the body/editor after reload. No mocked permissions or responses. The scenario uses the normal Docs route, so it verifies document-cache/editor access handling independently of an Attention item disappearing.

Source verifier: `work/browser/verify-installed-document-revocation.mjs`, enabled with `RESTLESS_SMOKE_REVOCATION=1` in the installed fixture. Full installed document/Attention/Room regression and cleanup pass, and a final database/role inventory confirms no doccli test resources remain. This proves browser access loss/reconnect handling within the normal refresh window, not instantaneous revocation of every already-issued WebSocket capability. Simultaneous model/human editing, the broader Room mention audit, and shipped end-to-end Doctor workflow probes remain open.

### Concurrent real model/browser diagnostic

Added `work/verification/document-model-human-smoke.py` and `work/browser/verify-model-human-edit.mjs`: actual native Codex mention-driven actor edit on one paragraph while the browser appends human markers to another, followed by third-client/reload and installed-CLI durable-body checks. Uses only an isolated `_test` company and the existing host-network harness fixture limitation.

First run did not reach the agent edit. The actual invocation was admitted and session-ready for gpt-6-astra, then settled failed with runtime_error/transport and zero usage snapshots; no document reply appeared. Evidence captured in `work/document-model-human-failure.json`; `work/document-model-human.log` records cleanup. Stopped the test after authoritative failed settlement rather than treating an observation timeout as completion; disposable database/role cleanup passed. Provider failure text was not preserved in that settlement (only its hash), and the later activity endpoint had no live session, so underlying provider cause remains unproven. The verifier now detects a failed settlement promptly. Do not mark concurrent model/human editing complete; capture live failure diagnostics before another model attempt or pursue other remaining work meanwhile.

### Actual model/human shared editing passes

Captured `work/document-model-human-outcome.json` clarified the earlier apparent EDIT_DONE reply: the agent explicitly said it could not report EDIT_DONE because the generic accountable-lead coordination boundary prohibited editing. Removed the verifier's substring-success assumption; the live body remains decisive. The original isolated transport failure remains unattributed, but subsequent actual provider turns reached the prompt and exposed this reproducible policy mismatch.

Non-lead Staff handling a native-document mention now receive an exact-document collaboration boundary and task context. They may perform an explicitly requested bounded live edit using current block/hash guards and Core's current access check, verify it, and reply on the exact thread. No named-version acceptance, sharing changes, repository edits, broader production or external effects are authorized. Accountable leads, Room mentions and ordinary coordination retain the existing production boundary. Two exact Rust boundary tests pass (`work/document-collaboration-boundary-tests.log`); an earlier empty test filter was corrected and is not proof. Daemon build passes.

`work/document-model-human.log` passes the actual installed Codex harness edit concurrently with 52 browser edits on another paragraph. All human markers and the agent's paragraph survive a third client, original-client reload and installed-CLI durable-body read. Exact actor/thread reply and disposable cleanup pass. This completes the first checklist item's installed tools plus concurrent human-edit requirement, in combination with its earlier permission/retry/version/sharing evidence. The host-network model fixture caveat remains.

Deployed the daemon prompt change. A brief restart lock collision recovered through the existing service manager; final `work/document-collaboration-boundary-live-doctor.log` reports live/Docs available and `work/document-collaboration-boundary-live-browser.log` confirms the owner draft remains editable without mutations. Remaining full-goal work includes shipped end-to-end Doctor workflow probes, broader Room mention/recovery audit and final requirement-by-requirement verification.

### Shipped collaboration Doctor workflow probe

Added owner-only `restless doctor --collaboration -c COMPANY` for local Linux installations. It provisions an independent zero-budget `_test` cell and Docs sidecar, invokes the installed runtime CLI with actor capabilities delivered over stdin, verifies document create/edit/checkpoint retry identity, private/comment/revoked permissions, comment replies and Room thread/membership enforcement, then destroys its resources. Negative checks require the expected permission error, not merely a failing process. Cleanup reports separately and verifies the CLI container is absent. Hosted/browser/model behavior is explicitly outside this command's report.

`work/collaboration-doctor-final-installed.log` passes the final shipped probe plus the complete installed document/Attention/Room regression and verifies test-plane/database/role cleanup. `work/collaboration-doctor-workflow-report.json` contains three verified checks and complete cleanup. `work/collaboration-doctor-wire-verified.log` verifies owner-only admission and rejection of arbitrary probe payloads. The first smoke caught the unsupported `-` input path; the final probe uses `/dev/stdin` through the existing CLI file contract.

Daemon deployed after `work/collaboration-doctor-final-build.log`; `work/collaboration-doctor-deployed-health.log` reports live with no recovery actions. `work/collaboration-doctor-deployed-browser.log` verifies the owner's recovered draft remains editable and unchanged. Resource check ran while the fixture was still active, so its idle test-container observation was not leakage; the fixture subsequently exited successfully and exact-container absence was checked. The reaper's default-root socket observation does not describe the running custom-root service. Broader Room mention/recovery audit and final full-scope coverage remain open.

### Final Room and build regression pass

`work/room-core-closeout.log` runs the complete current `restless-orgintel` Room suite against a new isolated PostgreSQL database: 29 tests pass, including exact structured mentions/retries, cross-pool cognitive lease reclaim, stale holder fencing, participant/actor lifecycle revocation, owner retirement cancellation, pagination, event replay and concurrent writes. The database was removed and its absence verified. This is current Core recovery evidence, not a model-transport simulation.

`work/collaboration-final-web-check.log` reports zero Svelte errors/warnings and a passing typography check. `work/collaboration-final-service-check.log` passes TypeScript; `work/collaboration-final-service-tests.log` reports 24 passing tests and the intentionally separate PostgreSQL test skipped. The real narrow-capability PostgreSQL adapter was separately verified in `work/document-lineage-postgres.log`, including checkpoint lineage behavior. The real Room Exec/lead harness/restart verifier is `work/verification/room-mention-model-smoke.py`; its run is still pending and is not yet completion evidence.

### Requirement audit — completed document and Doctor paths

Attention is complete against `work/document-real-review.log`, `work/document-review-race-fixed.log` and the final installed Doctor regression: real editable identity, independent invitation/review resolution, immutable accepted version, pending local edits surviving new-client/reload and revoked access refusal. The browser review verifier asserts exact version identity and that accepting a review leaves the collaboration invitation open.

Document mentions are complete against the installed Exec/staff exact-thread model replies, accepted-unresolved daemon interruption recovery (`work/document-mention-interrupted-smoke.log`), durable cancellation/stale lease Core assertions, command replay and completed-reply deduplication. The current model/human proof preserves 52 concurrent human edits alongside the agent's actual guarded body edit. No claim is made that arbitrary provider requests survive a process kill in place.

Doctor is complete for this local installation: startup provisions Docs, normal Doctor independently observes readiness and installed commands, and the new owner-only workflow command tests mutations in a separate cell with verified teardown. Browser rendering and model replies are explicitly excluded from a Doctor pass rather than inferred from command help.

`work/document-core-closeout.log` adds 23 current PostgreSQL-backed tests across actor context, company document semantics, documents, model admission and live checkpoint storage; all pass and the isolated database is removed. Room model verification found an unresolved interaction: `work/room-mention-model-smoke.log` proves Exec delivery but times out for a newly commissioned lead. The lead's separate generated commissioning message started Work, while its Room mention remained unresolved. Do not mark Room/harness closeout complete from Core tests alone. A separately prepared onboarded-lead fixture is being tested to isolate delivery.

`work/room-mention-ready-model-smoke.log` now passes real native Codex Exec and accountable-lead Room replies, exact thread/actor assertions, completed-message replay after daemon restart, a recovery scan without duplicates, zero unresolved obligations, and full teardown. Fixture setup acknowledges only the auto-generated commissioning message before model credentials are installed, so it tests an already-onboarded lead. It does not resolve or hide the separate fresh-commissioning interaction above. Both model test companies were destroyed; exact container names are absent. The full goal remains open until the combined-flow failure is understood and the final audit is complete.

### Commissioning overlap root-cause diagnosis

The reproduced case (`work/room-mention-commissioning-before.log` and `...-before-state.json`) did not lose its Room mention. The lead's commissioning invocation settled as a transport failure with zero usage snapshots. Querying the exact commissioning message's activity (`work/room-mention-commissioning-message-activity.log`) captured the cause: `agent session 222 retained 1 processes after cleanup`. The turn had already created attributable Work, but immediate process cleanup verification discarded its outcome and put the actor into failure cooldown. No owner-company records were involved.

Source fixes are under verification: exact-session cleanup now waits up to five seconds for observed disappearance after SIGKILL (still rejects live residue/unresponsive observation), and successful conversation completion notifies the scheduler only after releasing the durable lease and in-process actor slot. This drains inputs whose original wake arrived while the actor was busy; unusable turns retain failure backoff. The disposable host-network harness fixture now uses Docker init, matching the installed runtime's orphan-reaping behavior. The original combined-flow scenario will be rerun; these changes are not yet deployed or a completion claim.

The first cleanup-fixed rerun (`work/room-mention-commissioning-fixed.log`) delivered the queued Room reply, but the exact commissioning activity exposed another failure: `the cognitive-session inputs are no longer all owed to this Actor`. The model's ordinary `restless inbox` call had consumed its own captured commissioning input before atomic conversation finalization. Core now leaves ordinary conversation inbox reads unacknowledged; only finalization consumes those inputs. Live productive Work Attempts retain their feedback-delivery semantics. A new database regression verifies repeated inspection, exact reply replay and preservation of later input; two existing commissioning fixtures now use real cognitive finalization instead of implicitly acknowledging through inspection. The stronger final model verifier also requires a persisted commissioning owner reply, not only eventual Room delivery.

### Final completion audit

All required outcomes above are verified on the local installation. The historical entries retain failed iterations and the fixes they motivated; this section records the final state.

| Requirement | Authoritative evidence |
| --- | --- |
| Installed actor-authenticated Docs and concurrent editing | `work/collaboration-doctor-final-installed.log` covers installed CLI, Core, sidecar and PostgreSQL retries/permissions/checkpoints; `work/document-model-human.log` proves actual Codex guarded editing alongside 52 human edits, new client and reload. |
| Editable Attention, distinct collaboration/review lifecycles | `work/document-real-multiclient.log`, `work/document-real-review.log` and `work/document-review-race-fixed.log` prove identity, exact immutable acceptance, independent invitation resolution and preservation of pending edits. |
| Document mention delivery/recovery/access | `work/document-mention-interrupted-smoke.log` proves Exec/staff exact-thread replies, accepted-unresolved restart recovery and completed-reply deduplication. Core permission/lease tests and `work/document-browser-revocation.log` prove revocation boundaries and browser detachment/reconnect refusal. |
| Rooms UI, installed agent operations and mention replies | `work/room-real-browser.log` covers real two-browser threads, membership, reload, mobile and Escape; installed CLI regression covers permission/retry boundaries. `work/room-mention-commissioning-final.log` proves both a new lead's atomic commissioning reply and its queued exact-thread reply, plus Exec, restart and no duplicates. |
| Capability discovery and Doctor | `work/doctor-documents-installed-image.log` verifies actual service-down degradation and recovery; `work/collaboration-doctor-final-installed.log` and its JSON report verify the separate disposable workflow probe and cleanup. Owner-only decoding/authorization passes in `work/collaboration-doctor-wire-verified.log`. |
| Final targeted checks | `work/inbox-closeout-regression-fixed.log`: 50 database tests pass across Rooms, actors/teams and Work feedback. `work/document-core-closeout.log`: 23 document/context/admission/checkpoint tests pass. Frontend check has zero errors/warnings; Docs service has 24 passing tests with the real PostgreSQL adapter separately passing in `work/document-lineage-postgres.log`. `work/session-reap-settle-test.log` verifies delayed exit and rejection of live residue in real Docker. |
| Deployment and preserved owner draft | Final daemon build is `work/inbox-finalization-build.log`; deployed service is active and ordinary Doctor reports live/no actions in `work/collaboration-closeout-live-health.log`. `work/collaboration-closeout-live-browser.log` verifies the editable recovered draft and original text without modifying it. |
| Teardown | Final model and database fixtures report verified removal of their databases/roles/processes. Docker inventory has no `_test` containers. The resource reaper reports no orphaned configs, exited/idle/runaway test containers or leaked build processes; its default-root socket and macOS disk-path assumptions are not evidence about this custom-root Linux service. |

The final combined model run returned Exec's reply in 9 seconds and the lead's queued Room reply in 75.2 seconds, including the separate commissioning turn. The owner-facing commissioning reply was also durably recorded. Two root fixes prevent false failure or dropped acknowledgement: bounded exact-session exit verification, and inspection-only ordinary agent inbox reads with acknowledgement at atomic finalization. Successful conversation completion wakes the scheduler after releasing both actor guards. Work-attempt self-read feedback delivery remains covered and unchanged.

Verification boundaries remain explicit: model fixtures use the installed runtime image and current daemon with disposable host-network containers and init, because the host firewall blocks arbitrary bridge test ports. Doctor workflow probes are local-Linux-only and certify the listed CLI/Core/service operations; browser and model behavior have their own independent evidence above. No new hosted deployment certification or instantaneous revocation of every already-issued WebSocket capability is claimed. The local owner company and shared draft remain intact.

## Follow-up: direct document edits and simpler controls (22 September 2026)

An owner edit exposed a gap in the closeout: the installed CLI accepted guarded edits but its help
did not describe the operation payload. Exec delegated the edit, and Staff blocked on that missing
contract. Earlier successful tool-level and explicitly prompted model checks did not establish that
an ordinary owner request could discover the format and complete directly.

The installed `document edit --help` now includes insert/replace/delete examples, block/hash rules,
atomic guard behavior, exact retry identity, and the distinction between persisted live edits and
optional checkpoints. Exec's standing and immediate wake instructions explicitly permit bounded
owner-requested edits to an existing shared document in the current conversation. Broader production
assignments still use accountable leads and Staff.

The editor now has one title and sync indicator, Discuss and comments access, and a native overflow
popover for checkpoint creation, opening Documents and finishing collaboration. Type selection is
removed from both editing and creation; existing classification metadata remains compatible. Title
changes autosave for the document owner, using the current metadata revision without creating a
checkpoint. Other editors retain body editing without being offered an unauthorized metadata write.
The exact requested version remains distinct from the live document during review.

Validation evidence for this follow-up is in `work/document-simplification-*`,
`work/document-installed-help.log`, `work/document-direct-context-tests.log`, and
`work/document-direct-edit*` in the local workspace. The direct-edit scenario sends an ordinary
owner message through Attention, exercises the installed Codex harness, checks zero delegated Work,
verifies replacement and insertion before the final reply, and reloads concurrent human edits.
All scratch documents and model calls are confined to a disposable company.

Final follow-up results: Svelte/type check reports zero errors/warnings; production build and nine
context tests pass. The UI regression preserves two-browser editing and exact review snapshots.
The installed real-Exec scenario passes with zero delegated Work and **22 concurrent human edits**
preserved in a new client and after reload. A human-owned document's title autosaves across reload
without a checkpoint. Test databases, roles, runtime and Docs containers were torn down. The local
runtime was reconciled to the updated CLI image, the daemon restarted with the new instructions,
and ordinary Doctor reports `live` with no actions. The earlier blocked owner Work was not replayed,
and the owner's real document body was not changed during verification.


## Follow-up: proactive Exec result delivery (22 September 2026)

Successful Attempt settlement now commits a delivery obligation through the existing terminal
outbox. It routes to Exec; exceptions retain accountable-lead routing, and formal review handoffs
retain their own path. Output references accompany completion. Superseded completions are skipped.
Exec inspects the result, posts useful owner updates without a fresh owner message, and explicitly
stays quiet for redundant/internal observations. Final reply and exact input acknowledgement commit
atomically under the existing cognitive lease. Empty/interrupted output cannot consume a result;
quiet responses cannot consume owner questions or focused collaboration mentions.

Removed adjacent-agent-message merging from Exec, People and focused conversations: a proactive
result now retains its own message identity, timestamp and intent rather than inheriting an earlier
commissioning acknowledgement's stale next-step text.

Validation: six exact-execution Postgres scenarios passed, including concurrent flush, reconnect
before dispatch, interrupted lease, final-receipt replay, no owner input, and quiet acknowledgement;
scheduler and context suites passed. Web checks returned zero errors/warnings and the production
build passed. The local daemon and cockpit were deployed; Doctor reports live with no actions.

Live recovery: Work `d85ae896-fb59-4fe8-94dc-67f4a1ad3518` had finished before the fix with no
completion notice. Only that exact current successful Attempt was marked owed while the daemon was
stopped. Startup produced notice 71; real Codex Exec inspected the completed review and saved reply
72 at 2026-09-22T01:12:24Z without any owner message. A second daemon restart left one reply and a
consumed notice. The browser first exposed unwanted adjacent-message merging; after removing it,
replaying the real result as a delayed read response verified a separate message appeared through
normal polling without input or refresh. Desktop/mobile display checks passed. The document body
and review artifact were not modified by verification.

Evidence: `work/exec-callback-tests.log`, `work/exec-callback-final-tests.log`,
`work/exec-callback-web-check.log`, `work/exec-callback-display.log`,
`work/exec-callback-live-reply.json`, `work/exec-callback-doctor.log`, and
`work/browser/exec-proactive-result.png` / `exec-proactive-mobile.png` in the local workspace.

### Automatic document history and dismissible inspector

Replaced the browser's manual checkpoint workflow with Core-owned version capture. A ten-second
maintenance tick snapshots changed, committed live bodies after ten idle seconds, or once the prior
version is five minutes old during sustained edits. Document/live-row locks and matching-projection
checkpoint advancement preserve the existing Yjs seed and make repeated/concurrent ticks no-ops
when content is unchanged. The worker is independent of the company computer. Generic review
requests capture the current body; Work-bound reviews retain their explicitly pinned snapshot.
Restore preserves an unsnapshotted current draft as a `Before restore` version in the same transaction.

The inspector exposes Comments and automatic History, showing Review only for pending review work
or an explicit review action. Dismiss closes the tray without resolving the review or collaboration;
Attention query refreshes do not reopen it. Documents reclaims the closed inspector's space at wide
sizes and has a header button to reopen it. Routine checkpoint controls are removed; live sync state,
rather than difference from the last history entry, guards review/restore actions. Metadata edits
retry a history-only revision advance without asking the author to resolve a false title conflict.

The existing Bridge controls remain the visual source. Beautiful UI's compact contextual surfaces,
Cult UI's explicit expand/collapse controls, and Origin UI Svelte's conventional buttons were reviewed;
no external component code or new visual language was imported. Final desktop/mobile inspection
reduced extra tab-header space and confirmed Dismiss fits without horizontal overflow.

Verification found a live-codec mismatch in the actual owner draft: ProseMirror emits optional link
`title: null` and code-block `language: null`, while Core's validator had rejected both. Core now
accepts these explicit absent values, retains the exact committed projection (and therefore Yjs
lineage), and still rejects non-string non-null values. This also fixes checkpoint validation for
rich live documents. Regression coverage exercises automatic snapshots containing both attributes,
plus HTML/Markdown rendering and rejection of invalid attribute types.

Source checks: 17 real-PostgreSQL document/history/review tests and 11 document validation/rendering
tests pass. Frontend checks report zero errors/warnings, the production build passes, and read-only
inspection of the owner draft passes at desktop/mobile widths. The isolated installed-stack browser
proof covers two live editors, automatic history and continued typing/reload, History/Dismiss at
1600/1100/390 pixels, review capture without a checkpoint form, and Dismiss remaining closed after
a real polling interval. Verification-only scripts use disposable companies and remove their
containers, databases and roles; the owner's draft is never a test edit target.

Concurrent first-open verification also exposed a CRDT seed race: separate loads could regenerate
independent Yjs identities for the same initial version, then merge duplicate blocks. The sidecar
now commits the initial seed with the existing compare-and-swap capability before returning any
bytes to clients, and reads back the winning durable seed. A real PostgreSQL test races two stores
and requires identical returned state; all 25 sidecar tests pass with the database test enabled.

People chat headers now expose the existing intelligence popover for the selected actor, and the
team quality target is an owner-controlled select. The target affects future team coordination and
new Work; it is separate from the model's thinking effort. Direct changes are atomic, attributed
in `team_standard_changed`, retry-safe, and reject stale competing updates. Migration 63 permits
this explicit owner setting without manufacturing a commissioning message. The redundant
`commissioned` phase is no longer rendered in the header. Exec recovery no longer ends the whole
company scan, allowing addressed team conversations to proceed under their own actor leases.
The reported Daria message (92) was eventually answered by message 93 before deployment; the
observed delay was almost seven minutes. No duplicate owner message was submitted for testing.

Final installed verification passes for rich documents (links/code blocks), two concurrent editors,
automatic history, reload, and review dismissal at desktop/tablet/mobile widths. The isolated People
fixture changes its target to Fast and reloads with the saved value. Live read-only verification of
Daria's header confirms ChatGPT / Codex, gpt-6-astra, medium thinking effort, hover/focus/Escape,
and the Exceptional dropdown at desktop/mobile widths. The user's target was not changed.
29 PostgreSQL regression tests and 9 scheduler tests pass. The local company runtime is current at
schema 63, the Documents service is healthy, and Doctor reports no repair actions.

### Conversation requests in People and Attention

Attention now projects the latest direct conversation message for the current owner and each live
accountable contact. It reuses the transcript's typed intent decoder and admits only an explicit,
nonempty `ownerNeed`; prose is not classified heuristically. Reading a message does not dismiss it.
An owner reply or a superseding agent reply without a request clears it directly from source state.
Other conversations and internal agent mail do not clear it. No request table or new workflow exists.

People uses the same Attention projection for an amber Needs you label and request preview, and both
surfaces link back to the exact conversation message. Attention presents the original reply and a
same-tab Continue conversation action. The People header is one 56px row; role/id details remain on
hover, the intelligence popover and quality selector remain available, and the bottom focus card is
removed. The popover also fits a 320px viewport.

Verification: a real PostgreSQL test covers pending/read/other-conversation/answered/superseded
states and cleans up its isolated database. Frontend checks and builds pass. Live browser checks
cover 1600/820/390/320px headers, bounded popovers and controls, no sidebar horizontal overflow,
Attention at desktop/mobile widths, and return to the exact message in the same tab. The real
company transcript was inspected without sending messages or changing its settings. Final visual
calibration consulted Beautiful UI, Cult UI, and Origin UI Svelte for compact controls and restrained
state presentation; no external component code was copied.
