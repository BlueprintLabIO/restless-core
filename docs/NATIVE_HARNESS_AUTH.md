# Independent native harness authentication

Company → Intelligence provider contains direct connections, independent ChatGPT/Codex and Claude Code logins, installed agent software, additional harness installation, one company default connection/model, and optional per-agent overrides. The old Harnesses URL redirects there. The selected connection determines the execution harness automatically; there is no separate harness policy. Doctor retains operational checks. Native logins never inherit a direct provider key or subscription.

Native OAuth uses the installed vendor CLI. Codex starts App Server `account/login/start` with `chatgptDeviceCode`; the owner follows its verification URL/code. Claude runs `claude auth login --claudeai` in the company computer and opens the OAuth browser there, preserving the local callback. Completion is confirmed through native account status. Login attempts expire after 15 minutes and can be cancelled. Refresh and logout remain owned by the native CLI.

This is an explicit exception to the host-relay credential-custody model: native CLI OAuth credentials live in a private, company-local profile under `/company/home/.restless/harness-auth/{codex,claude-agent}` on the durable company volume. They are not represented as Infisical-held OAuth tokens. API keys have separate Infisical references `HARNESS_<harness>_API_KEY`; selected keys are passed to the native process through its environment, never command arguments or owner responses. Codex's native account manager may also persist its API credential in that private profile. The Infisical machine identity stays on the host.

Native subscription usage is not an API charge. Native API usage is marked `native_api_unmetered`: it bypasses the Restless model relay, so its tariff accounting and spend enforcement do not cover those calls. Provider billing remains authoritative. Existing scoped relay routes remain available for managed inference.

The native-auth endpoints are local-account-host only. They validate the company and harness, serialize config writes with other company changes, and expose curated native status rather than CLI output. Disconnect clears that harness's configuration and native login without altering direct connections. A disconnected selected native harness fails explicitly instead of silently falling back to a direct credential.

Local startup and company creation automatically start the company computer and run Runtime Doctor. Results are persisted under `diagnostics/<company>-startup-doctor.json` and shown on Company Doctor. A failed setup still runs diagnostics and does not prevent other companies from starting. Isolated test planes can disable automatic scheduling/startup work.

Per-agent intelligence assignments live in company configuration under
`agent_intelligence.<actor>`, containing a connection (`direct:<provider>` or
`harness:<id>`) and an unqualified model ID. They carry no credentials. An explicit
assignment replaces that actor's old model preference and company failover chain
for the next conversation or work session. Other agents retain their own route;
removing an assignment restores the company default. The reserved `agent_intelligence.default` entry applies to every agent without an override and supersedes older actor model preferences. Legacy configurations without that entry retain their previous routing until a default is selected. The first connected native account (or first direct connection saved with a model) becomes the default; subsequent connections do not replace it. Native setup resumes after daemon restarts. Changing assignments clears automatic wake backoff and notifies the scheduler; model cooldowns remain scoped to their original route. Native harness
assignments use the harness's independent authentication profile with the assigned
model. People and Intelligence provider show the effective model.

The local Intelligence provider page lists saved direct connections and offers
agent assignments across direct and native connections. New direct credentials
still require a daemon restart to load the gateway; the UI reports this pending
state. With no configured direct or native connection, Exec's message entry links
to Intelligence provider. An unavailable credential observation does not count as
an authoritative empty state.

Company → Vault is a read-only inventory of the company's Infisical directory,
including nested folders (up to Infisical's 20-level recursive limit). Requests
use `viewSecretValue=false`; the owner API also allowlists only names, paths,
references and update times and rejects rows outside the company directory.
Configured references outside that inventory and native OAuth profile locations
appear separately. The inventory does not expose secret values or edit/delete
credentials; connection setup remains in Intelligence provider.

Fallback model suggestions in company settings and agent assignments use `https://models.dev/api.json`. The browser checks hourly while the selectors
are mounted (and on returning to stale settings), persists a validated last-good
catalog, and offers a manual refresh. A failed fetch keeps cached suggestions;
without a cache, bundled suggestions remain available. Supported provider IDs are
mapped explicitly, including Moonshot → `moonshotai`; local LiteLLM gateways keep
their bundled/custom IDs. Only text-output, tool-calling, non-deprecated entries
are suggested, newest releases first. Refresh never changes saved assignments or
typed model IDs. Catalog presence does not establish account access, native CLI
compatibility, runtime admission or billing prices. The public request sends no
credentials or referrer. Parser invariants: `node scripts/verify-model-catalog.mjs`
from `web/`.

For signed-in Codex connections, App Server `model/list` with hidden models excluded is the authoritative model list. Its recommended default is used for first-connection setup. If native discovery is unavailable (including Claude), selectors clearly identify models.dev suggestions as a fallback. Custom model IDs remain available. Model selection lives in the company default and agent overrides, not in a second harness setting. Refreshing a model list never changes an existing assignment.

The embedded desktop opens in observation mode with a persistent instruction to click Take control. A larger high-contrast ring cursor stays visible against light and dark content in either mode, centered on the click hotspot. The instruction disappears when this tab owns control. Cursor visibility never grants keyboard or pointer control.
