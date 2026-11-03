# Sprint 110: polish pass

The bar is the Claude and ChatGPT desktop apps: the final styled render at once, nothing that
moves after it, and no row that changes height.

**How this was checked.** A local `restless-dev` stack ran with the Reno-scale seed
(`scripts/seed-reno-scale.mjs`), a scripted model (`tools/mock-model`) and one document review
waiting on the owner. Headless Playwright took a screenshot of every primary surface at 1440 and
390 wide, in light and dark. Each page was also probed for overflow, cut text without a tooltip,
row heights, small targets, contrast and broken images. Further passes covered loads with every read
held, loads with every read failing, an Exec chat in use (composer growth, Enter and Shift+Enter,
a streamed reply with a table and code, scrolling up and Jump to latest, focus) and frame-by-frame
measurements of what moved after first paint.

Screenshots are on the dev machine, not in the repo. The paths in the table are relative to
`~/s110/`:

- before: `before/<surface>-<desk|phone>-<light|dark>.png` (failing reads under `before/failure/`)
- after: `after/`, the same names
- measurements: `review-*.png`, `film-review/` (a filmed review card load), and the scripts
  `shots.mjs`, `measure.mjs`, `chat.mjs` and `rail.mjs`

## Ranked findings

| # | Issue | Surface | Screenshot | Fix | State |
|---|---|---|---|---|---|
| 1 | Folder rows arrive after the files and push every file down 39px. The explorer was drawn before the computer's file listing (and the files a person kept) had answered, and their folders go above the files. | Library, All files, cold | `before/library-desk-light.png` | Hold the list until the listing and kept files have answered once, for at most 4 s. | Fixed (PR #651) |
| 2 | A document review card opens on the "Request review" form and a "Review details" footer, then swaps in the decision form and its buttons. Everything below moves 14px, and the proposals skeleton shrinks by 34px when it resolves. | Inbox › document review | `review-1440.png`, `film-review/` | Until the reviews are read, show the decision's own fields and buttons, disabled. The proposals line loads as the empty line's box, and the version under review as a skeleton. | Fixed (PR #651) |
| 3 | Company's overview is 570px wide on a 390px phone. The settings page grid's implicit column sized itself to the charter's longest line. | Company › Overview, phone | `before/company-phone-light.png` | `grid-template-columns: minmax(0, 1fr)` on settings page bodies (lib/ui Page). | Fixed (PR #650) |
| 4 | A streamed reply sits 31px left of where its finished message lands. Its lines break differently and rewrap sideways when it lands. | Exec rail, People chat | `before/chat-replied-1440-light.png` | Indent the streamed copy and caret as the finished message is. | Fixed (PR #651) |
| 5 | The rail's resize default (380px) differs from its token (382px), so a rail opening without a saved width can resize a frame later. The allowlisted sub-pixel shift of a member's empty rail lines up with this. | Exec rail, open | `rail.mjs` measurements | Default 382px, and a closed rail keeps its width (`holdWhileDisabled`). | Fixed (PR #651) |
| 6 | Counts on coloured badges are white on the dark theme's gold and blue: 1.9:1 and 2.2:1. | Library, People (dark) | `before/library-desk-dark.png` | `--text-inverse`, as the top bar's badge already uses. | Fixed (#650, #651) |
| 7 | A document's block grip grows to 44px on a narrow window and covers the first letters of the line beside it. | Library document, 390 | `before/library-document-phone-light.png` | Leave `.block-handle` out of the phone-width 44px rule; touch screens hide it anyway. | Fixed (PR #650) |
| 8 | On a board card, the priority and outputs line is body-sized and wraps "4 outputs" onto a line of its own. | Work › Board | `before/work-desk-light.png` | One label-sized line, with a tooltip when it is cut. | Fixed (PR #650) |
| 9 | The document inspector's Close button takes a 40px row of its own above the tabs. | Inbox review, Library document | `review-after-390.png` | Put Close on the tabs' line. | Fixed (PR #651) |
| 10 | A grey smudge shows above the brand. The hidden skip link's shadow peeks into view. | Settings, cockpit | `before/acct-voice-desk-light.png` | Hide it far enough up. | Fixed (PR #650) |
| 11 | A streamed reply still lands in a different place. It streams below the "Exec is replying" row, then the finished message, with its name row, appears above that row, so the text jumps up and gains a header. | Exec rail | `before/chat-streaming-1440-light.png` | Render the streamed reply as the transcript's last message, header included, and keep the turn summary below it. ConversationTurnDock has to be restructured. | Open |
| 12 | The composer's context chip is cut to "W." or "E." on phones. The model chip has a fixed 136px, and the touch-sized buttons take the rest. | Exec rail and People chat, 390 | `before/chat-replied-390-dark.png` | Show the chip as its icon alone below ~420px (the tooltip already names the page), or let the model chip shrink to its mark. | Open |
| 13 | Inbox rows differ in height (74px vs 56 or 64px) depending on whether the title wraps to two lines. | Inbox list | `before/inbox-phone-light.png` | Decide: one-line titles with a tooltip, or reserve two lines on every row. | Open, needs a design call |
| 14 | When every company read fails, the page blurs behind "Reconnecting to your company… usually because Restless is updating". For a read that answered 503, that is not what happened. On the dev server the boot frame also stays blank for several seconds first. | Every surface, failing reads | `before/failure2/inbox-all-desk-light.png` | Tell "Core unreachable" apart from "Core answered with an error". Say the latter in place, with Retry. | Open |
| 15 | The composer grows 11px from one line to two, then 19.5px per line. Its one-line height is taller than one line. | Exec composer | `chat.mjs` heights | Bottom padding sized so one line fills the 44px exactly, so every line adds 19.5px. | Fixed (PR #652) |
| 16 | Wording: "Decisions · Included · Not on this computer" doesn't say what the owner can do. | Company › Intelligence › Models by job | `before/co-provider-desk-light.png` | Name the model, and say "runs on Restless" or "needs a model connection". | Open |
| 17 | The probe flagged info tips at 22×22 on a 390 window. | Every settings page, 390 | `before/co-provider-phone-dark.png` | None needed: on a coarse pointer, InfoTip already grows its hit area to 44px without moving the layout. | Not an issue |
| 18 | The disabled voice-preview button reads as an empty box. | Settings › Voice | `before/acct-voice-desk-light.png` | Hide it until a voice is chosen, or give it a label. | Open |
| 19 | When the facts line wraps on a phone, it leaves "·" separators dangling at line ends. | Company › Overview, 390 | `after/company-phone-light.png` | Below 560px each separator is a zero-height line break, so there is one fact per line. | Fixed (PR #652) |
| 20 | An empty board column ("Next 0") has no line saying so, and on a phone it is a tall empty box. | Work › Board | `before/work-phone-light.png` | Add a one-line empty state at the column's fixed height. | Open |
| 21 | The view strip on a phone is cut at the screen edge with no fade, so it doesn't look scrollable. | Library, 390 | `before/library-phone-light.png` | Add an edge fade mask while it can scroll further. | Open |
| 22 | A document shows its block grip beside the first block at rest, before any hover. | Library document | `before/library-document-desk-light.png` | Show it only on hover or focus of a block. | Open |
| 23 | Coverage gap: the sign-in page can't be reached on a local appliance (the owner is signed in on loopback). | Sign-in | n/a | Cover it with the identity harness or a gallery page. | Open |

What was checked and found right: there is no horizontal overflow on any other surface. Enter
sends and Shift+Enter adds a line. The composer grows to 8 lines and then scrolls. A pasted file
attaches (`Composer.svelte` onpaste). Chat sticks to the bottom while a reply streams. Scrolling up
shows "1 new ↓", which returns to the bottom. Focus rings are 2px `--intent-conversation`. The
reduced-motion rules cover the rail, the board, the caret and view transitions. The 404 page is
calm and has a way back. Slow reads draw final-shaped skeletons.
