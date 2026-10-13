# Direction and permission: act, then tell

**Status:** proposed (Sprint 80)
**Owner decision:** 2026-10-14. The owner asked for all four parts.
**Replaces:** the prose-only "a person directing a team someone else owns is a proposal" rule in
Exec's doctrine (`context.rs`, "People own teams").

## The problem

A member asked Exec: "can you ensure the website copy is always in first person speaking as the
business?" Exec answered with four paragraphs explaining that it had changed nothing, because direction for the team
is the owner's call, and that the message was "a proposal to them". Nothing was recorded or routed.
The owner was never asked, and the member had no way to see whether anyone ever would be.

Asking before every change makes the owner a dispatcher; `owner-cockpit §2.3` already rejects that.
Silently doing whatever anyone says makes the owner a passenger. The way through is to ask *less
often*, about the right things, and to make each ask and each answer one click.

## The rule

Every change Exec makes on a person's direction takes exactly one of three paths.

| Path | When | What happens |
|---|---|---|
| **Direct** | The person owns the team the change is for, or is the owner | Exec applies it. Nothing changes from today. |
| **Act, then tell** | The change is *reversible* and in a *grantable class* (below), or a standing grant covers it | Exec applies it now, records what it changed and how to undo it, and tells whoever owns the direction, with an Undo button. |
| **Ask first** | Anything else: irreversible, spends money, reaches outside the company, touches membership, credentials or legal standing | Exec files a **Request** with whoever owns the direction, tells the requester it is pending, and carries it out the moment it is approved. |

"Whoever owns the direction" means the team's person when the team has one, otherwise the owner.
That routing already exists for judgement.

### Classes

Classes form a small fixed list, not a policy language (`approval.rs` explains why). A change belongs to
one class or none. A change in no class is always Ask first.

| Class | Covers | Undo handle |
|---|---|---|
| `content` | wording, copy, tone and style rules, team briefs, charters, native documents and sheets | the document or charter revision before the change |
| `work` | priority, order and scope of Work within its goal; pausing and resuming Work | the prior Work fields |
| `staffing` | an agent's model; adding or retiring an agent within a team | the prior model or roster entry |
| `schedule` | creating, changing or pausing a schedule | the prior schedule |

Never act-then-tell, and never grantable: spend or budget changes, external effects (email, publishing,
payment, first contact), deleting data, membership and roles, connections and credentials, legal
attestation, and anything else without an undo handle. These are always Ask first, and the owner
decides them.

A member's change is **act-then-tell by default** for `content` and `work` inside a team nobody else
owns. `staffing` and `schedule` changes from a member are Ask first until a grant covers them,
because they spend model time.

## The four parts

### 1. Act, then tell, with Undo

- Exec applies the change, then records an **Applied** entry (Authority record
  `direction_applied`):
  - who asked, and the source message;
  - the class and the team;
  - a one-line summary in the requester's words ("Website copy speaks in first person as the business");
  - what changed (a reference to the document, charter, Work or schedule);
  - the undo handle.
- **Who is told:** whoever owns the direction, as a For you row: "Applied for Bianca: website copy
  speaks in first person as the business · Undo". The row opens the change: a document diff using
  S77's version compare, or the before and after for other classes.
- **Undo** is one click for 7 days, for the owner and the team's person.
  - It restores the handle, records `direction_undone`, and tells the requester in plain words:
    "Yail undid: website copy in first person."
  - If something has changed on top since, Undo says so ("Changed since. Open it to revert by
    hand.") and does not overwrite the newer work.
- **The requester** gets the confirmation in Exec's reply, short: "Done: the studio brief now says
  customer-facing copy speaks in first person. Yail can undo it."

### 2. Requests that are easy to make and easy to answer

- **Making one:** a person never has to know the rule. When Exec's path is Ask first, it files the
  Request itself with `restless request file --to <person|owner> --class <class|none> --team <team>
  --summary <one line> --change <exact change> --reason <why> --source-message <id>`.
- **In the rail**, the requester sees a request card in place of paragraphs of explanation:
  "Asked Yail: make customer-facing copy first person · Pending". The card updates live to
  Approved, Declined (with the reason) or Done.
- **In their For you**, the requester sees their open requests under "Your requests".
- **Answering one:** the request is an Inbox item for whoever owns the direction, holding the
  summary, the exact change, who asked and why. It has three buttons:
  - **Approve:** Exec carries it out, then it's Done.
  - **Approve, and always allow:** see part 3.
  - **Decline…:** an optional one-line reason, passed back to the requester.

  Keyboard: `Y` approves, `A` approves and always allows, `N` declines. The owner can also answer by
  replying to Exec in plain words ("yes, go ahead"); Exec records that as the decision with the
  message as its source.
- Exec carries out an approved request in the same Authority transaction as the decision, so an
  approval can never be "approved but not done".
- A request is answered once. Two owners or admins racing see "Yail already approved this".

### 3. Always allow

- **Approve, and always allow** records a **standing grant** (`direction_grant`): the grantee (one
  person, or every member), the class, and the scope (one team, or the company).
- Grants are offered only for grantable classes. The button names exactly what it allows, for
  example "Always allow Bianca to change content for Site Renovation Studio".
- With a grant in place, a matching change takes the **act, then tell** path. The owner still sees
  it and can Undo, but is no longer asked.
- Grants are listed and revoked in Company → Members → **Standing permissions**, one row each:
  "Bianca can change content for Site Renovation Studio · granted 14 Oct · Revoke". Revoking
  records `direction_grant_revoked` and takes effect for the next change.
- **Team ownership is the larger grant.** A Request about a team nobody owns offers one more
  answer: **"Make Bianca owner of this team"**. It reuses the existing `restless teams owner`
  record, and from then on Bianca's direction for that team is Direct. The People page's team
  "Owned by" picker stays the main place to change it.

### 4. One digest instead of many interruptions

- The owner's Inbox gains a **Today** card at the top. It is not an item, so it holds no
  decision. It says, for example, "Exec applied 4 changes for others · 2 requests waiting". Each
  part links to its list, and Applied entries can be undone from there.
- **Requests stay individual Inbox items**, because each one is a decision. **Applied entries
  roll up** into one For you row per day ("4 changes applied for Bianca and Bodhi"), which expands
  to the rows, each with Undo.
- **Email:** when the owner has email notifications on, the same digest goes once a day at their
  morning (company time zone), through Fleet's existing notification delivery. Requests older than a
  day are listed first. No email is sent on a day with nothing in it.

## Doctrine change for Exec

Replace the "proposal" sentences in "People own teams" with:

> When a person directs a team someone else owns, or the company, classify the change:
> - **Direct:** the person owns that team, or is the owner. Apply it.
> - **Act, then tell:** the change is reversible and in a class a grant or the default allows. Apply it,
>   then `restless direction applied ...` with its undo handle, and tell the person in one line that
>   it is done and who can undo it.
> - **Ask first:** anything else. `restless request file ...`, and tell the person in one line who
>   you asked. Do not explain the rule to them.
>
> Never refuse in paragraphs. Never apply an Ask-first change before its approval.

## Contracts

### Authority records

All are append-only in the governance store, like approvals and mandates.

| Kind | Body |
|---|---|
| `direction_applied` | `id, requester, source_message, class, team, summary, target{kind,id}, undo{kind,...}, routed_to` |
| `direction_undone` | `applied_id, by, result: undone \| changed_since` |
| `direction_request` | `id, requester, routed_to, class?, team?, summary, change, reason, source_message` |
| `direction_decision` | `request_id, by, decision: approve \| decline, reason?, grant_id?, team_owner?` |
| `direction_grant` | `id, grantee: <person> \| members, class, scope: company \| team:<id>, granted_by, source` |
| `direction_grant_revoked` | `grant_id, by` |

### Exec CLI

- `restless direction applied --class --team --summary --target --undo <json> --source-message`
- `restless request file ...` (above)
- `restless request show <id>`
- `restless direction grants` lists active grants, so Exec can pick the path.

### Owner and person API

All are under `/api/companies/{c}`.

- `GET /direction/requests?state=open|mine`
- `POST /direction/requests/{id}/decision {decision, reason?, always_allow?, make_team_owner?}`
- `GET /direction/applied?since=`
- `POST /direction/applied/{id}/undo`
- `GET /direction/grants` and `POST /direction/grants/{id}/revoke`
- `GET /direction/digest?day=`

Each endpoint is readable or decidable only by the person a request is routed to and the owner.
Members see only their own requests.

### Attention

- A routed `direction_request` is an Inbox item, `source.kind = direction_request`, with the three
  buttons.
- `direction_applied` entries become For you items, `sheet`/`doc`-style, with target and undo.
- The digest card is computed from the same rows.

## Not in scope

- A policy language, thresholds or spend tiers (`approval.rs`).
- Changing who may approve: the team's person or the owner, as for judgement.
- Grants for external effects. Email mandates remain the only standing permission for outbound
  email.

## Acceptance (smoke and unit)

1. A member asks for a content rule on an unowned team.
   - The brief changes at once.
   - The owner's For you shows "Applied for <member>" with Undo.
   - Undo restores the brief and tells the member.
2. A member asks for a spend change. A request appears in the owner's Inbox. Approve carries it
   out, and the member's rail card turns Done.
3. A member asks for a staffing change; the owner chooses "Approve, and always allow".
   - The member's next staffing change applies at once as Applied.
   - After Revoke, the next one asks again.
4. A request about an unowned team, answered with "Make Bianca owner", makes her next direction
   for that team Direct.
5. Two admins approve one request at once. One wins, and the other sees "already approved".
6. With an applied change and an open request, the digest says "1 change applied · 1 request
   waiting", and the email digest is sent once.
7. A member never sees another member's requests, and is refused the grant and undo endpoints.

## Sprint 80 split

- **Core session (engine, orgintel, Authority, Exec):**
  - the records;
  - the CLI;
  - the API with its role checks;
  - the attention projection;
  - undo for the four handle kinds;
  - the doctrine change;
  - acceptance checks 1 to 7 in the multiplayer smoke.
- **Cloud session (web and Fleet):**
  - the Inbox request item and its keys;
  - the rail request card;
  - "Your requests";
  - For you Applied rows with Undo and the daily roll-up;
  - Standing permissions in Company → Members;
  - the Today card;
  - the email digest through Fleet notification delivery.
- Order: the contracts above land first as types, with fixture responses, so both halves build in
  parallel.
