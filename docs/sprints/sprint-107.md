# Sprint 107: choose the intelligence

Owner direction (25 Oct 2026): a new company should start by choosing who thinks for it; the
decision model and voice (speech to text, text to speech) need a place in the UI. No everyday
model for now: one company default model is enough.

What prompted it: the blog company created on 25 Oct came up on the included
`deepseek-v4.1-flash` although the owner has Claude and ChatGPT connected, the same thing that
happened to the Restless company on 23 Oct. The decision model and everyday model can only be set
through the Exec's settings command. Dictation uses the browser's own speech recognition with no
choice of model, and nothing reads replies aloud.

## A. A new company chooses its intelligence first

- [ ] A1 Before the Exec's first message, a new company shows one step: "Who thinks for this
  company?" It lists the owner's connected AI accounts (Settings › Accounts: Claude, ChatGPT /
  Codex, API keys) with their models, pre-selects the account default (B1), else the account most
  of the owner's companies use, and offers "Same as <company>" for each existing company.
- [ ] A2 Choosing grants that account to the company (the same grant Manage access writes) and
  sets the company default model, then the Exec starts on it. The included model appears only
  when the owner has no account of their own, or picks it deliberately.
- [ ] A3 Nothing starts silently on the included model: if the step is skipped or fails, the
  company says so in its Inbox with a one-tap fix, never a quiet fallback.
- [ ] A4 The Exec names the company during onboarding from what the owner says (the blog company
  kept the placeholder "Jolly Heron" after being asked to rename itself).

## B. Account defaults that new companies inherit

- [ ] B1 Settings › Accounts gains "For new companies": a default model and a default decision
  model, each with the same picker the company uses. New companies inherit them; changing a
  default never changes an existing company.

## C. Company › Intelligence: default model and decisions

- [ ] C1 The page shows, one row each, using the same picker:
  - **Default model**: what the Exec and every agent use unless an agent has its own.
  - **Decisions**: the model behind structured decisions. Choices: included (the relay's
    `decide`) or an OpenRouter model through the owner's OpenRouter key. Say plainly that
    decisions are advisory and start in shadow, and show when one last ran.
  - **Fallbacks**: as today (`ModelFallbacks`).
- [ ] C2 Per-agent models stay where they are (People), and the row says which agents override
  the default.
- [ ] C3 No everyday model in the UI. The `everyday_model` key stays in config, unused by the UI,
  until the owner asks for it.
- [ ] C4 Rows never grow (house rules): one fixed-height row per setting, the picker opens below.

## D. Voice, on the owner's account

Voice is how the person talks to Restless, so it belongs to the account (Settings), not to each
company. A company that needs voice for its own work uses an account as a tool, like any other.

- [ ] D1 Settings › Voice: **Speech to text**: "Your browser" (default, today's behaviour) or a
  connected provider (an OpenAI or Deepgram key added in Accounts). With a provider, the Dictate
  button records, the plane sends the audio to the provider with the account's credential through
  the broker, and returns text. Audio is never stored.
- [ ] D2 **Text to speech**: off by default. When on, a play button on Exec and teammate messages
  reads them aloud with the chosen provider and voice (or the browser's own voice). Optional
  "read new replies aloud while this tab is open".
- [ ] D3 Members use their browser's speech by default; account voice providers apply to the
  owner's own sessions only unless the owner shares one.

## E. Found while moving the Restless company (25 Oct)

- [ ] E1 Moving a company to the owner's PC copied the workspace at about 0.4 MB/s (9 GB, about
  6 hours, company paused throughout). Find the bottleneck (the machine link? tar over the Docker
  API?) and make a move of a typical company take minutes, not hours.
- [ ] E2 A move can't be cancelled. Add Cancel, which restores the company where it was.
- [ ] E3 During a move the company card says "Exec can't start". It should say "Moving to
  <computer>" with the current step and progress.
- [ ] E4 Settings › Computers said "The computer was not removed. Try again." after a removal that
  worked (restless-core #484 has a draft fix).

## Acceptance

- [ ] A brand-new company created live on app.restless.run by an owner with Claude connected
  starts on Claude without touching Company › Intelligence, and its Exec names it.
- [ ] Changing the decision model on a company shows on its Intelligence page and the next
  decision uses it (visible in Activity).
- [ ] With an OpenAI key added, Dictate transcribes through it; with the key removed, it falls
  back to the browser and says so.
- [ ] Text to speech reads an Exec reply aloud.
- [ ] Every path checked live as owner, admin and member, with failure cases: no AI account, a
  revoked account, a bad key, phone width. The report says which paths ran.

## Not in this sprint

- An everyday or per-tier model (the owner: not needed now).
- Voice calls or always-on listening.
