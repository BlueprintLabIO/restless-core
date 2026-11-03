# Sprint 109 H2: Email integration (design for owner review)

Status: partly built (3 November). Built on A2 (one secret name) and D3 (`restless capabilities`).

Built so far, from production evidence on Reno (Core ede37227):
- A Resend key under any older name (`RESEND_API_KEY`, `resend-api-key`, `Resend`, ...) is the
  sending key without re-entry: bound to `resend.production` on load, resolved from a bound or
  unbound Resend name otherwise (#588).
- Without a mandate permit, `restless email send` puts the exact email in the owner's Inbox; Send,
  Ask for changes and Decline wait out Authority's Undo window; once approved it is sent once (#589).
- Company › Integrations › Email shows the observed status (`not_connected`, `key_only`, `ready`,
  `failing`) that `restless capabilities` gives agents, and the owner can send a test (#590).

Not built, waiting on open questions 1 and 2: domain setup, DNS records, the background checker,
`domain_pending`, the unverified-domain send guard, Cloudflare apply, and Receive replies.
Triage references are to `restless-triage-2026-10-28.md` (#2, #3, #11, #12, #15, #53, #54, and
improvement 4). The owner approved an Email preview layout on 29 October; this document is the
concrete behaviour behind it and should be reviewed against that preview before building.

## The owner problem

On 27 October the owner tried to let Reno send email. What went wrong:

- The Resend key was saved as `resend`, but sending looked for `resend.production`, and secrets
  could not be renamed, so the key was stuck (#2, #12). A2 fixed the name and added Rename.
- Exec said Resend was "bundled", then said there was no account (#3). Agents had no ground
  truth. D3 added the capabilities read; it still only knows "key present", not "domain works".
- There is no setup screen. Domain, DNS records, a test send and sending limits are improvised by
  agents in chat (#15), and key requests arrived as cards that asked for a key already in the
  Vault (#53, #54).

What the owner wants: Email is something you connect once, in one place, with a clear status,
and the agents read that status instead of guessing.

## What exists today (so the design reuses it)

| Piece | Where | State |
| --- | --- | --- |
| Send key name `resend.production`; `resend` migrates to it | `credential.rs` (`RESEND_SEND_BINDING`, `canonical_binding`, `migrate_bindings`) | Built (A2) |
| Webhook signing secret `resend.webhook`, never resolvable by agents | `credential.rs`, `ingress.rs` (`POST /inbound/<company>`, Svix-signed) | Built |
| Typed, mandate-gated send (`restless email preview/send`), observe (`restless email observe`) | `email.rs`, `crates/restless/src/main.rs` | Built |
| Outbound mandate: purpose, audience, sender, daily/total caps, expiry, first-contact rule | `docs/exec-email-mandates.md`, `EmailMandates.svelte` in Company › Resources | Built |
| Inbound receipts become messages routed to the right person | `inbound.rs` | Built |
| Capabilities: "Email (Resend)" built, connected when the key is present | `platform_capabilities.rs` (`INTEGRATIONS`) | Built (D3), key-only |
| Key requests answered into the Vault under the service's name | `owner_vault.rs` `answer_key_request`, `InlineVaultKey.svelte` | Built (B6) |
| Cloudflare DNS, domains | `INTEGRATIONS` | Listed, not built |

H2 adds the missing middle: a setup flow, a domain status, a test send, an honest health line,
and one Inbox item when it breaks. It does not add a second send path or a second limit system.

## User journeys

### 1. Owner connects email for a company (phase 1)

1. Company › Integrations › Email › **Connect email**.
2. **Create a Resend key.** The step says exactly what to click in Resend: API Keys › Create,
   permission *Sending access*, domain *the one you'll send from*. Paste it. Restless saves it to
   the company Vault as `resend.production` (A2 name, fixed by Restless, not editable here).
   If a `resend.production` key is already in the Vault, this step is shown as done with
   "Use the key already in the Vault" and a Replace link. No duplicate request card (#54).
3. **Add your sending domain.** Owner types `mail.renoco.com.au` (a subdomain is recommended
   and pre-filled from the company's domain). Restless shows the records to add.
4. **Add the DNS records.** A table of SPF, DKIM, DMARC (and MX for the bounce path) with a copy
   button on every Name and Value. If Cloudflare DNS is connected for that zone, one button:
   **Add these records in Cloudflare**, which shows the exact changes before applying them.
5. **Check.** Restless checks the records (public DNS and Resend's domain status) and shows each
   one as found / not found yet / wrong value. It keeps checking in the background for 72 hours;
   the owner can leave the page.
6. **Send a test.** To the owner's own address by default (any address the owner types is allowed;
   it is the owner's action, not an agent's). Shows "Accepted by Resend" with the message id, and
   "Did it arrive? Yes / It went to spam / It didn't arrive", which adjusts the health note.
7. **Who can send.** Links to the existing email mandate in Company › Resources (purpose, sender
   address, daily and total caps, expiry). Until a mandate is approved, every agent send needs the
   owner's exact-recipient approval, as today.

### 2. Agent sends email (phase 1)

Agents use one path: `restless email preview` then `restless email send`, through the mandate
permit flow that exists today. H2 changes what they check first, not how they send:

- Before saying anything about email, the agent reads `restless capabilities`. If Email is not
  ready, it says why in one line and links Company › Integrations › Email. It never asks the owner
  for a Resend key in chat or raises a key request while the integration is connected (B6).
- The sender address must be on the verified domain. Send refuses an unverified domain before
  calling Resend, with the reason the owner sees on the card.

### 3. Replies and inbound (phase 2)

Owner turns on **Receive replies** on the Email card: Restless creates the webhook in Resend (or
shows the URL `https://<plane>/inbound/<company>` and asks for the signing secret, saved as
`resend.webhook`), shows the MX record for receiving, and a "Send a test reply" check. Inbound
routing and the "a reply is not outreach" rule already exist; phase 2 is setup and status only.

## Screens (text mockups)

Company › Integrations is a new section in the Capabilities group (beside Intelligence, Accounts,
Vault). Its first cards: Email (Resend), DNS (Cloudflare), Domains, Payments (Airwallex, linking
to Finance). Every card is the same fixed height; detail opens in the right pane. Status words are
the same words the capabilities read and Exec use.

**Not connected**
```
Email                                                     Not connected
Send and receive company email through Resend.
[ Connect email ]                         Uses your own Resend account. Free up to 100 a day.
```

**Domain pending**
```
Email                                                  Waiting for DNS
Sending from  mail.renoco.com.au                    Key  resend.production · in Vault
Records       SPF found · DKIM not found yet · DMARC found          Checked 2 min ago
Agents can't send until DKIM is found. Usually under an hour, can take up to 72.
[ Show records ]  [ Check now ]
```

**Verified**
```
Email                                                            Ready
Sending from  hello@mail.renoco.com.au              Key  resend.production · in Vault
Today         12 of 50 sent (mandate cap)           Last test  31 Oct, arrived
Replies       Off · [ Turn on ]                     Who can send  Exec, by mandate ›
[ Send a test ]  [ ... ]                       ... = Replace key, Change domain, Disconnect
```

**Failing**
```
Email                                                     Not sending
Resend refused the key (401). It may have been deleted or rotated.
Agents have stopped sending. 2 emails are waiting.     Since 09:14
[ Replace key ]  [ Show details ]
```
Other failing lines, one at a time, most serious first: "DKIM record is gone from your DNS",
"Resend paused sending for this domain (bounce rate)", "Daily cap reached; sending resumes at
00:00 Sydney", "Resend's free limit reached (100 today)".

**DNS records step**
```
Add these to the DNS for renoco.com.au                     [ Add in Cloudflare ]
Type  Name                              Value                                Status
TXT   send.mail                         v=spf1 include:amazonses.com ~all [⧉]  Found
TXT   resend._domainkey.mail            p=MIGfMA0GCSqGSIb3DQ...          [⧉]  Not yet
MX    send.mail                         feedback-smtp.ap-...amazonses.com [⧉]  Found
TXT   _dmarc                            v=DMARC1; p=none;                [⧉]  Found
```
Values come from Resend's domain response, never typed by us. DMARC is ours to suggest (`p=none`
to start) and is skipped if the zone already has one.

## Where the key lives

- Send key: company Vault, name `resend.production` (A2). One per company. Restless owns the name;
  Vault Rename refuses moving it away while the integration uses it (existing fixed-reference rule).
- Webhook secret: company Vault, `resend.webhook`, only the ingress resolves it.
- Optional setup key (see open question 1): `resend.setup`, full access, used only by the setup
  and status checks inside the host, never by agents, deletable after setup.
- Cloudflare token (when connected): `cloudflare.dns`, scoped to DNS edit on named zones.
- Vault rows show "Used by: Email" for each of these (improvement 4, "used by").

## Capability read output

`restless capabilities` keeps its shape and gains a `state` and a one-line `action`, read from the
stored integration status (no live provider call on read):

```json
{
  "name": "Email (Resend)",
  "built_in": true,
  "connected": true,
  "state": "domain_pending",
  "key": "resend.production",
  "domain": "mail.renoco.com.au",
  "sender": null,
  "receiving": false,
  "limits": { "mandate_daily": 50, "sent_today": 0, "provider_daily": 100 },
  "action": "Add the DKIM record for mail.renoco.com.au (Company › Integrations › Email)",
  "checked_at": "2026-11-02T09:14:00Z"
}
```

States: `not_connected`, `key_only` (key present, no domain set up; today's case), `domain_pending`,
`ready`, `failing`. Exec states facts about email only from this row (D3).

## Failures and Inbox items

One owner Inbox item per integration problem, amended in place, never repeated (B3). It closes by
itself when the next check passes.

| Problem | Inbox item (owner words) | Agents |
| --- | --- | --- |
| Domain not verified after 24 h | "Email isn't ready: DKIM record not found for mail.renoco.com.au" · Show records | Can't send; say so once |
| Not verified after 72 h | Same item, amended: "Resend stopped checking. Check the records, then Check again" | Can't send |
| Key refused (401/403) | "Resend refused the email key" · Replace key (inline, saves as `resend.production`) | Stop; queue, don't retry |
| Records removed later | "Email stopped: your DKIM record is gone" | Stop |
| Provider paused / bounce or complaint threshold | "Resend paused sending for mail.renoco.com.au" + link to Resend | Stop |
| Daily cap reached (mandate or provider) | No Inbox item. A line on the card and the Work it blocks: "waits until 00:00" | Wait |
| Send outcome unknown | Existing mandate behaviour: reconcile before any new outreach | Existing |

A failed send never becomes a key request card. A key request for Resend while the integration
is connected is refused at creation and points to the Email card (B6).

## Permissions

| Action | Owner | Admin | Member | Agent |
| --- | --- | --- | --- | --- |
| See the Email card and status | Yes | Yes | Yes (no key names) | Yes, via capabilities |
| Connect, replace key, change domain, disconnect | Yes | No (Vault is owner-only today) | No | No |
| Add DNS records via Cloudflare | Yes | No | No | No |
| Send a test | Yes | Yes, to their own address | No | No |
| Approve the mandate, change caps | Yes | No | No | Propose only |
| Send email | Via the app | No | No | Within the mandate, or exact owner approval |

## What agents can and can't do

Can: read status; preview and send within an approved mandate (sender on the verified domain,
caps, expiry, one first contact per recipient, replies to people who wrote in); read observe
metadata; file a Restless fault when the integration misbehaves (D3).

Can't: see any key value; create, rotate or rename the key; change DNS; send from an unverified
domain; exceed caps; first-contact without the mandate or an exact approval; raise a Resend key
request while connected; tell the owner email "is bundled" or "works" unless the row says `ready`.

## Data model and API changes

- **Authority table** `restless_authority.company_integrations`: `company, kind ('email.resend'),
  state, domain, provider_domain_id, sender, receiving, records JSONB (type, name, value, found),
  last_checked_at, last_error, last_test (message id, at, owner answer), attention_id`. One row per
  company and kind. No secret values.
- **Status checker** in the engine: on connect, then every 10 min for 72 h while pending, then
  hourly and after any send error. Public DNS lookups plus Resend domain status; bounded, no model
  calls, nothing for companies without the integration (A0.2 idle rule).
- **Owner API** (owner_vault/owner routes): `GET /companies/:c/integrations`,
  `POST .../email/connect {key}`, `POST .../email/domain {domain}`, `POST .../email/check`,
  `POST .../email/test {to}`, `POST .../email/cloudflare-apply {zone}` (returns a diff first,
  applies on confirm), `DELETE .../email`.
- **Send guard** in `email.rs`: refuse when the row is not `ready` or the sender's domain differs.
- **Capabilities**: `integrations()` reads the row for `state`, `domain`, `limits`, `action`.
- **Web**: new `company/integrations` page and `IntegrationCard` (fixed height, no-jank rules);
  `company-pages.ts` entry; Vault "Used by" column; key-request guard in B6.
- **Catalogue**: DNS (Cloudflare) becomes `built: true` only when phase 1's record-apply ships.

## Phased plan

1. **Phase 1: send and domain.** Connect, domain records, copy buttons, status checker, test send,
   card states, one Inbox item, capability state, send guard, Cloudflare one-click (if open
   question 2 says yes). Verified live as owner, admin, member on a local stack and a real domain.
2. **Phase 2: inbound.** Receive replies toggle, webhook creation or guided paste, MX for
   receiving, test reply, "Replies on/off" on the card and in capabilities.
3. **Phase 3: other providers.** Same card and states for Postmark, SES or SMTP if asked; the
   table's `kind` and the capability row already allow it. Not designed further now.

## Open questions for the owner

1. **Setup key.** A Resend *sending-only* key cannot add a domain or read its status. Options:
   (a) you paste a full-access key once for setup and checks, kept as `resend.setup`, never usable
   by agents; (b) you add the domain in Resend yourself and Restless only checks public DNS and a
   test send; (c) one full-access key for everything. Recommended: (a). This also decides whether
   today's `restless email observe` keeps using `resend.production`.
2. **Cloudflare one-click in phase 1?** It needs a Cloudflare token with DNS edit on the zone.
   Build it with Email, or ship copy buttons first and Cloudflare as its own integration next?
3. **Default sending domain.** Recommend a subdomain (`mail.yourdomain`) so marketing mail can't
   hurt your main domain's reputation. Agree, or default to the root domain?
4. **Who pays for Resend.** Each company uses the owner's own Resend account (as today), or
   should Restless offer a shared, metered sender later?
5. **Admins.** Should an admin be able to connect or replace the email key, or stays owner-only
   like the Vault?
6. **Limits shown.** The card shows the mandate cap and Resend's plan limit. Do you also want a
   Restless-wide default cap (for example 50 a day) when no mandate is set, or keep "exact
   approval per email" as the only fallback?
7. **Test send recipients.** Any address you type, or only addresses of company members?
