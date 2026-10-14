# Sprint 88 — People: everyone, their work and their chat, on one page

**Status:** landed (S88-01 263e23a9, archived list 0a3e1a5d; S88-02..06 c1ef45b8..da57736f)
**Programme:** owner feedback; quality of the cockpit
**Depends on:** Sprint 85 (S85-02, one chat for every agent; S85-15, the Work sheet beside a chat).

## Outcome

On 14 October the owner asked for People to be a directory. They want to:
- check in on every employee and see their current work;
- talk to them;
- re-hire or archive them.

People and Chats as two tabs felt awkward. The owner reviewed two mockups the same day and chose the
design below ("amazing, love it").

**The person is the page.** On one screen:
- a directory of everyone on the left;
- the selected person's chat in the middle;
- their Work, with the pages it made, on the right.

Chat stops being a separate idea, and so does the Work page.

## Layout

```
┌──────────────┬───────────────────────────────┬────────────────────────┐
│ Directory    │ Header strip                  │ Work                   │
│ Search       │ El  Elif · Lead designer      │ Now                    │
│ Everyone …   │     Reno Studio · Opus 5.5  ⋯ │  Reno's own site, v6   │
│              ├───────────────────────────────┤  [preview][phone]      │
│ Exec  Idle   │ From Exec: Build v6 …         │  [desktop][notes]      │
│ Reno Studio  │ ▸ Shared a page: reno-site-v6 │  Checks 2/4 · $9.35    │
│  Elif Working│ You: hero too big on phone    │ Next                   │
│   Emeka …    │ Elif: fixing it now           │  Pricing page          │
│   Esra  Idle │ ◌ Working · 2m · resizing     │ Done this week         │
│ Colleagues   │                               │  ✓ Reno site v5        │
│ Chats        │ [ Message Elif…            ]  │ All of Elif's work     │
└──────────────┴───────────────────────────────┴────────────────────────┘
```

### Left: the directory (S88-02)

- **Grouping:**
  - Exec first.
  - Then each team, with "Owned by <person>" under its name. The lead comes first, with staff
    indented under the lead.
  - Then human colleagues.
  - Then group chats, under "Chats". These replace the People / Chats tabs.
- **Each row:**
  - initials avatar and name;
  - one status word, coloured by meaning: Working (green), Waiting (amber), Blocked (red),
    Idle (quiet);
  - one line: the title of their current Work, or their role when they have none.

  Rows ellipsize; nothing is cut mid-word (S85-20). An unread message adds a dot.
- **Filters:** Everyone · Working · Needs you (with a count) · Archived. Search filters names, roles
  and Work titles.
- **Archived** lists retired agents. Their page is read-only, with Re-hire as its only action.

### Middle: header strip and chat (S88-03)

- **Header strip:**
  - name, then role, team and the model (the composer chip's menu);
  - "Owned by" as a quiet line (S85-19);
  - a ⋯ menu with Change model, Move team, Archive and Re-hire.

  There is no separate Message button: the composer is the action. A colleague's header shows
  their access instead of a model.
- **The chat** is the one conversation (S85-02). It opens at the latest message (S85-03).
  - Exec's messages to the agent show inline as "From Exec", as now.
  - When the agent shares a page, an inline card opens that page in the preview viewer. The chat
    and the Work panel point at the same file.
  - The live turn shows as a quiet one-liner ("Working · 2m · resizing the hero"), not a wall of
    tool rows (S85-04).

### Right: the Work panel (S88-04)

- **Now:**
  - the Work in motion: its title, a status pill, and "revises v5 · review by Esra next";
  - its outputs as thumbnails in a 2-up grid: the live preview, phone and desktop screenshots,
    documents. A thumbnail opens the existing viewer;
  - "Checks 2/4 · $9.35 · Details". Details opens the full Work sheet (S85-15). The old Work page
    is gone and `/work/<id>` lands here with that Work highlighted.

  When nothing is in progress, Now shows the last delivered Work and its pages, so the panel is
  never empty.
- **Next:** proposed and waiting Work, titles only.
- **Done this week:** finished Work. A page an independent reviewer accepted carries a small
  "Reviewed · accepted" badge, so the owner knows what is safe to send to a client. "All of
  <name>'s work" opens Work filtered to them.
- **For a lead,** the panel shows the team's Work, grouped by person.
- **Exec** shows Work it commissioned and is coordinating.
- **Colleagues** show the Work they own or follow.

## Behaviour

- **Desktop at ≥1280 px:** three columns. The Work panel is collapsible with a header toggle, and the
  choice is remembered per person.
- **1024–1279 px:** the Work panel starts collapsed.
- **Phone:**
  - the directory is the page;
  - a person opens full screen with a back arrow;
  - Work is a toggle in their header that swaps the chat for the panel.
- **Archive:**
  - Archiving asks once. OrgIntel's refusal (leads a team, owns open Work, owes a judgement) is
    shown as written, with the way forward (S88-01).
  - Archived agents stay in history, Activity and Exec's "Changed by people".
- **Data:**
  - Rows read the cockpit people view: status, current Work and spend.
  - The panel reads the Work graph and its artifact refs. Nothing new is stored.

## Work items

| Item | What | Owner |
|---|---|---|
| S88-01 | `restore_actor`; archive and re-hire routes, recorded as setting changes | core (done, 263e23a9) |
| S88-02 | The directory: groups, status words, current Work, filters, Chats folded in, tabs removed | cloud |
| S88-03 | Header strip with the ⋯ menu; no Message button; shared-page cards in the chat | cloud |
| S88-04 | The Work panel: Now with output thumbnails, Next, Done with review badges; team view for leads | cloud |
| S88-05 | Widths: collapsible panel at ≥1280, collapsed by default below; phone layout with the Work toggle | cloud |
| S88-06 | `/work/<id>` lands on the agent's page with that Work highlighted in the panel | cloud |

## Acceptance

On the Reno-scale seed, in the built-in browser:
- The directory shows every agent with a status word and current Work, with nothing cut mid-word.
  The filters work, and group chats appear under Chats.
- Selecting Elif shows her header, the chat at its latest message, and her Work panel: v6 with its
  thumbnails, Next, and Done this week.
- A thumbnail opens the viewer. Details opens the Work sheet.
- An idle agent's panel shows their last delivered Work.
- Archiving an agent moves them to Archived with a read-only page. Re-hire restores them. Activity
  shows both.
- At 1100 px the panel starts collapsed. At 390 px the Work toggle swaps chat and panel.
- `capture-surfaces` finds no clipping, sideways scroll or raw text at 1440, 1024 and 390 px.
