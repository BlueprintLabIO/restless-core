# Company settings audit

The Company area now separates editable direction/settings from historical records.
Changes were verified on a disposable company; the owner's company settings, credentials,
identity and charter were not changed for testing.

| Surface | Result |
| --- | --- |
| Navigation | Resources and Authority combined as Access & limits. Decision history and External activity follow the settings. Mobile has a labelled page selector. The old Authority link redirects to the merged page. |
| Charter | Name and charter retain their existing save paths. Internal navigation now protects an unsaved charter draft. |
| Identity | Add/Edit owner direction for Truth, Voice, Visual language and Culture. Save creates a new attributed release through the existing Authority promotion path. Older releases and independent evidence remain intact. Version history expands to show its actual statements. Internal metadata is secondary. |
| Access & limits | Visible spend-limit editor with exact decimal validation, owner check and stale-value protection. Outcome selector restores its saved value if a request fails. Permission boundaries and recorded payment/party grants are explained separately from editable model limits. |
| Decisions | Explicitly history, with links to pending decisions and the related work. Historical decisions are not rewritten. |
| External activity | Explains governed messages, submissions and payments and why internal chats/document edits do not populate this history. Unavailable remains distinct from empty. |
| Intelligence provider | Direct, native harness and agent-assignment controls audited. Shared button styling. Refresh clears recovered read errors. Native authentication polling cannot replace a newer action response, and read failures clear after recovery. |
| Vault | Failed reads no longer claim “Checking” or connected. Refresh, metadata search, empty state and credential-location details preserved. |
| Computer | Observer/control entry preserved; failed session-client initialization now reports an error instead of silently disabling entry. |
| Doctor | Startup read failure has a retry action, startup errors stop pending polling, and stale diagnostic results are labelled. Explicit Recheck control. |
| Shared controls | Consistent tokens and buttons. Help text uses a native top-layer popover positioned within the viewport, accessible by focus or click and dismissible with Escape. |

## Verification

- `cargo build -p restlessd` passed.
- Three real PostgreSQL identity scenarios passed: existing release/restart/immutable Work binding contract;
  owner editing preserves independent evidence and historical versions and rejects stale drafts;
  two concurrent first promotions cannot both become current.
- Real isolated daemon/browser: Identity add/edit/save/reload and stale rejection; spend-limit save/reload,
  invalid decimal and stale rejection; outcome save/reload and failed-update rollback; Charter/name save,
  reload and unsaved navigation; version contents; native sign-in failure UI; custom provider input;
  Doctor read/retry; Vault read failure/recovery.
- Loaded Company pages inspected at 1440, 390 and 320 pixels, including the mobile identity editor.
  Expanded permissions, tooltip bounds, Escape dismissal and page-selector navigation checked separately.
- Svelte/TypeScript checks passed without errors or warnings; production frontend built successfully.
- OAuth account authentication and destructive recovery actions were not executed against the owner's
  accounts or running company. Their presentation, loading/error handling and existing implementation
  paths were audited; successful third-party sign-in is not claimed by these tests.

The browser regression is `web/scripts/verify-company-settings.mjs`. It requires an empty disposable
company whose ID ends in `_test`, a running test daemon with built cockpit assets, and Playwright Chromium.
Set `RESTLESS_SMOKE_ORIGIN`, `RESTLESS_SMOKE_COMPANY`, optionally `RESTLESS_BROWSER_EXECUTABLE` and
`RESTLESS_VERIFY_OUTPUT`. It writes settings only in that explicitly selected disposable company.

Visual calibration: Beautiful UI's restrained control/state hierarchy, Cult UI's compact disclosure
patterns, and Origin UI Svelte's labelled form controls were consulted. No upstream code or additional
UI runtime was imported. Existing Restless typography, surfaces, spacing and button primitives are used.
Local screenshots and logs are in `work/company-audit` outside the repository.

## Installed result

The built cockpit was copied to the local served directory and `restless-local.service` restarted.
Live Identity, Access & limits, Decision history, External activity, Intelligence provider, Vault and
Doctor were verified. The live desktop opened in observer mode and returned to Company without
claiming control. The disposable company, runtime, database and roles were removed afterward.
