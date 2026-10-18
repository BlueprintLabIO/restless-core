# Sprint 98: Accounts the owner holds, granted to companies

Owner-approved direction (2026-10-18): "lots of accounts may be convenient to share between
companies and grant access between companies such as GitHub, Facebook, eBay… make any generic
account / identity / secret / profile exist on a per-owner level, grant to companies, and allow
members to use them but not abuse them." Decisions:

- **Flexible, powerful, permissive by default.** The owner decides how an account is used. One eBay
  account selling for several companies at once is a legitimate, supported use.
- **Members may direct agents to use a granted account.**
- **Personal logins are supported**, with one small, quiet notice when one is added: use it within
  the platform's own rules; the owner is responsible for how it is used, and Restless is not
  responsible for misuse.

AI accounts (Settings › AI accounts) are the first instance of this pattern and become one kind
of account in it.

## The model

1. **Account:** an identity the owner holds: a GitHub account, an eBay seller account, a Meta page,
   a Stripe account, a domain registrar, an inbox, any website login, any API. Lives at the owner's
   account level (the local appliance's account, or the Cloud account), never inside one company.
2. **Credential:** how the account proves itself. One account may hold several: an OAuth or app
   install, an API key or token, a username and password with an optional TOTP secret, a signed-in
   browser profile (cookies and storage), a passkey later. The best one available is used for each
   action; the owner can pin one.
3. **Grant:** a company may use an account, with a set of abilities and optional limits. Grants are
   what companies, members and agents get; a credential never leaves the account.

## Use without exposure

Agents and members get the ability to act as an account, never its secret:

- **API and OAuth calls** go through the account broker, which attaches the credential on the way
  out (the model gateway already does this for AI keys; the tool gateway does it for MCP tools).
  Nothing inside a company computer can print, copy or export it.
- **Short-lived tokens** where the platform supports them (a GitHub App installation token per
  task, an OAuth access token per session), scoped to what the grant allows.
- **Logins** run in a managed browser identity: the password and TOTP are filled only on the
  account's own domains (a page elsewhere asking for them gets nothing), cookies stay in the
  identity's profile, and 2FA prompts that need a person (a push, an SMS) come to the owner, or to
  whoever the grant names, as an Inbox item with the code field.
- **Command-line tools** (`gh`, `stripe`, `aws`) get their credential injected per run by the
  broker; the agent sees the tool's output, not the token.

## Abilities and limits

A grant lists abilities in five tiers. Defaults are permissive; the owner can widen or narrow any
grant in one click ("Full access", "Read only") or per tier.

| Tier | Examples | Default for a new grant |
|---|---|---|
| Read | read issues, view listings, check orders, read inbox | allowed |
| Draft | write a post, prepare a listing, open a pull request, draft an email | allowed |
| Act | publish, send, list an item, reply to a buyer, merge | allowed, act then tell |
| Spend | buy, run ads, pay, refund | allowed up to a cap the owner sets when granting; above it, asks |
| Admin | change account settings, add users, change password, delete | off, grantable |

"Act then tell" and "asks first" use the existing authority path (S71 act then tell, direction
grants, owner approvals), so they read and behave like every other approval.

Optional limits on any grant, none on by default:

- **Scope:** these repos, this Page, this store, these recipient domains, these categories.
- **Caps:** actions per day, spend per week or month.
- **Time:** an expiry date; quiet hours.
- **Who:** every member and agent in the company (default), or named people and agents.

## People and agents

- **Owner:** adds accounts, grants them, sees all use, revokes or freezes anything in one click.
- **Company admins** request accounts and wider grants through app requests ("Snake Oil asks to use
  eBay to list and sell: Approve · Approve with limits · Deny"); they cannot grant themselves.
- **Members** direct agents to use any account granted to their company, within the grant.
- **Agents** act on someone's behalf. Every use names who directed it: "Exec, at Bianca's request,
  using Owner's eBay, listed 'Canon AE-1' in Sydney Resale."
- Nobody but the owner sees or exports a secret; the owner can reveal one deliberately (Security
  re-check) and the reveal is logged.

## Sharing across companies

- An account can be granted to any number of companies at once; each company's actions are
  recorded as that company's.
- What a company makes with an account is attributed to it (a listing, a repo, a post) so its
  Work, Library and Activity show only its own; the owner's account view shows everything.
- **Browser identities are shared too.** Concurrent use of one signed-in browser profile is
  serialised by a short lease (one session drives it at a time, others queue for seconds, not
  minutes) so sessions do not log each other out; a stable network exit per identity keeps
  platforms from seeing it jump around. This is a technical safeguard, not a policy limit.

## Accountability (not restriction)

- **Use log:** every use, with company, person, agent, action, target and result, on the account's
  page and in each company's Activity; exportable.
- **Notices, not blocks:** the owner is told of a first use by a new company, a burst well above
  the usual rate, a first spend, or a new destination. One tap: "Fine" or "Freeze".
- **Freeze and revoke:** freeze one grant, one account everywhere, or all outside actions; rotate
  a credential in place without re-granting.
- **Health:** each credential shows Signed in, Expiring, Expired or Refused, and asks the owner to
  fix it before a company hits it.

## Kinds of account at launch

- **AI providers** (migrated from AI accounts, unchanged behaviour).
- **Generic API:** base URL, auth style (header, bearer, query, basic), key; called via the broker.
- **Website login:** sign-in URL and domains, username, password, optional TOTP; used through a
  browser identity.
- **Browser profile:** sign in once by hand in the owner's managed browser; the profile is kept.
- **GitHub** as the first app-native kind: a Restless GitHub App installed on chosen repos or an
  organisation, short-lived tokens per task, plus `gh` injection.
- **MCP tool connections** (S57) become accounts too: connected once, granted to companies, with
  their per-tool decisions as the grant's abilities.

Later, by demand: Google Workspace, Meta Business, Shopify, Stripe, eBay API, X, LinkedIn.

## Personal-login notice

Shown once, small and quiet, when a personal login or browser profile is added, and in the
account's details: "Use this account within the platform's own rules. You are responsible for how
it is used; Restless is not responsible for misuse." No blocking, no repeated warnings.

## Where things live

- Accounts, credentials and grants are owner-level records, stored with the account (the local
  appliance's authority store, or the Cloud account plane), credentials in the vault (Infisical)
  as AI keys are today.
- Company planes hold grants and use records, never credentials. On Cloud, the broker that attaches
  a credential runs where the credential lives, or hands the company a per-use, scoped token.
- The company Vault stays for secrets that belong to one company only.

## Tickets

Core session (`core`): data model, broker, policy, engine.

- **S98-01 Accounts and grants model.** Account, Credential, Grant, Use records at owner level;
  grants mirrored to company planes; AI connections and S57 tool connections migrated with no
  behaviour change. Acceptance: existing AI and tool grants still work after migration; a grant
  revoked at the owner level stops use within one turn.
- **S98-02 Account broker.** One broker for API calls, OAuth refresh and CLI injection, generalising
  the model and tool gateways; `restless account use <account> …` for agents, plus transparent
  injection for configured CLIs. Acceptance: no credential bytes appear in a company computer's
  files, environment or transcripts (checked by a scenario that greps for them).
- **S98-03 Abilities, limits and authority.** The five tiers mapped per account kind; caps, scope,
  expiry, quiet hours, who; Act and Spend wired to act-then-tell and approvals; every use
  attributed to the directing person. Acceptance: a scenario per tier, including a cap crossed.
- **S98-04 Browser identities.** Managed profiles per account; domain-pinned fill for password and
  TOTP; human 2FA relayed to the Inbox; lease serialisation and a stable exit per identity.
  Acceptance: two companies listing on one account in parallel without logging each other out; a
  look-alike domain gets no credential.
- **S98-05 GitHub App.** Install flow, per-repo grants, per-task tokens, `gh` injection.
- **S98-06 Use log, notices and freeze.** Records, rate baselines, first-use and spend notices,
  freeze and rotate. Acceptance: a burst triggers one notice; freeze stops the next use.

Cloud session (`cloud`): owner and company surfaces.

- **S98-07 Settings › Accounts.** AI accounts becomes Accounts: every kind in one list with
  health and the companies it is granted to; Add account by kind (API key, website login, browser
  sign-in, GitHub, MCP tool, AI provider) with the personal-login notice; per-company grant editor
  in place (abilities by tier, "Full access" and "Read only" presets, optional limits, who).
- **S98-08 Company › Accounts.** What this company may use, with abilities and limits in plain
  words; "Request an account" and "Ask for more" through app requests; admins' requests land in
  the owner's Inbox with Approve, Approve with limits, Deny.
- **S98-09 Using accounts in conversation and Work.** Agents name the account and ability they
  used in receipts; a member can say "use the shop's eBay" and the agent picks the grant; Act and
  Spend approvals show the account, the company and who asked.
- **S98-10 Activity and notices.** The account's use log (filter by company, person, ability),
  first-use and unusual-use notices with "Fine" and "Freeze", and one-click freeze or revoke
  everywhere from the account.
- **S98-11 2FA and browser sign-in.** The owner's managed-browser sign-in flow for a browser
  profile account, and the Inbox card for a relayed 2FA code.

## Order

1. S98-01 model and migration, with S98-07 Accounts page on top of it (owner can see and grant).
2. S98-02 broker and S98-03 abilities, with S98-08 and S98-09 (companies can use and request).
3. S98-04 browser identities, S98-11 sign-in and 2FA (logins and profiles).
4. S98-05 GitHub App.
5. S98-06 and S98-10 use log, notices, freeze.
