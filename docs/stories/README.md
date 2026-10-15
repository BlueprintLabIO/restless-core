# User stories

The stories Restless is checked against. Each one says what a person wants and what they should see, in
the owner's terms rather than as a feature. The list grows from use: every problem the owner reports
becomes a story here, with the date and the check that now guards it. That way the same problem
cannot come back unseen.

- **core:** run on every UI change, on the Reno-scale seed (`scripts/seed-reno-scale.mjs`), in the
  built-in browser at desktop and phone width.
- **full:** run before each release.

The 210 stories of the 14 October audit are in
[`docs/audits/user-story-gap-audit-2026-10-14/README.md`](../audits/user-story-gap-audit-2026-10-14/README.md)
(US-001 – US-210). The stories below were added after it.

## From the owner, 14 October (afternoon)

| ID | Tier | Story: I want … and I see … | Fix | Check |
|---|---|---|---|---|
| US-211 | core | I want to change one line of my brand voice. I see Voice as a list, edit the line in place, and it is saved as a new brand version. | S85-01 | browser: Company › Brand & voice |
| US-212 | core | I want to remove a wrong brand statement. I see a remove control on the row, and it is gone after saving. | S85-01 | browser |
| US-213 | core | I want to accept what Exec learned from my corrections. I see its draft inside the right section, with Accept and Dismiss. | S85-01 | browser |
| US-214 | core | I want to talk to a staff member the way I talk to their lead. I see the same chat view. | S85-02 | browser: People › staff |
| US-215 | core | I want the agent badge in the top bar to take me to that agent. I see that agent's chat, not Exec's. | S85-02 | browser |
| US-216 | core | I want to open a chat and read the latest. I see the newest message, without scrolling. | S85-03 | `verify-motion` |
| US-217 | core | I want to watch an agent work without the screen jumping. Tool rows never flash open, and replies keep their width when they finish. | S85-04 | `verify-motion` |
| US-218 | core | I want a quiet composer. I see no "Under way" or "Asked …" strips under it. | S85-05 | browser |
| US-219 | core | I want to know why Exec couldn't reply. I see a plain sentence with a link to fix it, not a code under a fold. | S85-06, S86 | `capture-surfaces` raw-text probe |
| US-220 | core | I want "Needs you" to take me there. I click it and land on the page it names. | S85-07 | browser |
| US-221 | core | I want to send screenshots to Exec. I see small thumbnails above my text before sending. After sending they show as thumbnails, and older ones still render. | S85-08 | browser |
| US-222 | full | I want to copy a long code block from anywhere in it. The copy button stays in view. | S85-09 | browser |
| US-223 | core | I want to see what Exec told my lead and how they answered. I see it in the lead's chat in People. | S85-10 | browser |
| US-224 | full | I want to start a group chat. I see a clean sheet with people and one button. | S85-11 | browser |
| US-225 | core | I want my Library to match what Exec says it archived. Retired outputs are gone from the default view without a reload. | S85-12, S86-02 | browser on seed |
| US-226 | core | I want no buttons that do nothing useful. There is no "+" on Library and no sparkle on Work. | S85-13 | `capture-surfaces` |
| US-227 | core | I want Done to stop piling up. The board shows the last week, and I can clear the rest with a small celebration. | S85-14 | browser on seed |
| US-228 | core | I want to look at a piece of Work. I see its agent's chat with the details beside it, not a separate page. | S85-15 | browser |
| US-229 | full | I want tooltips only when they add something. A visible file title has none. | S85-16 | `capture-surfaces` |
| US-230 | core | I want to widen a cramped sidebar. I drag its edge, and it stays that width. | S85-17 | browser |
| US-231 | core | I want a calm Company menu. I see names only, with no grey status on the right. | S85-18 | `capture-surfaces` grey-status probe |
| US-232 | full | I want to see and change who owns a team. I see a quiet "Owned by" line with a menu. | S85-19 | browser |
| US-233 | core | I want to read every row. Nothing runs under a button or toolbar. | S85-20 | `capture-surfaces` overlap probe |
| US-234 | full | I want changing an agent's model to read like choosing one. I see the same labels both times. | S85-21 | browser |
| US-235 | core | I want to know which list holds my decisions. I see Decisions and For you, never "Inbox · Inbox". | S85-22; superseded by US-247 | browser |
| US-236 | core | I want Exec to notice what I changed. After I raise the ceiling or rename the company, Exec knows. | S86-01 | unit + smoke |
| US-237 | core | I want Exec's "done" to be true. When Exec says it archived something, the Library agrees. | S86-02 | doctrine + browser |
| US-238 | core | I want to know why a model stopped working. I see who removed it and when, and one Inbox item to fix it. | S86-03 | unit + API |
| US-239 | core | I want Exec to read my screenshots. It describes them, or says once that this model can't see images. | S86-04 | unit |
| US-240 | core | I want never to be told to run a command. Exec's replies name pages, not shell commands. | S86-05 | unit |
| US-241 | core | I want to find a skill or tool to add. I search Apps, find items from well-known registries, and install one deliberately. | S87 | browser + unit |
| US-242 | core | I want Exec's suggestions to be installable. Its suggestion opens the item in Apps. | S87-05 | browser |
| US-243 | core | I want to see everyone at a glance: what each is doing, and who needs me. People is a directory with a status word and current Work per person. | S88-02 | browser on seed |
| US-244 | core | I want to check in with someone and talk to them in one place. Their page has their work and model above our chat. | S88-03 | browser |
| US-245 | full | I want to let an agent go, and bring them back later. Archive and Re-hire on their page keep their history. | S88-01 | API test + browser |
| US-246 | core | I want to act on any Inbox item without hunting for its buttons. Decisions stay at the bottom right of the middle pane while its content scrolls, including requests, replies, charter drafts and document reviews. | S89-04 | browser + attention-footer geometry check at desktop/phone widths |


## From the owner, 15–16 October (Sprint 89)

| ID | Tier | Story: I want … and I see … | Fix | Check |
|---|---|---|---|---|
| US-247 | core | I want one Inbox ranked by urgency. Decisions, review requests and updates share one list with essential actions and discoverable shortcuts. | S89-03 | inbox-feed ordering tests + browser |
| US-248 | core | I want to stop or redirect a reply. One anchored composer button stops an empty draft or sends new direction immediately, for Exec and Staff alike. | S89-02 | real owner-reliability Stop/redirect + verify-motion |
| US-249 | core | I want to reach any active agent. A non-lead Staff member receives my message directly and keeps its production role. | S89-08 | rooms backend suite + real Staff conversation |
| US-250 | core | I want to know who needs me and who Exec is waiting on. People distinguishes Needs you from Waiting; Exec shows delegated company work. Hover never moves the status. | S89-06, S89-09 | real owner-reliability People journey |
| US-251 | core | I want to clear finished work and manage goals. Controls stay in the toolbar; archiving a goal is explicit and reversible. Counts alone never declare an outcome achieved. | S89-05 | goal candidate tests + real archive/undo/edit |
| US-252 | core | I want useful Health recommendations. The complete diagnostic wraps and one click sends it to Exec. The control remains clickable on hover. | S89-10, S89-15 | real diagnostic send with controlled degraded read |
| US-253 | core | I want a focused team form and a readable person profile. Team editing uses a dialog; profiles use an opaque full-height right drawer. | S89-07, S89-11 | real team create/rename + drawer checks at 1440/390 |
| US-254 | core | I want a useful quiet Inbox. The office floor fills the pane without an extra recent-outcomes or tell-Exec card. | S89-11 | empty Inbox projection with real office graph at 1440/390 |
| US-255 | core | I want to reach Home or a company without a blank screen. A slow read keeps the loading wave, and a failed or stalled read gives a recovery action. | S89-01, S89-12 | delayed, failed and hanging real-read journeys |
| US-256 | core | I want to understand an old brand draft I cannot accept. The page explains the version conflict and lets me ask Exec for an updated draft without overwriting my newer changes. | S89-04 | real stale proposal and real Exec message |
| US-257 | core | I want readable failures. Stored provider errors show a useful sentence and keep technical details folded. | S89-02, S89-15 | turn-failure tests + seeded historical provider error |
| US-258 | core | I want conversation search and menus to stay on screen. Popovers share viewport-aware placement and keep focus behavior. | S89-06, S89-15 | overlay/search journeys pending final audit |
| US-259 | core | I want account navigation and company switching to agree. All companies opens the authoritative list; creation stays visible at the plan limit and leads to upgrading. | S89-12, S89-13 | Core destination tests; Cloud runtime verification pending |
| US-260 | core | I want unnecessary account chrome removed. AI Apps redirects to Connections, checkboxes remain a sensible size, and the desktop brand is not duplicated. | S89-14 | Core phone/redirect journey; Cloud visual verification pending |
