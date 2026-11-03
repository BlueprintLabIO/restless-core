# Hard blockers and catch-22s: audit, 3 November 2026

The owner calls these critical bugs because they stop work getting done.

**Hard blocker.** An agent (Exec or staff) needs something it can't get on its own, and neither
the owner nor an agent has a working way to provide it.

**Catch-22.** A circular dependency. For example: approval is needed to request the approval, or an
owner card exists but no agent command can raise it.

## Sources and method

**Production evidence (read only).**
- **Plane company data:** the work graph, Attempts, handoffs, Attention, and every Exec
  conversation in full (843 messages across the six companies).
- **How it was read:** GET requests from the owner session signed in to the built-in browser.
- **Fleet:** read through `fleet-sql.sh`, SELECT only.
- **Not read:** staff-to-staff conversations. The `restless` CLI contexts in `~/code/work/s106-prod`
  return 401 and would need a fresh login, which counts as a write.

**Code audit.** Every gate in `restless-engine`, `restless-owner`, the CLI, `web/src` and Cloud
(`vm-provider`, Fleet) was checked against two questions:
- **(a)** When it refuses, is there a reachable path to get unblocked? That means an agent command
  that raises an owner card, or an owner page linked from the refusal.
- **(b)** Does the refusal tell the agent that path in words it will act on?

**Out of scope.** Already in hand elsewhere, so not redone here:
- #595: Exec raises the email mandate card.
- #588: Resend key names.
- #596: fault delivery.
- The generic mandate model.

## Ranked findings

Ranked by how much work each one blocks. **Fixed** links the PR. **Owner** means it needs the
owner's decision.

### 1. Resumable Work never resumes (no card, no wake)

- **Evidence:**
  - 34 Attempts on 23 Work items across the companies.
  - Restless company: Sprint 107 A1–A3, A4, C and D have been Blocked since 29 Oct, with no
    progress for five days.
  - The staff summary reads: "[resumable] the agent produced nothing for 9 minutes and the turn
    was cut... the next wake continues from durable state".
- **Code:**
  - `health.rs:273` classifies the cut as resumable.
  - `staff/execution.rs:1638/1773` returns `Blocked` with a `[resumable]` prefix.
  - `staff/recovery.rs:195/417` then calls `finish_work_attempt(Blocked)`.
  - Nothing reads the prefix. The scheduler only claims proposed or active Work, so the promised
    "next wake" never comes. The only exits are a plane restart (`resume_after_platform_restart`)
    or someone running `restless work resume`.
  - The lead is told the next wake continues, so it does nothing.
- **Fixed** (#602):
  - A `[resumable]` block resumes automatically, at most twice per Work revision.
  - The owner's own Stop is excluded.
  - Each resume records a `work_resumed_after_interruption` event.

### 2. Missing session-ownership marker turns finished work into blocked work

- **Evidence:**
  - 20 Attempts on 17 Work items: Reno designers, reviewers and supervisor; Potion Scheme
    print-production and rules-writing; Restless maintenance, conversation and cloud.
  - Quotes: "agent session ownership marker /tmp/restless-agent-….sid is missing; broad process
    cleanup was refused" and "nothing will be retried automatically. Ask Exec to check and carry
    on."
- **Code:**
  - `acp.rs:2458-2472` and `codex.rs:998` raise the error.
  - `read_session_id` turns a slow or failed `docker exec cat` (8 s timeout under memory
    pressure) into "missing".
  - Staff call `with_agent`, which turns `CleanupFailed` into `Err`. That throws away the
    productive outcome, even a met one, and records unknown recovery.
  - `7338704a` fixed only the case where the computer restarted, and only in `acp.rs`.
- **Open:**
  - Keep the outcome when only process-ownership cleanup is unverified.
  - Retry the marker read.
  - Copy the restart guard into `codex.rs`.
  - Estimated at about 50 lines. Not done here: it needs the ACP harness tests, which need a
    computer.

### 3. Account-grant refusals say "ask the owner", but no command asks (catch-22)

- **Code:**
  - `account_broker::decide` produces "Ask the owner to widen the grant", "Ask the owner for more
    (the company's Accounts page)" and the Ask decisions.
  - `tool_gateway.rs:1218`: "An account's browser cannot wait for approval: ask the owner to allow
    acting with it."
  - `account_git.rs:277`: "A push cannot wait for approval: ask the owner to allow it."
  - The only agent command, `restless apps ask-account`, never names an account.
  - The card that answers this already exists: an `account_request` naming an account, "asks for
    more on an account", with Approve. Nothing on the agent side raised it.
- **Evidence:** Restless and Jolly Heron are waiting on GitHub and Cloudflare grants. Sydney
  Resale's eBay sign-in card has been pending since 23 Oct.
- **Fixed** (#601):
  - A refusal the owner can lift (an ability that is off, or an ask a browser or push can't wait
    for) now raises that card, deduplicated per account.
  - It tells the agent "Asked the owner".

### 4. The agent that asked for an account never hears the answer

- **Code:**
  - `account_broker::settle_request` and its two callers (`approval_decisions.rs:936`,
    `owner_accounts.rs:865`) message nobody.
  - The Deny action still promises "the company is told". Only the API-key path tells the asker
    (`owner_vault.rs` `tell_asker`).
- **Fixed** (#601): approving or denying messages the requesting agent.

### 5. A Vault key that isn't bound to its name loops the agent (catch-22)

- **Code:**
  - `platform_capabilities::already_held` counts keys the Vault holds without a binding, and
    answers "The company Vault already holds X. Use it by name … no owner request is needed."
  - `credential::resolve` only reads bindings, so the same name fails.
  - The failure said to run `restless credential set`, which is owner-only.
  - Asking again returns `already_held` again.
- **Fixed** (#599):
  - `resolve` falls back to a Vault secret with exactly that name.
  - The refusal names `restless apps ask-account --kind api`.

### 6. Refusals that send agents to owner-only commands (catch-22)

- **Code:**
  - `approval.rs:749`: "approve with `restless approve -c X --party Y`" on every first contact.
    `approve` is owner-only, though the card was already raised.
  - `credential.rs:329`: names `restless credential set`.
  - `prompts/align-and-learn.md:37`: tells Exec to run `restless identity voice-learn`, which is
    owner-only.
  - `wire::authorize`: "{cmd} is an act of owner authority" names no way out, across 35 commands.
- **Fixed** (#599):
  - Each owner-only command has a door: the agent command that raises the owner's card, the card
    already raised, or the owner's page plus how to tell them.
  - The refusal ends with that door.

### 7. First-contact answers reach Exec, not the staff member who prepared the send

- **Code:** `approval::announce_decisions` sends to `exec` only. The preparer recorded on the
  `approval_required` record waits forever.
- **Fixed** (#599): the answer also goes to each non-Exec preparer of that party or call.

### 8. Workspace "differs from requested source": Work can't be taken back

- **Evidence:** Restless company, 7 Attempts on 6 Work items, every one abandoned and recreated.
  Quote: "Retrying the same job won't help."
- **Code:**
  - `staff/workspace.rs:597-604` keeps one persistent worktree per Work. When the requested base
    commit differs from where the last Attempt left HEAD, every successor fails.
  - The Attempt is then left running until orphan recovery, which invites a resume that fails the
    same way.
- **Open:**
  - Save the old tip under `refs/restless/preserved/<attempt>`.
  - Reset a clean worktree to the exact commit.
  - Close a failed setup as `Failed` with the real cause.
  - Medium risk (git state); needs its own PR.

### 9. Proposed Work requiring abandoned Work can never start (fixed, #603)

- **Evidence:** Frankensurf, "Prove which agent tools actually work", stuck since 18 Oct with 0
  Attempts.
- **Code:**
  - Readiness is `upstream.status <> 'completed'` (`restless-orgintel/src/attempts.rs:72,126`).
  - Abandoning never looks at dependents.
  - `add_work_edge` accepts an abandoned upstream.
- **Fixed** (#603): abandoning tells each open dependent's lead (`work edge ... --remove` or
  abandon), and new requirements on abandoned Work are refused. Already-stranded Work needs the
  sweep in finding 10.

### 10. Blocked Work with no card and nobody asked

- **Evidence:**
  - Frankensurf "Reconcile the public 0.50.4 release", blocked since 19 Oct. Its "accountable
    recovery" is a single message to the lead.
  - Items 1, 2 and 9 also end in this state.
- **Open:** a sweep that raises one card per revision for Work blocked over 24 h with no pending
  handoff, with Resume and Abandon. That is the backstop for this whole class.

### 11. Unknown generic-effect outcome blocks the key forever

- **Code:**
  - `effect.rs:551` refuses a retry while the outcome is unknown.
  - `restless effect-reconcile` always fails: "provider-specific reconciliation is not
    available".
  - No card is raised: `attention.rs` only shows `tool_call_approved` keys.
  - `effect_reconciled` is read in `company.rs:879/1640` but never written.
- **Owner:** decide whether the owner may attest "it happened / it didn't" from an
  `effect_outcome` card. That is new authority over irreversible effects, so it isn't done
  silently here.

### 12. Publishing a service never works on a hosted plane

- **Code:**
  - Cloud's `vm-provider/plane.mjs` never sets `RESTLESS_PUBLISHED_SERVICE_PROVIDER` (or its URL
    and token). Core's own compose template does.
  - The agent read "candidate image builds require the production cloud-http provider". The owner
    read "could not be authorized. Try again."
- **Fixed in part:** the refusal now says publishing isn't available on this plane, not to retry,
  and to file it.
- **Owner:** whether Cloud runs a publication provider for planes. Until then, publishing is off
  on hosted planes.

### 13. Staff can't open a Library document

- **Evidence:** Reno studio-researcher, 28 Oct: "I still can't open the matching Library document
  from my session... The supervisor, Exec or the owner needs to copy the file into it or give me
  access". Reno Exec on 12 Oct: sharing returned an error on both files.
- **Open:** the agent has no command to request access, so there is no card.
- **Fixed in part:** `restless document request-collaboration` now appears in the agents'
  instructions. It existed but no prompt named it.

### 14. "Start the computer" hides the monthly-hours cap (fixed, #599)

- **Code:**
  - `restless-owner/src/owner.rs:8935` turns every `recover` error into "The computer could not
    be recovered. Try again."
  - The real reason ("monthly Runtime cap … raise or clear the cap") has a fix in Company ›
    Resources, which the message doesn't name.
- **Fixed** (#599): the start route returns 409 "Raise or clear the limit in Company → Resources", as waking the desktop already did.

### 15. Quota block survives switching the connection; a short 429 becomes a 1 h block

- **Code:**
  - `authority.rs:733` clears only `credential` blocks when a new credential is granted.
  - `health.rs:409` and `model_gateway.rs:621` turn a short 429 into an hour labelled "out of
    credit or quota".
  - The quota card says "Work resumes within minutes of a top-up". That isn't true for
    already-claimed Work (item 1).
- **Fixed** (fix/model-avail-live-row, 3 Nov; lifted from the unmerged fix/s109-blocker-cards):
  - Granting a provider another connection clears that provider's quota and credential
    cooldowns, not only credential ones.
  - A rate limit (429) cools down for the provider's own wait, or two minutes, never an hour, and
    raises no "out of credit" card: it clears by itself.
  - The first output of any turn on a model lifts that model's cooldown at once, rather than
    when the turn ends.
  - The quota card's top-up wording (item 1) is still open on fix/s109-blocker-cards.

### 16. Native-turn cap raises a "spend limit" card that can't lift it

- **Code:**
  - `spend.rs:303` refusals are `[budget]`.
  - `attention.rs:3653` maps any `[budget]` refusal to the spend-stopped card, whose editor sets
    only the spend limit.
  - Raising the native limit doesn't wake Exec.
- **Open.**

### 17. Setting changed directly in Settings blocks all future proposals for it

- **Code:**
  - `attention.rs:4105` hides a proposal once Settings matches it, but never records
    `setting_decided`.
  - `setting_proposals::pending` still counts it.
  - Every later proposal for that setting (keyed per variant, so every schedule) is refused:
    "already waiting for the owner's answer". The owner sees no card.
- **Open.**

### 18. A judgement handoff with a stale brief vanishes

- **Code:** `attention.rs:2888` drops it with a `warn` once the Work revision changes. The handoff
  stays pending, the Work stays blocked, and nobody is asked to re-brief.
- **Open.**

### 19. Cloud: setup on a sold-out location retries forever, reason lost

- **Code:**
  - `vm-provider/server.mjs:426` returns `location_out_of_stock`.
  - Fleet `plane_provider.rs:167` flattens every non-2xx response to `provider_refused`.
  - `store.rs:1312` retries every 120 s with no end.
  - The owner sees "Usually about a minute" indefinitely.
- **Open** (Cloud).

### 20. Cloud: deleting an account needs support, and the refusal says otherwise

- **Code:**
  - `fleet-web/routes/account/+page.server.ts:909` says to delete each company from its Service
    page.
  - Archiving needs runtime status `stopped`, but plane-hosted runtimes are always
    `plane_hosted`.
- **Open** (Cloud).

### Lower

- **Staff calls recorded as Exec's:**
  - `coordination.rs:1501/1520/1539` hard-code actor `exec` for `finance-reserve`,
    `finance-payee-propose` and `setting-propose`.
  - The answer goes to Exec, not the staff member.
- **Home-site helper offline:**
  - `home_egress.rs:163` replies "Turn it on, or ask the owner" with no card.
  - The same text is used when no helper is assigned at all.
- **Codex/Claude reconnect loop:** `owner_oauth_login.rs:262/487`, "Restore that connection
  before reconnecting", with no restore action.
- **Dead links in Inbox items (fixed, #599):**
  - `attention.rs:1574` links `/{company}/vault`; the page is `/company/vault`.
  - `attention.rs:912` links `/company/limits#spend`; there's no such page.
- **Unrecorded faults:** `coordination.rs:408/414` ignore errors from `ensure_actor` and
  `record_delivery` when filing a fault. This is #596's area, noted for it.
- **Stale refusal replayed:**
  - A reused ACP session (`responsibility = work:{id}`) and replayed feedback repeated a stale
    GitHub refusal six times after the fix (Restless maintenance, 29 Oct).
  - New Attempts after a block or failure should start a fresh session.

### External, with a working owner path

These are not Restless bugs. They are listed because they stopped work for days:
- GitHub flagged the BlueprintLabIO grants (fixed 27 Oct).
- Cloudflare token scopes (Reno DNS, Email Routing). A "done" card was accepted without checking
  that the scope works.
- Facebook password field detection (`account_browser.rs:970`).
- Brand voice waiting for the owner's OK (Jolly Heron since 25 Oct).

## The structural guard

`crates/restless-engine/src/doors.rs` holds two lists. Its tests fail when either list is
incomplete.

- **`OWNER_ONLY_DOORS`** gives every command in `wire::OWNER_ONLY` a door:
  - **`Ask`:** an agent command that raises a named Inbox card.
  - **`Raised`:** the gate has already raised the card.
  - **`Tell`:** the owner's page, plus how Exec or staff tell them.

  `wire::authorize` appends the door to its refusal.

  Tests:
  - Every owner-only command has a door.
  - Every `Ask` door is a real command that isn't itself owner-only.
  - Its card is a known kind.
  - Its spelling appears in the agents' instructions.
  - The refusal text contains the door.

- **`ATTENTION_KINDS`** says who raises every Inbox card kind: system, person, or an agent command.

  Tests:
  - Every `kind: "…".into()` in `attention.rs` is listed.
  - Every agent-raised kind has a real, non-owner-only command.
  - The agents' instructions name that command.

- **A text guard** fails when an agent-facing source or prompt tells an agent to run an owner-only
  command (`restless approve`, `credential set`, `company set`, `identity voice-learn`, …).

Adding an owner-only command, a card kind, or a refusal that points at the owner's CLI now fails CI
until it has an agent's way through.
