# Sprint 57 — Connect anything, govern by consequence

**Status:** draft for founder alignment
**Programme:** company extensibility (connectors, tools, plugins)
**Decision:** proposes ADR 0013 — *Tool calls are effects when they have consequences* (written at
ticket breakdown, after founder alignment)
**Depends on:** ADR 0002 owner provider-authentication handoffs, the Attempt-scoped MCP gateway
(`mcp_gateway.rs`, rmcp), the credential plane (Infisical references), the governed-process effect
runner and its receipts (authority-plane §2.2, §9), the approval rule (`approval.rs`)

## Outcome

An owner opens **Company → Connections**, chooses **Gmail**, and signs in with Google in their own
browser. Within a minute the connection shows what Restless observed: the account, the tools the
server offers, and how each will be used: *reads freely*, *acts with a receipt*, or *asks you
first*.

The owner then tells Exec: "Clear my inbox. Reply to the supplier threads, and chase the two
invoices that are overdue." Exec and its Staff read and label mail without asking. They reply to
people the owner already corresponds with, and each send leaves a receipt carrying Gmail's own
message id. They stop to ask once before writing to someone new. The owner can freeze the
connection, and the next send is refused with a plain reason.

The same works for a tool Restless has never heard of. The owner pastes an MCP server URL or a local
command, or installs a plugin bundle. Restless probes it, proposes how each tool will be governed,
and the owner grants it once. If anything the owner wants can't be connected, the cockpit says which
interface is missing. It never pretends.

**Product hypothesis.** If a company can't act through the owner's everyday tools, starting with
email, it can't run the business. Most of the value of "runs the business for you" is gated on this
sprint.

## Observed friction

Each item was read on 5 October 2026 from `main` at `a99cd0d`, the Sydney Resale pilot record, and
the live Sydney Resale company.

| Friction | Evidence |
|---|---|
| The owner's Gmail can't be used at all | Email is one host-side **Resend** effect sending *as the company* (`email.rs`, `coordination.rs` `send_mandated_email` → `credential::resolve(.., "resend.production")`). No path reads or acts on an owner mailbox |
| MCP writes are impossible | `mcp_gateway.rs`: `"MCP tool is outside the reviewed read profile"` and `"… reviewed public read profile"`. Every brokered MCP call is a read |
| Tools are hand-reviewed one provider at a time | Hard-coded profiles: `DEEPWIKI_READ_TOOL`, `reviewed_recurring_ch_connection`, `ReviewedHttpReadProfile`, `BrokerReadProfile`. Each new tool needs Core code |
| Two governance models and five transports | `ConnectedTool` (OAuth remote MCP via `mcp-remote` *inside* the Runtime, scopes and observed tools but no allowlist) beside `LocalMcpServer` with `stdio` (runs in the Runtime, ungoverned), `host_http`, `broker_stdio` and `public_http` (allowlisted, read receipts) |
| Effects run only CLIs | authority-plane §2.2: the runner "launches the actor-selected installed CLI with exact argv". There is no receipt shape for a tool call |
| A real pilot stalled on attaching one tool | Sydney Resale pilot, 25 Sep: "Clapping Hands is an experimental local stdio MCP server … No Clapping Hands connection has been installed or claimed working". Exec, 5 Oct, live: "the worker lacked Clapping Hands tools" |
| Harnesses rightly can't bring their own tools | `tools/custom-harness/presets.json` sets `HERMES_ACP_SKIP_CONFIGURED_MCP=1`; ARCHITECTURE §4.3 forbids ambient MCP discovery. So every tool must arrive through Restless, and today almost none can |

The references set the bar. Codex installs any MCP server with `codex mcp add` (stdio or remote URL)
and, since March 2026, versioned **plugins** bundling skills, connectors and MCP servers. Hermes
registers any MCP server's tools (stdio, Streamable HTTP, SSE) as native tools with filtering, and
reaches the user on many messaging channels. Restless must match their *reach* without giving up
what neither offers: an accountable company acting on the owner's behalf.

## Design stance

**Core contract**

- **Govern the consequence, not the transport.** authority-plane §2.2 already says the effect service
  is "an accountability boundary, not an API gateway". This sprint applies that to tools: an MCP tool
  call with a meaningful external consequence becomes an effect, with the same intent, idempotency
  key, receipt, unknown-outcome handling and freeze as a CLI effect. Reads remain ordinary work.
- **Every interface becomes one of two execution paths.** Remote MCP, local MCP, plugin bundles and
  OpenAPI services all become **MCP tool calls through Core's gateway**. CLIs stay **argv through the
  effect runner**. The browser stays the company browser. There is no third path, and no per-service
  adapter (authority-plane §2.2: "must not grow one Restless command, payload schema, or adapter per
  external service").
- **Credentials stay host-side.** OAuth tokens, API keys and refresh tokens are Infisical references
  resolved by the gateway at call time. The Runtime sees one gateway endpoint and an expiring,
  Attempt-scoped capability, as `mcp_gateway.rs` does today for reads. The in-Runtime `mcp-remote`
  credential directory is retired.
- **Permissive by default, approvals rare** (authority-plane §6). A granted tool is usable without
  asking. Approval comes only from the existing rule (first contact with a new external party, or
  revocation re-armed) or from an owner reservation. There is no policy language.
- **Probe, never guess.** A connection is *working* only after an authenticated tool list and one
  harmless call succeed. The tool contract (name, schema, annotations) is digested at grant. A
  changed contract makes only the changed tools unavailable until reviewed again.

**Default pattern**

- **Classification is judgement, made once.** At grant time Exec proposes a class for each tool,
  using the tool's name, description and schema, and the MCP annotations `readOnlyHint`,
  `destructiveHint`, `idempotentHint` and `openWorldHint`. The MCP spec says annotations from
  untrusted servers are hints, so they can't decide alone. The owner sees the proposal on the grant
  and can change any class. An unannotated, unclassified tool is treated as `acts`.
- **Three classes, shown in owner language:**

  | Class | Owner sees | Governance | Examples |
  |---|---|---|---|
  | `reads` | *Reads freely* | Gateway allowlist; read receipt | search mail, list invoices, get issue |
  | `acts` | *Acts with a receipt* | Effect: intent, idempotency, receipt, freeze; approval only by the existing rule | send reply, label thread, create issue, update CRM row |
  | `reserved` | *Asks you first* | Effect with owner approval on every call | delete mail, refund, publish, change account settings |

- **The external party is a typed field, not an adapter.** For an `acts` tool that reaches a person or
  organisation, the grant records which argument names the party (for example `to` and `cc` on a
  send). The existing first-contact rule then applies unchanged. Tools with no party record none.

**Product hypothesis**

- Most popular services now ship an MCP server (often remote, with OAuth). The rest are covered by a
  mature open-source OpenAPI→MCP bridge run as a local MCP server, and by CLIs.
- Codex/Claude-style plugin bundles become a common distribution format worth accepting as-is.

## Interfaces in scope

| Interface | How it connects | Execution path | Status this sprint |
|---|---|---|---|
| Remote MCP (Streamable HTTP; legacy SSE) | URL + MCP authorization (OAuth 2.1, protected-resource metadata) or bearer reference | Gateway → upstream | **Core** |
| Local MCP (stdio) | Command + args + env references; runs host-side in a disposable, networkable worker (generalising `stdio_mcp.rs`) | Gateway → worker | **Core** |
| Plugin bundle (Codex/Claude plugin manifest) | Install from a Git URL or path; skills → company Library (Sprint 55), MCP servers → Connections awaiting grant | As above | **Core** for the import; format churn is a hypothesis |
| OpenAPI HTTP service | Spec URL + credential reference, served by a pinned OSS OpenAPI→MCP server as a local MCP | Gateway → worker | **Stretch**: one real service |
| CLI | Installed in the company computer | Effect runner (unchanged) | Existing |
| Company browser | Persistent session | Existing browser rules | Existing, out of scope |

## The Gmail journey (acceptance anchor)

1. Owner: Company → Connections → Gmail → **Connect**. This is a provider-hosted sign-in in the
   owner's browser, outside the Runtime (authority-plane §7.1, ADR 0002). Tokens land in Infisical.
2. Restless probes: authenticated tool list, then one harmless read (the mailbox profile). The
   connection becomes *working* with the observed account address.
3. Exec proposes classes. Reads (search, get thread, list labels) are `reads`. Draft, label, archive
   and send are `acts`, with `to`/`cc`/`bcc` as party arguments on send. Delete and filter changes
   are `reserved`. The owner grants with one confirmation, adjusting any class.
4. Work runs. Reads stream through the gateway with read receipts. Each send is an effect whose
   receipt carries Gmail's message and thread ids. A send to a party never reached before becomes one
   **owner-approval** item in Attention with the prepared message. Approving it lets that send through
   and records the party.
5. Failure and control. If the gateway loses the result of a send, the outcome is *unknown* and is
   reconciled by a governed read of Sent mail before any retry (authority-plane §9.3). **Freeze** on
   the connection refuses the next `acts` call immediately. **Disconnect** revokes the token and
   deletes the reference.

## Scope by layer

### Kernel / Authority

- **Tool-call effect.** Extend the effect intent with a second execution kind beside argv:
  `(connection, tool, canonical arguments, class)`. The receipt is unchanged in shape: effect class,
  purpose, tool, the exact call (arguments as sent, minus secrets), outcome, external references from
  the result, idempotency key and execution number.
- **Idempotency for tools that have none.** MCP has no idempotency key, so Restless dedupes by intent
  key: a completed intent returns its receipt; a lost result becomes *unknown* and blocks blind retry.
- **Connection grant.** A typed Authority record: connection, granted actors or teams, per-tool class
  and party argument, tool-contract digest, expiry, frozen flag. Owner grant and revoke are
  governance facts in the Authority record. Tool calls themselves are not ledger entries beyond their
  receipts.
- **Approval** reuses the one existing rule with the party taken from the declared argument, plus
  `reserved` meaning "owner approves each call".
- **Adversarial tests:** a Runtime caller can't invoke an `acts` tool without an intent; a capability
  can't be replayed across Attempts or widened to another connection; tampering with arguments after
  intent fails; replaying an intent returns the same receipt; *unknown* blocks retry; a frozen
  connection refuses; a changed tool contract is unavailable until re-granted; the exact OAuth token is
  absent from the Runtime container's filesystem, environment and process arguments.

### Gateway (`mcp_gateway.rs`, host-side)

- One gateway path for all connections. It holds upstream sessions, resolves credentials at call
  time, enforces the grant and class, and routes `acts`/`reserved` calls through the effect runner
  before forwarding.
- MCP authorization client for remote servers (OAuth 2.1 discovery and dynamic client registration
  where the server supports it; a bring-your-own client id otherwise), refresh handled host-side.
- Generalise the disposable stdio worker so local MCP servers (and the OpenAPI bridge) run host-side
  with network access and injected credential references, never in the Runtime.

### OrgIntel

- Exec's context lists the company's working connections and their classes as capabilities, so
  planning can rely on them instead of discovering tools mid-Attempt.
- When Work needs a capability the company lacks, Exec can **propose a connection** (service, why,
  which Work). It reaches the owner as a prepared handoff, not as instructions.
- No new Work state. Tool receipts are referenced from Attempts like other effect receipts.

### Runtime

- The ACP launch contract gives each actor one MCP server: the gateway, scoped to that Attempt's
  granted tools (ARCHITECTURE §4.3 "already-authorised MCP servers"). Harness-native MCP discovery
  stays disabled.
- Retire the in-Runtime `mcp-remote` credential directory and the legacy in-Runtime `stdio`
  transport after migrating existing connections.

### Owner cockpit

- **Company → Connections**: a suggestion row for common services (Gmail, Google Calendar and Drive,
  Slack, Notion, Linear, GitHub, Stripe, Shopify, Xero). These are known endpoints or plugin bundles,
  not curated adapters. Below that, **Add**: an MCP URL, a local command, a plugin Git URL or (stretch)
  an OpenAPI spec URL.
- Each connection shows observed status, account, and its tools grouped *Reads freely / Acts with a
  receipt / Asks you first*. Recent receipts lead, not configuration. Freeze and Disconnect are on the
  connection. Class changes happen in place.
- Tooltips, not subtitles, for class meaning and consequences. No roster or status wall (CLAUDE.md).
  Follow `docs/FRONTEND_DESIGN_REFERENCES.md` in the final pass.

## Acceptance

All live connector runs use a `_test` company and a **dedicated test account** for each service. They
never touch the owner's real mailbox or a live company (evaluation-dogfood §9.6.1).

1. **Gmail, end to end.** Steps 1–5 of the journey on a test Gmail account, run by Exec and one Staff
   member from a single owner instruction. Evidence:
   - read receipts;
   - send receipts whose message ids appear in the test account's Sent folder;
   - one owner-approval item for a new recipient, then the approved send;
   - a forced lost result resolved as *unknown* → reconciled, with no duplicate in Sent;
   - freeze refusing a send;
   - disconnect leaving no token reference.
2. **No token in the Runtime.** Grep the company container's filesystem, environment and process
   arguments for the **exact** access and refresh token strings. Zero matches.
3. **Arbitrary MCP.**
   - Connect one non-suggested remote MCP server that has both read and write tools, by URL.
   - Connect one local stdio server by command.
   - An agent completes a task using a `reads` and an `acts` tool on each.
   - Changing one tool's schema upstream makes only that tool unavailable until re-granted.
4. **Plugin bundle.** Install one public Codex-format plugin from Git. Its skills appear in Library;
   its MCP server appears in Connections awaiting grant; after the grant an agent uses it.
5. **Harness neutrality.** The same Gmail task succeeds under two different harnesses (for example
   Codex and Hermes), because tools come from the gateway, not harness configuration.
6. **Honest gaps.** Adding a service with none of the supported interfaces shows which interface is
   missing. It doesn't produce a fake connection.
7. **Deletion done.** The per-provider read profiles are removed, with the in-Runtime `mcp-remote`
   path and the legacy in-Runtime stdio transport. Clapping Hands is deprecated and was not
   migrated.
8. **Stretch: OpenAPI.** One real service with an OpenAPI spec and no MCP server is connected
   through the OSS bridge, and an agent performs one `acts` call with a receipt.

## Deletable machinery

`DEEPWIKI_READ_TOOL`, `ReviewedHttpReadProfile`, `BrokerReadProfile`,
`reviewed_recurring_ch_connection` and the per-profile install functions (`install_public_http_read`,
`install_brokered_stdio_mcp`, `install_host_mcp`); `ConnectedTool`'s in-Runtime `mcp-remote`
materialisation and `runtime_credential_dir`; the in-Runtime `stdio` transport. One connection model
replaces the two, and one install path replaces the four.

## Out of scope (named next)

- **Inbound events.** Webhooks or watches from connected services waking Exec (a new order, a reply
  from a supplier). This is the natural Sprint 58: the Hermes gateway lesson applied to the company.
- **Owner channels.** Reaching the owner on Telegram, WhatsApp or email for the prepared last mile.
- **A marketplace or discovery UI.** Suggestions are a short list; there's no ranking, reviews or
  store.
- **Per-call model judgement of consequences.** Classification happens at grant; runtime governance is
  typed.

## Risks and dispositions

| Risk | Disposition |
|---|---|
| Google classes Gmail read/modify scopes as *restricted*: a published OAuth app needs verification, and an unverified app in testing mode is limited to listed test users with short-lived refresh tokens | **Pending decision (T1).** Probe Google's own hosted Workspace MCP endpoint, if one is usable, against bring-your-own OAuth client for self-hosted owners. Cloud may need a verified app |
| A malicious or compromised MCP server lies in its annotations or results | **Guarded.** Annotations are only hints; classes are granted; results are untrusted data in context, like web content |
| A tool contract changes silently upstream | **Guarded.** Digest at grant; changed tools fail closed individually |
| Owners over-grant `acts` on destructive tools | **Accepted.** Permissive by design; Exec defaults destructive tools to `reserved`; freeze is one click |
| The plugin format churns | **Accepted.** Import is a thin reader; the formats are files |
| A gateway outage blocks every connector | **Accepted** for V0. It is the same process as the daemon, and receipts make retries safe |

## Open questions for founders

1. Gmail first, or Gmail and Google Calendar together as the anchor? Calendar invites are a second
   party-reaching `acts` shape that would test the party argument design.
2. Should Exec's classification proposal need owner confirmation on every grant, or only when it
   proposes `acts` or `reserved` for a tool annotated `readOnlyHint`?
3. Is a team-level grant needed now, or are actor-level and company-wide grants enough for V0?

## Tickets

Ticket files are written at breakdown. Each will cite the friction above, name its layer and state
what it deletes.

- [ ] T1 — Probe the Gmail paths: Google's hosted Workspace MCP endpoint vs a community Gmail MCP
  server with a bring-your-own OAuth client. Record scopes, verification limits and the token
  lifetime observed. *Decision input, Kernel/Gateway*
- [x] T2 ([ticket](sprint-57/s57-t2-tool-call-effect.md)) — Tool-call effect kind, intent idempotency, receipt and unknown-outcome reconciliation, with
  adversarial tests. *Kernel*
- [x] T3 ([ticket](sprint-57/s57-t3-connection-grant.md)) — Connection grant record (classes, party argument, contract digest, freeze, expiry) and the
  first-contact rule applied to declared parties. *Kernel*
- [x] T4 ([ticket](sprint-57/s57-t4-one-gateway.md)) — One gateway path: host-side remote MCP with MCP authorization, credential resolution,
  class enforcement and routing through the effect runner. *Gateway*
- [x] T5 ([ticket](sprint-57/s57-t5-local-worker.md)) — Host-side local MCP worker (generalised `stdio_mcp`), networkable, with credential
  references. *Gateway/Runtime* Live evidence: Sprint 61 connections smoke assertion 15 (a local
  command connection with an `env:` credential reference, read through the gateway). Destroying a
  company now also ends its workers and removes their cache.
- [x] T6 — Launch contract: one gateway MCP server per actor, scoped to granted tools; retire
  in-Runtime `mcp-remote` and legacy stdio. *Runtime* Done by deletion: `connected_tool.rs`,
  `mcp_gateway.rs`, `stdio_mcp.rs`, the `connected-tool` and `local-mcp` commands, the owner
  `/company/mcp/*` routes and the Limits page's MCP rows are gone, and `mcp-remote` left the
  company image. No migration: the owner deprecated Clapping Hands, and Restless carries no bespoke
  connectors. Legacy Authority tables are left orphaned rather than dropped.
- [ ] T7 — Exec classification proposal and connection proposals as prepared handoffs; connections
  in Exec context. *OrgIntel*
- [ ] T8 ([ticket](sprint-57/s57-t8-connections-page.md)) — Company → Connections: suggestions, add by URL, command or plugin, probe status,
  three-class grant, receipts, freeze, disconnect. *Cockpit*
- [x] T9 ([ticket](sprint-57/s57-t9-plugin-import.md)) — Plugin bundle import (skills → Library, MCP servers → Connections). *Runtime/Cockpit*
  Live evidence: Sprint 61 assertion 16 (a Git clone over https, the server added with its `plugin:`
  source, and the skill reaching candidacy through `restless skill add`). Exec's own skill step
  still needs a model run.
- [ ] T10 — Acceptance runs 1–7 on `_test` with dedicated accounts; T11 (stretch) the OpenAPI bridge
  and acceptance 8. Acceptance 2 (no token in the Runtime), 3 (arbitrary MCP, local stdio and a
  changed contract), 4 (plugin bundle) and 7 (deletion) are carried by Sprint 61's connections smoke
  against a fixture provider. Acceptance 1 (Gmail) and 5 (two harnesses under a model) remain.
- [ ] T12 ([ticket](sprint-57/s57-t12-telegram-attention.md)) — Telegram Attention channel: pairing,
  delivery and approval buttons through the cockpit's decision functions. Built; open until a real
  bot run on `_test`. *Kernel/Cockpit*
