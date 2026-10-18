# Sprint 96: company setup inside the cockpit

The owner asked for starting a company to stop being unstable: a modern stepped setup that still
reads as a conversation, and a clean hand-over between Cloud, the company's computer and the
cockpit. Approved design: https://claude.ai/artifact/8tnuWTLoevBF9Chs7vwavF

## What changed

- **Setup is a mode of the cockpit.** `/<company>/welcome` renders inside the same `AppShell` as
  every other page. While setting up, the top bar's tab group holds the steps (Name, Model only
  when Exec has none, Your idea, Read back); search, the Exec control and the phone dock wait. On a
  phone one line says where the owner is.
- **The steps are the cockpit's own chat.** Exec's messages, its question card with likely answers
  and "Something else…", and the composer, which never leaves the page. The thread grows upward
  from the composer, so a reply never moves what is already on screen. The read-back is a card
  whose lines are fixed in place by clicking them.
- **The name comes first.** Core's "Start a company" is one click (no pop-up); the generated name
  is the first question. Exec's welcome brief no longer asks for the name; it reads back the name
  the owner chose. Copying another company's setup lives in Company, setting by setting.
- **No model:** the Model step links to Intelligence, and Intelligence returns to setup by itself
  once Exec can think (checked every 3 s while setup waits).
- **Landing:** "Let's do it" saves the charter, the step tabs become the company's tabs in place,
  and Exec's rail slides in from where the setup chat sat. No View Transitions API needed; with
  no rail on screen the page fades in; reduced motion skips it.
- **Cloud** (restless-cloud): until a new company's computer is up, the account page shows that
  cockpit's frame (same bar, steps and pane, at the same size) and checks every 3 s, instead of
  the company list and the full-screen "Opening…" field.

## Fixed on the way

- The setup page built its Exec conversation query inside `$derived`; on a fresh load that query
  stopped reporting and froze the page on its first frame (the old welcome page did the same).
  The chat is now `CompanySetup.svelte`, keyed by company and viewer, with its queries built once.
- Quoted answers saved with `\r\n` kept their quote on screen.

## Verified

Local stack with a scripted mock model, headless Chromium at 1440×900 and 390×844: new company,
rename (top bar updates at once), Model step and the return from Intelligence, a typed answer, a
tapped answer, read-back edit (layout shift 0), "Let's do it" (charter saved, tabs swap, rail at
380 px), no horizontal overflow on a phone. `npm run check` 0/0, web unit tests 212/212, engine
context tests 20/20; Cloud `npm run check` 0/0, unit 25/25, waiting frame measured at the same
bar and pane boxes as the cockpit.
