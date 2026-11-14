# Sprint 120: an owner can get in, and we can measure it

**Status: planned, not started (14 Nov 2026 AEDT). Spans restless-core and restless-cloud. Part B
waits on the owner's conversations with small business owners.**

Sprints 116 to 119 make the company move first, ask, repair and learn. This one makes it possible
for a small business owner to come in on Restless Cloud, and gives the owner a number for whether
it feels right. First owners are planned from Mon 30 Nov.

Today, admitting someone is a hand-written database row. A person who is not yet admitted is told
to check their email, and nothing arrives. Nothing suggests a first task. The first run uses words
a café owner will not know. No restore drill is on record. The company's own truth document still
says the lead never produces, which was reversed on 19 Oct. And the measure the product was built
around has never been taken.

Founding lines this restores:

- `docs/specs/owner-cockpit.md` §16.2: measure "owner interventions per accepted outcome".
- `docs/specs/evaluation-dogfood.md` §6.3: the owner intervention taxonomy, where "agent
  chasing/status request" is a sign of product failure.
- `ARCHITECTURE.md` §10.6: the smoke-test scenarios, which "begin as manually observed company
  runs".
- `docs/company-identity/restless-company-truth.md`: "Your company should keep working when the
  chat ends."

## What the owner decided

- The three decisions of 14 Nov (see Sprint 116) govern what a new owner will meet.
- Public content bar (27 Oct): nothing inaccurate under the owner's name. The truth document and
  the README are changed only with the owner's approval of the exact words.
- Releases are cut from main, Core and Cloud together.
- **For the owner to settle in part A.** Whether a person who is not admitted may be told so (see
  the note there).

## What exists to build on

- Cloud's admit path (restless-cloud, read only for this spec): `signupPermitted` in
  `apps/fleet-web/src/lib/server/signup-policy.ts` checks the bootstrap list, then the
  `fleet_signup_allowlist` table (`services/fleet-api/migrations/0048_signup_allowlist.sql`), then
  a pending company invitation. The table takes effect at once, with no rollout.
- Registration answers "Check your email for the verification link." whatever the result
  (`apps/fleet-web/src/routes/account/+page.server.ts`). This is deliberate, so that the form does
  not reveal which emails have accounts.
- The welcome interview (`crates/restless-engine/prompts/welcome-the-owner.md`) and tap answers
  (`ownerReplies`).
- First-run screens: `web/src/routes/[companyId]/welcome/+page.svelte`, `SetupBrand.svelte`,
  `WelcomeTour.svelte`; and `web/scripts/capture-surfaces.mjs`, which already probes for raw ids
  and commands.
- Backup and restore for a self-hosted install (`docs/self-hosting.md`, `restless appliance
  backup`), and Cloud's `scripts/fleet-backup-drill.mjs` with its two sibling drills.
- Decision history and the nudge test (`is_nudge` in `crates/restless-engine/src/org_review.rs`)
  for the attention diary.

## A. One command admits a person (Cloud)

**Problem.** G1.

**Build.**

1. One operator command takes an email and an optional note, adds the allowlist row and sends the
   invite email with the sign-up link. Running it twice is harmless.
2. A person who signs up and is not admitted is told so plainly: they are on the list and will get
   an email when their place is ready. They are not told to check an email that will never come.
3. The Cloud work is ticketed and tracked in restless-cloud's own sprint files. This spec names it
   so both releases are cut together.

**Note for the owner.** The neutral message exists so the form cannot be used to find out who has
an account. Telling a person they are not yet admitted reveals only that the address is not on the
list, not whether an account exists. That looks acceptable, and it is the owner's call, to
confirm during ticketing.

**Done when.** One command, then the invited person signs up from the email and reaches the
welcome interview. A second, uninvited address gets the honest message and no account.

## B. Three starter outcomes

**Problem.** G2.

**Build.**

1. After the welcome interview, Exec offers three starter outcomes as tap answers. Each is a real
   job that ends in something the owner can look at the next morning.
2. The three are slots, left empty here. The owner fills them from conversations with small
   business owners ("what do you keep meaning to do for the business and never get to?").
3. A chosen outcome becomes the company's first goal through the existing "Make this a goal?"
   path, so Sprint 116's daily review picks it up.
4. The owner can ignore all three and type their own. No template library.

**Done when.** A new company's interview ends with three tap answers, and tapping one creates a
goal with Work started under it.

## C. Plain words on the first run

**Problem.** G3. The words are: charter, Vault, API key, compute, revision.

**Build.**

1. Sweep the first-run screens and Exec's first-day messages for those five words. Replace each
   with what it does for the owner, or move it out of the first run. Settled replacements are a
   ticketing decision; the owner signs them off.
2. Add the five words to the `capture-surfaces` text probe for the first-run screens, so they
   cannot come back unseen.
3. Company settings pages keep their names. This is about the first hour, not a rename.

**Done when.** The probe finds none of the five on the welcome path at desktop and phone width.

## D. One restore drill

**Problem.** G4. A backup has to outlive the server it protects, and a restore has to be seen.

**Build.**

1. Restore one real company from a backup onto a fresh host and open it as its owner: chat
   history, Work, Library and the company computer's files.
2. Record where the backups are kept, how old the newest was, how long the restore took and what
   was missing. Keep the record with the release evidence.
3. If backups live only on the server they protect, say so and stop for the owner's decision
   before any owner is admitted.

**To confirm.** Cloud has drill scripts. Whether a full restore has ever been run and recorded was
not verified for this spec.

**Done when.** The drill record exists, with a restored company seen in the browser.

## E. The truth document and the README

**Problem.** F1 and F2.

**Build.**

1. `docs/company-identity/restless-company-truth.md`: `organisation.lead.nonproducing` and
   `organisation.exec.dispatch` are brought in line with what the product does (leads produce
   directly since 19 Oct; Exec does small work itself since 4 Nov).
2. `product.autonomy.proactive` is restated to match Sprints 116 to 119 once they have passed
   their tests, and not before.
3. The README's opening is put beside the truth document's landing line, and the owner chooses
   the pitch. A draft of each is prepared; nothing is published without the owner's yes.

**Done when.** The owner has approved the exact words, and no claim in either file is contradicted
by the audit's findings.

## F. Three days away, with an attention diary

**Problem.** H1.

**Build.**

1. The owner leaves one of their own companies alone for three days, with goals set.
2. Every time the owner steps in, it is logged: when, what, and which of the eight kinds in the
   evaluation spec's taxonomy it was. Owner messages and Inbox decisions are exported; the kind is
   marked by hand. A file, not a feature.
3. The number is interventions per accepted result, with the share that were chasing or status
   requests.

**Done when.** The diary and the number exist and are kept with the release evidence.

## G. Three smoke tests through the real interface

**Problem.** H2. Sprint 118 covers the stalled worker and duplicate work.

**Build.** Run three more from `ARCHITECTURE.md` §10.6 in the browser, as an owner would: a vague
mission, an external purchase (only the purchase waits; internal work continues), and human
takeover of the company computer. Record what the owner saw at each step.

**Done when.** Each has a short run record with pass or fail against the table's expected
behaviour.

## Tickets

| | Ticket | Layer | Closes | Reuses | Makes deletable |
| --- | --- | --- | --- | --- | --- |
| [ ] | 120-A One command admits (Cloud) | Cloud | G1 | `fleet_signup_allowlist`, `signupPermitted` | Hand-written allowlist rows |
| [ ] | 120-B Three starter outcomes | Prompt | G2 | Welcome interview, `ownerReplies`, goals | Nothing |
| [ ] | 120-C Plain first-run words | Owner surface | G3 | `capture-surfaces` | The five words on the welcome path |
| [ ] | 120-D Restore drill | Runtime, Cloud | G4 | Backup and restore, Cloud drill scripts | Nothing |
| [ ] | 120-E Truth and README | Docs | F1, F2 | The audit's claim table | Two claims that are no longer true |
| [ ] | 120-F Away test and diary | Verification | H1 | Decision history, `is_nudge` | Nothing |
| [ ] | 120-G Three smoke tests | Verification | H2 | §10.6 table | Nothing |
| [ ] | 120-H A real person gets in | Verification | G1 to G4 | All of the above | Nothing |

## Slice per layer

- **Kernel.** No change.
- **OrgIntel.** No change beyond the welcome prompt.
- **Runtime.** One restore, observed.
- **Owner surface.** Words on the first run. No new screen.
- **Cloud.** One command and one message, in restless-cloud.

## Cost

No new automatic wake. Starter outcomes add nothing until the owner taps one. The away test and
the smoke tests spend real model turns inside the test company's spend limit.

## Done when

- A non-technical person, admitted with one command, reaches a prepared decision the next morning
  without help from the owner. What they saw is recorded.
- There is a number for interventions per accepted result.
- Live per role on the welcome path: owner and member, desktop and phone, with failure cases (an
  uninvited address, an expired invite).

## Out of scope

- Sending email to customers without the owner's own Resend account (G5); done by hand for the
  first owners.
- A warning before credit runs out (G6), and acquiring missing capabilities without a yes (G7).
- Business vocabulary such as customers, offerings and deliveries (G8).
- The spend "fails closed" claim (F3), character names (F5), and the site claims list (F6), which
  must be fixed before any public post but not in this sprint.
- A full three-way comparison of single agent, loose team and company (H3).
- A measurement dashboard. The diary is a file.
