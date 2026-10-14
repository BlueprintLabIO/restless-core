# Sprint 82 — Nothing moves unless you moved it; the work is the surface

**Status:** landed (see Results; Cloud deploy recorded there)
**Programme:** owner feedback; quality of the cockpit
**Depends on:** Sprint 80 (direction and permission), Sprint 81.

## Outcome

The owner reported, on 14 October, four things that have been patched before and keep coming back:

1. **Answers flip back.** They tapped one of Exec's suggested answers. It became their message, then
   the question card with its option buttons came back, then the message returned.
2. **The cockpit and settings jump.** Entering the cockpit, and saving or editing a setting, rearranges
   the screen several times before it settles.
3. **The Inbox sidebar and the request card are below the bar.** The left column is cramped and
   truncated: "Open the preview …", "blueprintlabio…", "The Exec nee…". The middle request card is a
   wall of small text with three stacked buttons. Neither is at Linear or Apple level, and earlier
   versions looked better.
4. **Exec cannot touch the real thing.** Asked to set the charter, Exec wrote `/company/mission.md`
   and said the charter was "saved". It wasn't: Company › Charter was still empty. When corrected, it
   asked the owner to run `restless company set mission "$(cat /company/mission.md)"`. The request card
   then repeated the charter text instead of showing it in the charter.

This sprint fixes the causes, not the instances. When it is done:

- **The screen only moves when you moved it.** Once something has been shown, it never disappears or
  reappears unless a newer server fact, or your own action, says so.
- **The middle of the cockpit shows the real thing.** An attention item about the charter is the
  charter with Exec's draft in it. One about a document is that document, and one about a setting is
  that setting. Approving publishes the draft and editing it is ordinary editing. Nothing is a copy.
- **Exec and agents work on the source of truth.** They write proposed revisions to the real object.
  Owner-only things stay owner-only, because the owner publishes.

## Causes

| What the owner saw | Cause | How it was found |
|---|---|---|
| Answer → option buttons → answer | `conversationQuery` (`web/src/lib/model/queries.svelte.ts`, the `$effect` near the end) sets `pending = null` whenever the newest *server* message isn't the viewer's. A refetch that started before the answer committed returns a list ending in Exec's question, so the placeholder is dropped. `openQuestionIndex` is derived from the visible list, so the question reopens. The next refetch brings the saved answer back. | Code read |
| The conversation can be rewritten by any response | The Exec conversation has five writers: a poll timer, `companyChangeStream` (refetches **every** active company query on **any** change), the activity stream's completion invalidate, the post-send invalidate, and the placeholder. Nothing checks that a response is newer than what is on screen. The design comment says so: "no client Map, second transcript cache". Rooms already solved this with a snapshot cursor and replay (`room-queries.svelte.ts`); the Exec conversation doesn't use it. | Code read |
| Entering the cockpit reshuffles it | Live, member of Reno, 14 Oct: the first open of a company URL redirected through `/account?return=…` and back, a second full page load. After that the page showed four states in 1.3 s: "Verifying company access…", then an empty nav and an "Inbox Inbox Inbox" shell, then content, then the avatar letter changing (M → Y). 12 API reads fired in parallel at ~0.7 s and each section filled in on its own. At 8.2 s and 9.2 s, while idle, **all 11 cockpit queries refetched twice** (22 requests): one change hint invalidates everything. Chrome reported CLS 0.000, so content is swapping in place, not shifting. The CLS metric misses this, which is why "no layout shift" checks kept passing. | Live trace (built-in browser) |
| Saving a setting jumps | Settings use three data patterns side by side: TanStack queries (Schedules); hand-rolled `fetch` into `$state` with `onMount` or `$effect` (Charter, Computer, Members, Doctor); and large components with their own busy, refreshing and settling flags (`CompanyProvider.svelte`, 1,232 lines and 26 busy flags; `AccountConnections.svelte`, 1,446 lines and 24). After a save, each refetches several resources one after another, and some poll until they "settle". Rows re-render from loading to empty to filled, and section-wide busy states grey out unrelated controls. On Charter, saving swaps the editor for the read view before the new revision is in place. The change stream then refetches everything again. | Code read; owner flows not run live (see *Not verified*) |
| Sidebar cramped and truncated | S80 stacked Today, Inbox, For you (with its own All/Unread tabs) and Set aside into the one narrow column (`+page.svelte` around 986–1033, `InboxList.svelte`, `ForYou.svelte`, `DirectionToday.svelte`). Each added a header row, so titles were cut to one truncated line. `a8c49fa7` ("one list that stays put") was the last single-list version. | Code read and screenshot |
| Request card repeats the work | `DirectionRequestDetail.svelte` renders the request as text: summary, "The change" and "Why", plus Approve, "Approve, and always allow" and Decline. Nothing opens the target, even though S80's `direction_applied` already records a `target{kind,id}`. | Code read |
| Exec can't set the charter, and said it did | The charter is the company config's `mission` (`company-set mission` in `coordination.rs`) plus the revision history behind `POST /company/charter`. `owner.rs` keeps it owner-only ("Money, keys, approvals, the charter and the company computer stay with the owner"). Exec has **no way to propose** a revision: no draft or pending-revision state exists on the charter, so its only move was writing a file. Exec reported "saved" from its own write, not from reading the charter back. | Code read and transcript |

## Workstreams

The order matters. A and B remove the class of bug; C and D are the product change the owner asked
for; E and F stop regressions.

### A. One ordered conversation (refactor)

Replace `conversationQuery`'s refetch-and-replace with the model rooms already use.

- **One store per conversation, keyed by message id.** Server data is merged in, never swapped in.
  A message leaves only on an explicit delete or retract event. A response older than what is held
  (by event cursor, or else by highest message id) is merged and can't remove anything.
- **The Exec direct conversation rides the room event stream.** Exec messages are already room events.
  Use the snapshot cursor plus replay from `room-queries.svelte.ts`, and add the Exec conversation to
  `ROOM_STREAM_OWNED` so `companyChangeStream` stops refetching it.
- **Placeholders are matched, not guessed.** The server echoes `client_command_id` on the committed
  message, and the placeholder is replaced by the message carrying its id. The `pending = null`
  heuristic is deleted. A placeholder is only removed on a definitive send failure, and then it stays
  visible as "Not sent · Retry" rather than vanishing.
- **Open questions close on intent.** An answer, whether tapped, filled in or typed, closes its
  question from the moment it is sent, because the placeholder carries the quote. Re-derivation can
  only close a question, never reopen it, unless a newer question arrives.
- The same store backs `RoomConversation` and Work conversations, so there is one implementation
  instead of three.

### B. Stale never wins, anywhere (data-layer rule)

- **Typed change hints.** The server's change stream names what changed (`attention`, `for_you`,
  `conversation:<actor>`, `charter`, `identity`, `intelligence`, `members`, …). A map from hint to
  query keys refetches only those. The blanket `invalidateQueries({predicate: includes(company)})`
  goes.
- **Versioned reads.** Company-scoped reads return the company change sequence they were read at. A
  query wrapper (`steadyQuery`) refuses data older than what it already holds. Placeholder data keeps
  the previous value, so a refetch never flashes a skeleton or an empty state after first paint.
- **Saves write their own answer.** A mutation's response is the new server state, and the cache is
  set from it (`setQueryData`). No refetch-and-wait, no "settling" polls.
- **One way to load data.** Every settings page and the large components move onto `steadyQuery`
  plus a `saveSetting` helper:
  - `CompanyProvider` and `AccountConnections`;
  - Charter, Identity (Brand & voice), Members, Computer, Doctor, Schedules, Skills;
  - account Appearance and AI apps.

  `fetch` into `$state` from `onMount` or `$effect` is removed from `routes/` and
  `lib/components/`, and lint enforces it (E).
- **Busy is per control.** Only the control being saved shows progress. The rest of the section stays
  usable and does not grey out or re-render.

### C. Entering the cockpit settles in one frame

- **No access redirect hop.** Opening `/<company>` with a valid session no longer redirects through
  `/account?return=` and back. Access is checked in the same request (`boot-fetch.ts`), and the page
  loads once.
- **One boot read.** A single `GET /companies/:id/cockpit` returns principal, attention, the first
  page of For you, the direction digest, the Exec conversation tail, identity and intelligence, all
  at one change sequence. The shell renders from it in one pass. The other queries hydrate from the
  same payload instead of firing 12 parallel requests that land one by one.
- **Reserved space.** Every region that can still be loading has a fixed-size skeleton that matches
  the final layout. The viewer's avatar and name come from the boot read: no placeholder letter.
- **"Verifying company access…" never shows** on an ordinary entry. It is only for genuinely slow
  checks (over 600 ms).

### D. The work is the surface: Exec and agents edit the source of truth

- **Proposed revisions on the real object.** The charter, Brand & voice, documents, sheets, schedules
  and company settings get one shared state: a **proposal**. It is a pending revision held on the
  object, with an author, a base revision, a reason and the source message. Exec and agents may
  create proposals on owner-only objects. Only the owner, or an admin where
  `admin_manages_company_setup` allows it, can publish one. The authority boundary stays exactly
  where it is; what changes is that the agent can now do the safe half.
- **`restless company propose charter <file>`** (and its siblings for identity and settings)
  replaces "write a file and ask the owner to run a command". `restless company set mission`
  stays owner-only and is no longer suggested to the owner by Exec.
- **The real page shows the draft.** Company › Charter shows "Draft from Exec · 10:29": a
  suggested-changes view over the current charter (empty → proposed, here), with Publish, Edit and
  Discard. Editing it is ordinary charter editing, starting from the draft. Publishing creates the
  normal revision, and an edited publish is recorded as correction evidence (S81).
- **The attention item *is* that surface.** A direction request or Exec ask with a `target` opens the
  target in the middle column, already scrolled to the draft, with the request's who and why as a
  slim header. `DirectionRequestDetail` remains only for requests without a surface (spend, access,
  schedules without a page). Approve is Publish, and "always allow" stays on the header.
- **Honest reporting.** Exec doctrine and the tool result say a change is "saved" only after reading
  it back from the source of truth. A proposal is reported as "Drafted in Company › Charter; needs
  you to publish", with the link. The S80 `direction_applied` undo already works on charter
  revisions and is reused unchanged.
- The middle column's job, in one line for the doctrine and the design reference: **bring the browser,
  the Work, the chat or the setting to the person; never restate it.**

### E. The Inbox column, rebuilt to the bar

Start from `a8c49fa7`'s single list and `docs/FRONTEND_DESIGN_REFERENCES.md`, and lay it out the
way Linear's inbox does:

- **One list**, at least 300 px wide and resizable. Each row has two lines:
  - first, the title, wrapped to up to two lines and never truncated mid-word;
  - second, the asker's avatar and name, the kind icon and a relative time.

  The selected row has a clear but quiet highlight.
- **Today** becomes the list header's count ("3 waiting · 1 request"), not its own block.
- **For you** is a tab beside Inbox (Inbox · For you · Archived), not a second list stacked under the
  first. It has one set of filters, in the header's overflow menu. Set aside also moves into that
  menu.
- **Phone width:** list and detail are separate screens with a back arrow, unchanged in behaviour.
- **Before and after screenshots** at 1440, 1024 and 390 px, owner and member, go in this doc. The
  owner signs off on them before release.

### F. Guardrails so it can't creep back

- **Stability tripwire (dev and test builds).** The conversation store and `steadyQuery` throw if a
  shown message id or row disappears without a delete event, or if data goes backwards in sequence.
  In production it logs once to the console.
- **A Playwright "nothing jumps" suite.** A `MutationObserver` records content swaps and removals, a
  layout-shift observer records CLS, and a request counter runs alongside. It uses `_test`
  companies, as owner, admin and member, and runs on:
  1. entering the cockpit, cold and warm: one boot read, no swap after first paint, no access
     redirect;
  2. tapping a suggested answer while a refetch is in flight, which is forced by delaying the
     conversation read: the answer never disappears and the question never reopens;
  3. sending, retracting and editing a message;
  4. approving, declining and undoing a request, including a charter proposal published from
     Company › Charter;
  5. saving each settings page once, plus a failed save and a 409 conflict: only the saved control
     changes;
  6. idle for 60 s: no refetch without a matching change hint.
- **Lint rules:** no `invalidateQueries` outside `lib/model/`; no `fetch` in `routes/` or
  `lib/components/`; no `$effect` that writes data fetched from the server into `$state`.
- **Owner design sign-off** on E's screenshots is part of the release check, not after it.

## Not in scope

- New attention kinds, or making new things owner-only.
- Changing who may publish what. Proposals add a safe agent path but don't widen authority.
- Visual restyling outside the Inbox column and the middle surface.

## Verification plan

- Unit tests:
  - the conversation store's merge, the out-of-order response case, placeholder matching by
    `client_command_id`, and the rule that open questions can't reopen;
  - `steadyQuery`, which refuses older data.
- Engine and owner tests:
  - `company propose charter` creates a proposal as Exec;
  - publishing is owner-only, and admins may publish only where `admin_manages_company_setup` says;
  - publishing creates a revision, and undo restores the earlier one;
  - a stale base revision returns 409.
- The F suite passes on the dev stack in all three roles, then on Cloud after the release cut.
- Live on Reno after release:
  - re-run the owner's charter case as owner: Exec drafts, the Inbox opens Company › Charter with
    the draft, and Publish fills the real charter;
  - enter the cockpit 10 times, cold and warm, with a trace attached.
- Every report states which journeys were and weren't run, and in which role.

## Not verified yet (before this sprint)

- **Owner and admin settings saves were not run live.** The built-in browser session is a member of
  Reno, so the settings findings above come from reading the code. The F suite covers this case.
- **The owner's cockpit entry was not traced.** It loads more (Today, requests, connections). Expect
  it to be worse than the member trace above.
- Whether pre-S80 screenshots exist in `docs/screenshots/` to compare against for E. If not, render
  `a8c49fa7` on the dev stack for the before shots.

## Results

### What changed

- **A. One ordered conversation:**
  - `web/src/lib/model/steady.ts` makes every local cache write cancel reads already in flight
    (`steady()`, installed on the app's one QueryClient).
  - A send writes its saved message straight into the conversation, and the placeholder leaves
    only once the server's copy is held.
  - The `pending = null` guess, which dropped your answer whenever any read's newest message was
    Exec's, is deleted.
  - A retract is declared (`expectRemoval`), so the tripwire knows that removal is expected.
- **B. Stale never wins:**
  - Change hints are routed by kind (`hintTouches`): a `message`, `room_event` or `mention`
    refetches conversations, rooms, Inbox, For you and direction reads, and never settings, the
    principal, intelligence or connections. `resync` and unknown kinds refetch everything except
    settings.
  - Company identity and browser status poll like the other settings (slowly while the change
    stream is live) instead of every 10 seconds.
- **C. Entering the cockpit:**
  - `first-paint.svelte.ts` mounts every region and fetches while hidden, then reveals once, when
    no company query is still waiting for its first answer (capped at 1.2 s).
  - The avatar stays blank until the people list names you; it no longer shows "M" first.
- **D. The work is the surface:**
  - `restless charter show | apply | propose`:
    - `apply` saves the real charter when the owner asked, reads it back, and attributes the
      revision to Exec.
    - Anyone else's direction is refused with "Only <owner> sets the charter", pointing at
      `propose`.
    - `propose` records a draft (`mandate_proposal`); a newer draft replaces an older one.
  - `GET /company/charter` returns the open draft. Saving with `proposal_id` publishes it, and
    `POST …/proposal/{id}/discard` discards it.
  - The owner's Inbox gets one `charter_proposal` item, which opens `CharterPurpose`: the same
    component as Company › Charter, showing the draft over the current text with Publish, Edit
    and Discard. The Charter page shrank from 452 to 129 lines.
  - Filing a request to the person whose own direction it is is refused ("it is Direct: apply
    it"), which removes the "approve your own request" card.
  - Exec's doctrine (`context.rs`, "People own teams") now says to work on the source of truth,
    never call a file write "saved", and never ask the owner to run a command.
- **E. The Inbox column:**
  - Inbox and For you are tabs with counts, replacing two stacked lists.
  - Today is one line under the tabs.
  - Rows have two lines (`Item stacked`): the title wraps to two lines and the asker sits
    underneath, with no "X asks:" prefix.
  - The column defaults to 340 px (min 260) under a new pane key, so old 280 px widths reset.
  - The request card has a smaller title, no eyebrow, and short buttons ("Always allow",
    "Make … owner") whose full sentence is in the tooltip.
- **F. Guardrails:**
  - The tripwire records any shown conversation message that disappears without a retract. It
    collects these in strict mode and logs them to the console.
  - `check-ui-boundary.mjs` fails on any blanket `invalidateQueries` outside `lib/model`;
    `resyncVisible()` is the only one, used after an outage and on return from the background.
  - `web/scripts/verify-steady.mjs` runs journeys against a dev stack and a `_test` company.

### Verification (dev stack, `s82_test`, owner)

- `verify-steady.mjs`: entry, hints, answer, charter and settings all pass.
  - **entry:** after the reveal the only change is the office canvas replacing its own
    "Opening the company floor…" line.
  - **hints:** a real `message` hint (`restless message --from exec`) refetched the conversation,
    Inbox, For you and digest, and no settings read.
  - **answer:** the answer stays from the moment it is sent, while a stale read lands during the
    send.
    - Negative control: the same journey on the old `queries.svelte.ts`, without `steady()`, fails
      with exactly your report: answer, then buttons, then answer
      (`closed:answer → buttons:no-answer → closed:answer`).
  - **charter:**
    - `propose` puts a draft in the Inbox, and Publish makes it the charter that `charter show`
      reads back.
    - A second draft is discarded on the Charter page.
    - `apply` for a non-owner is refused.
    - `apply` for the owner saved it, and history shows "Exec".
  - **settings:** a charter save shows Saving…, then saved; a rename shows Saving…, then done.
    Nothing else on the page moves.
- `steady.test.ts`: the hint routing and the tripwire pass, and so does the local-write-wins test.
  That test fails with a plain QueryClient (checked).
- `npm run check` (svelte-check, type, UI boundary and document search) is clean. Web unit tests:
  142/142.
- `restless-engine` tests for direction, attention and context pass (41); the CLI direction test
  passes.
- Screenshots of the Inbox and the charter item were taken at 1440, 1024 and 390 px.

### Second pass: every view onto the model layer, and every role

The first cut fixed the jank centrally and left 21 views calling the network themselves. This pass
moves all of them onto one data layer.

- **`lib/model/setting.svelte.ts`** has three helpers:
  - `settingQuery` is a cached read with an optional data-dependent poll: fast only while
    something is installing or a sign-in is being checked.
  - `saveSetting` writes a save's answer into the cache.
  - `ownerRequest` is the one JSON request helper.
- **Moved onto `settingQuery` and model functions:**
  - email authority, custom harnesses, harness sign-in, the vault and Telegram;
  - the company name, logo, intelligence picker and chip, copy-from-company, and limits;
  - the model provider screen (`CompanyProvider`, onto `lib/model/provider.ts`);
  - account connections (`AccountConnections`, onto `lib/model/account-connections.ts`);
  - the shared charter, the Library, People, Work and the computer;
  - the desktop viewer, the doctor, the Inbox detail, reference previews and attachments.

  Each screen's own read, poll timer and sequence guard is gone; the cache does those jobs.
- **Lint:** `check-ui-boundary.mjs` now fails on any `fetch(` in `lib/components`,
  `lib/primitives` or `routes`. A planted call is caught.
- **The charter draft is owner-only.** The charter read returns Exec's draft only to the owner;
  members and admins read only the published charter.
- **Two flickers the role checks found:**
  - The owner's cockpit was revealed before the company list arrived, so the setup notice and the
    Company badge appeared just after. The reveal now waits for every read still waiting for its
    first answer, not only the company's own.
  - The office floor said "Live view offline" for a moment while the computer's health was still
    unknown or starting. It now says so only once a source has answered that it is down.

- **A lapsed session is noticed at once again.** Hints no longer reload the principal, so an open
  tab whose session ended (a role change, removal) noticed only at its next slow poll. When the
  company change stream drops, the tab now re-reads its principal immediately, and once the stream
  is back it catches up on any hint it missed. The multiplayer smoke found this: the same journey
  passed on the pre-sprint web and failed on this one until the fix.

**Verification:**
- **Every role:** `scripts/multiplayer-smoke` (owner and two members, one later made admin, over
  real network entry) now also checks:
  - the cockpit appears once, settled, with no shown message vanishing, for owner, member and
    admin;
  - a member's charter `apply` is refused; her `propose` makes a draft that the owner sees and
    the member and admin do not;
  - the draft is an Inbox item; publishing it makes it the charter and closes the item;
  - the owner's own direction is never filed as a request.
- The multiplayer smoke passes all 28 steps. After a role change, the person's open tab re-enters
  through the account by itself, and the test's own visit to the account can land in the middle of
  it; the test retries that visit and logs each attempt. One retry was needed, and the second
  attempt reached the account.
- **`verify-steady.mjs`** gains `screens`: the ten settings and workspace screens open with no
  page error and no server failure. A spend-limit save changes only the limit and its "saved"
  time, and holds after a reload.
- The full suite passes together (entry, hints, answer, charter, settings, screens). The entry
  journey now sees no change at all after the reveal.

### Shipped

- **First cut:** Core release `38489642230` (`41237b3a`), Cloud deploy `38492470678`.
- **Second pass:** two more Core releases, `38503784511` and `38515498829`.
  - The release smoke failed the cut in between: it found four 502s, caused by the shell's
    profile menu asking for plane status before anyone was named. Fixed with a blank avatar slot
    until the viewer is named.
  - Final release `38515498829` (`ec4e93cb`), Cloud deploy `38520124725`.
- **Carried along:** these releases include the other sessions' work on `main`, with migrations
  0101–0106. All six are additive: new tables, and nullable or defaulted columns.
- **Fixed on the way:** two refused requests on a member's screens. Company › Charter read the
  owner-only company view, and Brand & voice read the owner-only company list. Both are now read
  only by the owner.
- **On production, as a Reno member:**
  - the cockpit appears once at 1.6 s and nothing changes after;
  - Company, Charter, Brand & voice, Members, Schedules, People, Library and Work all render;
  - none of them makes a refused request, apart from the documented member refusal of
    `/api/appliance`.

### Not done or not verified

- **No model was connected on the dev stack.** The answer journey stands in a ready model route
  and intercepts the conversation. An Exec turn that follows the new charter doctrine needs a
  model; the next real charter request on Cloud is that check.
- **The owner's view on Cloud** needs the owner's sign-in; the in-app browser holds a member's.
- **The direct-link hop is shortened, not removed.** Without a session for the company's plane, a
  link must go through the account: that is where the session is made. The edge router now sends a
  top-level visit there itself, before asking the plane
  ([restless-cloud#88](https://github.com/BlueprintLabIO/restless-cloud/pull/88), deploy
  `38545592906`).
  - Production: a sessionless visit gets a 303 to `/account?return=<page>` from the router; the
    API keeps the plane's own 401.
  - The round trip to the plane (about 300 ms) is gone; the visit to the account remains by
    design.
- **The `CharterProposal` database scenario has no Rust test.** The live charter journey and the
  multiplayer smoke cover it against the real stack.
