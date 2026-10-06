# Sprint 63 — Apps: one place for everything that gives the company an ability

**Status:** draft for founder alignment
**Programme:** company extensibility (connectors, tools, plugins)
**Depends on:** Sprint 57's one gateway and Connections page (ADR 0014), Sprint 55's skill library and
candidates, S57-T7 (connection proposals as prepared handoffs, open), Sprint 61 Tier A (the
connections smoke) as the regression net under the move. **Interacts with Sprint 62**, which
restores Company's section navigation and moves Account → Connections and AI apps under a settings
"Integrations" group. Those are account-level model connections and stay where Sprint 62 puts them;
this sprint removes Connections and Skills from Company's section list, so land after Sprint 62's
Company navigation or edit the same list once
**Mockups:** [Apps canvas](https://claude.ai/artifact/EtK7CJKxAQR2uLT7tZdinQ): row A (the Apps
surface, an app page, the grant step, mobile) and row C (Exec brings the app to the Inbox). Row B,
Linear's settings pattern, was considered and not chosen.

## Outcome

An owner who has never heard of MCP, a CLI, a skill or a plugin opens **Apps** in the top
navigation. They see three things:

- what Exec recommends adding, and for which waiting Work;
- what the company already uses, and whether anything needs them (an expired sign-in);
- a browsable set of popular apps.

They choose **Add**. If the app reaches a service, they sign in with that service, and Exec proposes
in plain language what the company may do with it. If it is know-how, there is nothing to sign in
to. Either way, one confirmation and it is in use. Work that was waiting on it resumes by itself.

When Exec hits a missing ability during Work, the same app arrives in the Inbox as one prepared card
with a single **Add** button. Apps is where the owner browses. The Inbox is where they are asked.

**Product hypothesis.** Four technically different mechanisms (remote and local MCP connections,
skills, plugin bundles, and later CLIs) are one idea to an owner: *an app gives my company an
ability*. Presenting them as one kind of thing, in one first-class place, makes extensibility usable
by a non-technical owner without hiding what each app may do. The VS Code Extensions view is the
reference: a top-level surface for rarely changed, high-leverage configuration, with workspace
recommendations.

## Observed friction

Read on 6 October 2026 from `main` at `79f2704`.

| Friction | Evidence |
|---|---|
| Abilities are split across four technical nouns in two places | Company → Connections (MCP URL, command, plugin Git URL: `connections/+page.svelte` offers `URL`, `Command`, `Plugin`) and Company → Skills, grouped as "Capabilities" beside Intelligence and Vault (`company-pages.ts`) |
| The owner must know the mechanism before adding anything | The Connections add flow asks first for a transport. A plugin's skills reach Skills as candidates, while its servers reach Connections (S57-T9), so one thing the owner added appears in two places |
| Only five tools are suggested, and they are buried | `SUGGESTIONS` in `web/src/lib/model/connections.ts`: GitHub, Linear, Notion, Stripe, Sentry, shown as a row at the bottom of a Company sub-page |
| Exec cannot bring a missing ability to the owner | S57-T7 is open; `source-capability.md` tells Exec to prepare a handoff, but no owner surface renders it as a one-click add |
| The cockpit spec and the working agreement disagree on the primary navigation | `owner-cockpit.md` §3.1 (Core contract) lists four areas: Attention, Work, People, Company. CLAUDE.md lists five: Inbox, Work, Library, People, Company. The shipped shell follows CLAUDE.md |

## Design stance

**Core contract**

- **An app is a presentation, not a new entity.** The cockpit projects one **App** over the records
  that already exist: an Authority connection (ADR 0014), a skill row and its disposition
  (Sprint 55), and built-in effects such as Resend. No new table, lifecycle or install state
  machine. The one concept, one owner rule holds: Authority owns connections and grants; OrgIntel
  owns skills; the cockpit only joins them.
- **The owner sees what an app does, never its transport.** Cards and pages say "Sign in with
  Linear", "Know-how, no sign-in", "Sign-in and know-how". MCP, skill, plugin and CLI appear only in
  tooltips and on a Details disclosure for the curious. Governance keeps its owner words from
  Sprint 57: *Reads freely*, *Acts with a receipt*, *Asks you first*.
- **Probe, never guess.** An app is *in use* only when its live state says so: a connection whose
  last probe passed, a skill that is accepted. An entry in the popular list is a known address, not
  a claim that it works. The verified mark appears only from a passing Tier B canary (Sprint 61, named
  next). Until then, no app shows it.
- **Adding stays governed exactly as today.** Sign-in, grant classes, first-contact approval, skill
  acceptance and freeze are unchanged. Apps changes where and how they are presented, not who may
  do what.

**Default pattern**

- Three sections, in this order: **Exec recommends**, **In use**, **Browse**. Recommendations lead
  because the product runs the business for you; browsing is the fallback.
- The **Apps** tab carries a dot when something needs the owner, such as an expired sign-in or a
  recommendation for blocked Work. The count belongs to the Inbox; the dot only says "look here".
- Browse is a short curated list of popular apps, by job (Communication, Engineering, Finance,
  Sales, Commerce, Documents), plus **Add from a link** for anything else. One link field accepts a
  service's MCP address, a GitHub link to a plugin or skill, or a local command, and Restless works
  out which it is.
- An imported plugin is one app. Its connections and skills are grouped under it on the app page.

## Navigation change (amends the working agreement and a Core contract)

The top navigation becomes six surfaces: **Inbox, Work, Library, People, Apps, Company**.

- `CLAUDE.md`, primary experience: "the five surfaces — Inbox, Work, Library, People, Company"
  becomes "the six surfaces — Inbox, Work, Library, People, Apps, Company". The rest of the
  paragraph stands: navigation only, never an agent-administration dashboard.
- `owner-cockpit.md` §3.1 (Core contract) is rewritten to the six shipped areas, which also
  corrects its missing Library and its *Attention* name for the Inbox. Apps is described as "what
  gives the company abilities: services it can use and know-how it can apply, with what each may do".
- Company keeps what configures the company itself: Charter, Identity, Members, Intelligence
  (model providers and harnesses), Vault, Schedules, Limits and Computer. Connections and Skills
  move out.

These amendments land in the same change as the navigation, after founder alignment, not before.

## Scope by layer

### Owner cockpit

- Route `/<company>/apps` (the surface) and `/<company>/apps/<app>` (the app page). The old
  `/company/connections` and `/company/skills` redirect there. The OAuth callback path is unchanged
  (Sprint 61 T1: the router already routes it).
- `companyShellTabs` gains Apps for owners, with its keyboard shortcut and dot. The mobile dock gains
  Apps.
- The app page follows mockup A2: what the company may do with each tool (class switches), recent
  receipts, account, who may use it, Freeze and Disconnect. For a skill app: what it teaches, when it
  was last used, and Remove. For a plugin: its parts.
- The grant step follows mockup A3: Exec's proposal grouped by class, who may use it, and **Allow**,
  which names the Work it resumes.
- The Inbox renders an app recommendation as mockup C: why, what it will be allowed to do, **Add**.
- `COMPANY_PAGES` loses Connections and Skills, the Company overview loses their rows, and the
  command menu gains "Add an app" and one entry per popular app (`add slack`).
- Copy review: no MCP, CLI, skill or plugin in visible text outside tooltips and Details.

### OrgIntel

- S57-T7, finished here: Exec can **recommend an app** for Work it accountably owns. This is an
  ordinary owner handoff naming the app (catalogue key or link), why, and the Work it unblocks. It
  renders in the Inbox and in Apps' **Exec recommends**. No new Work state.
- A skill candidate that Exec or Staff added (`restless skill add`) appears as a recommendation,
  not only on the skills list.

### Kernel / Authority

- Record a connection's **origin** (catalogue key, link or plugin) so the cockpit can group a
  plugin's parts as one app. This is a nullable column on the existing connections table, not a new
  record.

### Runtime

- None.

### The catalogue

- Replace `SUGGESTIONS` with a small data file of popular apps: key, name, plain description, job
  category, interface, address and sign-in kind. It is a list of known addresses, not adapters, and
  it ships with the cockpit build. An entry is added only after one live connection to it has
  succeeded on a `_test` company, and that check is recorded in the file's history.
- Start with the services Sprint 57 named (Gmail, Google Calendar and Drive, Slack, Notion, Linear,
  GitHub, Stripe, Shopify, Xero, Sentry), each live-checked or left out, plus the company image's
  built-in skills.

## Acceptance

All live runs use a `_test` company (evaluation-dogfood §9.6.1).

1. **One surface.** Apps shows connections, skills, a plugin and Resend together, each with true
   live state. The old Company pages redirect, and the Company overview no longer lists them.
2. **Add a service.** From Browse, an owner adds a catalogue app with an OAuth sign-in. The grant
   step shows Exec's proposal, Allow puts it in use, and an actor session lists its tools. Observed
   in Core and on a Cloud plane. Sprint 61's smoke still passes after the move.
3. **Add know-how.** An owner adds a skill app; it is accepted and selectable in the composer, with
   no sign-in step.
4. **Add from a link.** Pasting a remote MCP address, a GitHub plugin link and a GitHub skill link
   each produces the right kind of app without the owner choosing a type.
5. **Exec recommends.** In a `_test` company, Work blocked for lack of a service produces one Inbox
   card and one Exec-recommends entry. Adding it resumes that Work without any further owner
   message. This needs a model run, so it is recorded as one dogfood observation, not an automated
   test.
6. **Plain language.** A copy check finds no "MCP", "CLI", "skill" or "plugin" in visible text on
   Apps, app pages, the grant step or the Inbox card, outside tooltips and Details.
7. **Navigation contract.** `CLAUDE.md` and `owner-cockpit.md` §3.1 match the shipped six surfaces.
8. **Mobile.** Apps and an app page work at 390 px, with no horizontal scroll and every action
   reachable without hover.

## Deletable machinery

`SUGGESTIONS`; the Company → Connections and Company → Skills pages and their `COMPANY_PAGES`
entries; the Company overview's Connections and Skills rows; the transport picker
(`URL / Command / Plugin`) in favour of one link field.

## Out of scope (named next)

- **CLI apps.** A pinned, user-space install with a Vault credential and a live probe (`gh auth
  status`). It needs the user-space package decision (`company-runtime.md` §19 #6, mise
  recommended) and Sprint 42's application roles. The mockups show Vercel as a CLI app to test
  whether the mental model holds.
- **Verified marks.** These come from Sprint 61's Tier B canaries.
- **Model providers and harnesses as apps.** They stay in Company → Intelligence. Revisit if owners
  look for them in Apps.
- **Ratings, reviews, publishers and paid apps.** Not a marketplace.

## Risks and dispositions

| Risk | Disposition |
|---|---|
| A sixth tab weakens the calm top bar | **Accepted.** Apps is navigation, not status. It carries a dot, never a count or activity |
| "App" hides a real consequence behind a friendly word | **Guarded.** Every app shows what it may do in the three owner classes before Allow; nothing about governance changes |
| A catalogue entry silently stops working upstream | **Accepted** until Tier B. *In use* reflects the company's own live probe, so the owner is not misled about their own apps |
| Apps becomes a second home for Company configuration | **Guarded.** Only what gives the company an ability moves. Intelligence, Vault and Limits stay in Company |
| Members see apps they cannot add | **Accepted** for V0: Apps is owner-only, like Connections today |

## Open questions for founders

1. Is a sixth top-level surface right, or should Apps replace Company's entry point instead?
2. Does the composer's `$` menu keep saying "skills", or use the app's name and know-how wording?
3. Should the Telegram Attention channel (Sprint 57 T12) be an app, since it is something the owner
   adds and signs in to?

## Tickets

Ticket files are written at breakdown, after alignment. Each will cite the friction above, name its
layer and state what it deletes.

- [ ] T1 — Navigation contract: amend `CLAUDE.md` and `owner-cockpit.md` §3.1; add the Apps tab,
  shortcut, dot and mobile dock entry. *Cockpit, docs*
- [ ] T2 — The Apps projection: one owner read model joining connections, skills, plugin origin and
  built-ins with live state. Includes the connection `origin` column. *Cockpit, Authority*
- [ ] T3 — The Apps surface (Exec recommends, In use, Browse) and the catalogue file, replacing
  `SUGGESTIONS`. *Cockpit*
- [ ] T4 — The app page and the grant step, moved from Connections and Skills; redirects; delete the
  old pages. *Cockpit*
- [ ] T5 — Add from a link: one field that recognises an MCP address, a plugin link, a skill link and
  a command. *Cockpit, Gateway*
- [ ] T6 — App recommendations (finishes S57-T7): an Exec handoff naming an app, rendered in the
  Inbox and in Exec recommends; adding resumes the Work. *OrgIntel, Cockpit*
- [ ] T7 — Acceptance runs 1–8 on `_test`, Core and Cloud; final visual pass against
  `FRONTEND_DESIGN_REFERENCES.md`. *Verification*
