# Sprint 64 — Sidebars that read clearly, and a Library that holds everything the company makes

**Status:** built and verified on a `_test` company; the model-driven dogfood (T9) waits on a model
connection (see Results)
**Programme:** owner cockpit
**Depends on:** Sprint 19's review targets (S19-T5, Runtime-served files), the Work graph's artifact
references, Sprint 62/63's sidebar structure (`8c1639f`, `d63a871`).
**Mockups:** `sidebar-targets.html`, shared with the owner on 10 October 2026 (People, Chats, Work,
Library, Apps, Company, phone, and People beside the 1 October design).

## Outcome

Every left sidebar reads clearly at a glance: a real title, a search field, rows at full body size,
what each person or goal is doing in readable colour, and the selected row raised like the selected
top-navigation item. Every row leads somewhere real.

Library holds everything the company makes, not only native documents and sheets: the sites,
images, PDFs, decks and recordings that Work produced on the company computer. The owner opens any
of them in place. A deck plays as a slideshow.

**Product hypothesis.** The owner found the sidebars "lost their soul" after two restructures made
every surface small, grey and flat. Presence comes from legible type, real controls and live
readings, the way the chat area is "subtle but clear", not from cards or chrome.

## Observed friction

Read on 10 October 2026 from `main` at `ea4d8a1`, with captures of three versions on a `_test`
company.

| Friction | Evidence |
|---|---|
| Sidebars are timid | 11px tertiary labels, 28px rows in secondary ink, a grey wash for selection (`SidebarGroup`, `SidebarRow`) |
| Library shows orphaned headings | `Documents` and `Sheets` are folding headings with nothing under them in a new company; "Needs your review" left the sidebar in `8c1639f` |
| Library cannot show most of what the company makes | `library/+page.svelte` reads only `documents` and `sheets`; sites, images, PDFs and decks exist only as Work artifact references |
| Produced files open only inside one review | `issue_review_ticket` (`owner_review.rs`) resolves an Attention item; there is no way to reopen a finished site or deck |
| Decks are downloads | `.pptx` is `is_runtime_review_download`; there is no slide player and no guidance for Staff to make a deck the owner can play |
| Work's sidebar has no voice | the goals explanation was replaced by a bare "Propose goals" row; goals show no progress |
| Chats cannot say what was said | `RecentDirectConversation` carries only the last message id and time; group rooms carry no activity time |

## Design stance

**Core contract**

- **One sidebar kit, one weight.** `SidebarShell`, `SidebarGroup` and `SidebarRow` change once; every
  surface (People, Work, Library, Apps, Company) inherits it. No second tier.
- **Library is a projection.** It joins native documents, native sheets and Work's available
  artifact references. No new table, copy, export or custody state. Superseded and missing
  references are not listed.
- **Opening a Library file reuses the review origin.** A Library ticket resolves one available
  artifact reference instead of one Attention item, probes the exact file, and serves it read-only
  through the same isolated origin and TTL. This widens the review viewer from "the current review"
  to "any file Work recorded", for the owner only. It remains a bounded read path, not a general
  file server: the path must be beneath `/company`, have a renderable type, and belong to a
  recorded, available artifact.

**Product hypothesis**

- Live readings in the sidebar (what someone is doing, a goal's progress, a setting's value) make it
  a status summary the owner trusts more than a separate dashboard.
- HTML and PDF decks are enough. `.pptx` stays a download until a real company needs previews.

**Default pattern**

- Colour is semantic only: green working or live, amber waiting on the owner, red blocked or
  broken, blue unread.
- Staff record a deck with artifact kind `deck`. Library groups by declared kind first, then by
  file type.

## Decisions

Made by the agent under the owner's go-ahead (10 October 2026), recorded here per the solo-founder
working arrangement.

1. **Pins are per viewer, in browser storage.** Pinning is a convenience, not company state, so it
   does not justify a new table. Accepted risk: pins do not follow the owner to another device.
2. **Goal progress is derived from Work, never invented.** "4 of 6 done" counts completed Work under
   the goal; "1 blocked" counts blocked Work. No "on track" judgement until something produces one.
3. **A deck is what Staff declare as one.** An HTML or PDF file is a deck when its artifact kind is
   `deck` (or `slides`, `presentation`), or its label or file name says deck or slides. A `.pptx` is a
   deck that downloads. The planned `<meta>` marker was dropped: the cockpit cannot read a file on its
   isolated origin before opening it.
4. **Company keeps its search client-side.** It filters the known settings rows; it is not a
   settings index.
5. **Library files are the owner's view for now.** A collaborator's projection carries no file
   locations and the Library ticket sits on the owner router. Collaborators still see documents and
   sheets. Revisit when a collaborator needs a produced file.
6. **An HTML deck steps; a PDF deck reads.** The HTML deck contract (the `presentation-deck` skill)
   lets the player step slides and show "2 / 3". The browser's PDF reader ignores a changed `#page`
   once loaded, so a PDF deck is shown in the reader, page after page, with Present for full screen
   and no counter that would lie. A PDF frame is not sandboxed, because the reader refuses to run in
   a sandboxed frame; the file is already on its own read-only origin and runs no page script. The
   same applies to a PDF in the Inbox review frame, which was blank before.

## Tickets

- [x] **S64-T1 Sidebar kit at full weight** (cockpit). Title and action, search field, 34px rows at
  body size and weight 500, two-line rows with a toned reading, a right-hand reading, raised white
  selection, 13px weight-600 section labels, empty blocks with real buttons. Deletes the quiet
  label and wash-selection styles.
- [x] **S64-T2 People** (cockpit). Title and "+", search, People | Chats switch; each person's second
  line coloured by state (working, waiting on the owner, blocked) from the Work graph and owner
  handoffs; a working dot from Work; humans with pending invitations; the 1 October welcome in the
  main area for a new company.
- [x] **S64-T3 Chats previews** (OrgIntel + cockpit). Recent direct conversations and group rooms
  carry their last message's text (bounded) and time; Chats rows show the preview and sort by the
  last message.
- [x] **S64-T4 Work** (cockpit). All work, Waiting on you (owner handoffs), In progress (active) with
  counts; goals as two-line rows with derived progress and a progress bar; the goals explanation and
  a real button for a new company.
- [x] **S64-T5 Library files** (owner gateway + cockpit). Library lists available artifact
  references beside documents and sheets, grouped Documents, Sheets, Sites, Images, Decks, PDFs and
  Media with counts; a Library ticket endpoint opens one in the isolated origin.
- [x] **S64-T6 Library viewer and slideshow** (cockpit). Files open in the main pane: sites and
  images framed, PDFs inline, media in a player, decks in a full-screen slide player (arrow keys,
  counter, escape). Needs your review, pins and Archived as rows.
- [x] **S64-T7 Apps and Company** (cockpit). Apps at full weight; Company with search and a live
  reading on every row from `company-setup-rows`.
- [x] **S64-T8 Staff make playable decks** (engine prompts). When asked for a presentation, Staff
  produce an HTML deck (one section per slide, `<meta name="restless:deck">`) or a PDF, record it
  with kind `deck`, and never hand back only a `.pptx`.
- [ ] **S64-T9 Dogfood** (evaluation). On a `_test` company, Staff produce a site, an image and a deck;
  each appears in Library under the right kind and opens; the deck plays.

## What this makes deletable

- The quiet sidebar grammar (`d63a871`): small tertiary labels, wash selection, the separate People
  row styles.
- Library's folding Documents and Sheets headings.

## Results

Checked on 10 October 2026 on the local `ui_test` company (dev profile, daemon rebuilt from this
change).

- **Library ticket (T5).** Four real files written to the company computer under
  `/company/outputs/s64-seed/` and recorded as available artifact references on two Work items:
  `POST /library/open` returned a review URL for each; the URLs served `text/html` (deck, with the
  player contract), `application/pdf` (21 KB, three pages), `image/svg+xml` and `text/html` (site,
  with its own stylesheet). An unrecorded id returned 404; the same logo marked `superseded` returned
  409, then opened again once restored. A POST without a same-origin `Origin` is refused 403 by the
  existing owner boundary.
- **Viewer and slideshow (T6).** In the cockpit the HTML deck reported `1 / 3`; Next stepped to
  `2 / 3`, `3 / 3` and held at the last slide; ArrowRight, ArrowLeft and Home each moved exactly one
  step. The site rendered with its CSS, the logo whole, and the PDF deck in Chromium's reader (full
  Playwright Chromium; the desktop app's pane cannot capture the reader in screenshots).
- **Chats previews (T3).** `group-conversations` returned the group room with its last message's
  sender, time and text; Chats listed it as "You: The booking p…" with markdown removed.
- **Work (T4).** With one completed and one active Work under a goal, the sidebar showed All work 1,
  In progress 1, and the goal with "1 of 2 done" in green.
- **People, Apps, Company (T1, T2, T7).** Rendered at 1400px and 390px wide with no page errors.
  Company shows each setting's live value; its search filters the rows. People's switch fits its
  track on a touch phone.
- **Deck skill (T8).** The PDF export the skill describes was run with the company's Chromium. The
  first version printed only slide 1, because the player hides the other slides; the skill now
  carries print rules and the export has all three pages.
- `svelte-check` 0 errors, 0 warnings; `check:type` passes; `cargo check -p restless-orgintel -p
  restless-owner` passes.

**Not yet run (T9).** No model is connected in the dev profile and the local GPT route is not
provisioned on this host, so Staff have not produced a site, image or deck themselves. The bundled
skill reaches companies with the next company image. Run T9 on a `_test` company once a model is
connected: ask the Exec for a one-page site, a logo and a three-slide deck, then check each appears
in Library under its kind and that the deck plays.

### Audit and smoke (10 October 2026)

After the owner hit a raw error opening a Library file, every cockpit route and hot path was
audited on the local `ui_test` company with Playwright: a crawl of 64 page states at 1400px and
390px, and three interaction smokes (26 shell, People, Work, Library, Company and Apps steps; 10
editing, conversation and navigation steps; 6 phone and shortcut steps). Bugs found and fixed:

| Bug | Fix | Commit |
|---|---|---|
| Opening a Library file while the computer slept showed a raw Docker error | The open wakes a sleeping computer; a stopped one says so in a sentence; failures read as sentences with Try again | ad8ab25 |
| `restless sleep` always refused as "busy (restore)" | The command no longer holds a lifecycle lease and counts it as activity | ad8ab25 |
| Work outputs could not be opened; each said "Linked evidence for this Work" | Open (a real button) plays the output in the Library; the line names its kind and file | ad8ab25, 3aaf394 |
| Work board cut off its Done column beside the wider sidebar | Columns fit four abreast | ad8ab25 |
| People search for nobody showed a blank list | "No people match" whenever nothing shows | ad8ab25 |
| A group room opened with no row selected | The room shows in Chats, selected | ad8ab25 |
| Sidebar search drew a second focus ring | One ring, on the field | ad8ab25 |
| On a phone an open Library file sat under the list's search and views | They step aside while a file is open | a8a10e1 |
| Library and Apps phone views marked the selection three different ways | The raised chip everywhere | a8a10e1 |
| Favicon 404 after a client redirect from a nested route | Root-absolute icon and manifest links | a8a10e1 |
| Typing straight after Enter in a document title lost characters | Body selected and focused synchronously (10/10 runs keep every character; 3/5 without the fix) | 3aaf394 |
| Present moved focus into the deck frame, which swallowed Escape | Focus stays in the cockpit; windowed fallback without full screen | 3aaf394 |
| On Cloud every Library file and review opened blank: the plane issued its loopback review origin (`http://<ticket>.localhost:7794`), which a remote browser cannot reach and an https cockpit blocks (found by the hosted smoke) | A plane reached over the network serves tickets on its own host under `/review/<ticket>/`, sandboxed into an opaque origin (PDFs excepted); local planes keep their separate origin; `RESTLESS_REVIEW_ON_PLANE_HOST=1` opts a local plane in. Boundary checked: tickets refused when wrong, malformed, traversing, writing, or presented in the other mode; the sandboxed page reads neither storage nor the owner API | this commit |
| Ten `_test` document sidecars crash-looped (≈827 restarts each) after their companies were removed, unseen by the reaper, churning the host network | The reaper flags a sidecar whose company has no config and removes it when stopped or crash-looping | this commit |

Final run on main: crawl 64/64 clean (no console, page or HTTP errors, no overflow); smokes 26/26,
10/10, 6/6 with no errors; document typing 5/5.
