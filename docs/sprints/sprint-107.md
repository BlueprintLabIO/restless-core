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
  - Cause (lines at f86d4d9c). Both halves were true. Onboarding told the Exec the owner had
    already chosen the name: `crates/restless-engine/src/context.rs:771` appended 'The owner
    named the company "…"' and `crates/restless-engine/prompts/welcome-the-owner.md:8` and `:33`
    said the same, so a generated "Jolly Heron" was kept as the owner's choice. And the Exec had
    no way to rename: `DirectionOperation` (`crates/restless-engine/src/direction.rs:371-379`)
    ended at the charter operations, and the only writer of `display_name` was the owner's own
    settings session (`update_company_setup`, `crates/restless-owner/src/owner.rs:6838`), so the
    name could change only through `proposedName` (`welcome-the-owner.md:51`) at the read-back.
  - Fix. Onboarding says whether the name was chosen (a recorded `company_renamed`) or is a
    placeholder to replace from the owner's words, and `restless charter rename` lets the Exec
    rename at once on the owner's word. On Cloud the name lives in the owner's account, so the
    command refuses there and the name goes through `proposedName` or Company settings.

## B. Account defaults that new companies inherit

- [x] B1 Settings › Accounts gains "For new companies": a default model and a default decision
  model, each with the same picker the company uses. New companies inherit them; changing a
  default never changes an existing company.
  (Companies started on this computer; hosted creation goes through Fleet, not yet wired.)

## C. Company › Intelligence: default model and decisions

- [x] C1 The page shows, one row each, using the same picker:
  - **Default model**: what the Exec and every agent use unless an agent has its own.
  - **Decisions**: the model behind structured decisions. Choices: included (the relay's
    `decide`) or an OpenRouter model through the owner's OpenRouter key. Say plainly that
    decisions are advisory and start in shadow, and show when one last ran.
  - **Fallbacks**: as today (`ModelFallbacks`).

  Built (4 Nov) as "Models by job" under Models: Main, Everyday and Decisions rows. Decisions
  offers Included (gate active, `decision_model = "included"`), Included watching only (unset,
  shadow), an OpenRouter model through the company's OpenRouter account, or Off; the row shows
  the state, what it saved or would save and when it last ran, from `GET .../model-jobs`
  (also `GET .../decisions`).
- [x] C2 Per-agent models stay where they are (People), and the row says which agents override
  the default.
- [x] ~~C3 No everyday model in the UI.~~ Superseded 3 Nov: the owner asked for it ("some agents
  should be automatically a smaller agent like sonnet 5.5"). Everyday is a row; unset means the
  main model's cheaper sibling (Opus to Sonnet, GPT Sol to Terra, Gemini Pro to Flash), "Same as
  main" stores `off`, and an agent with its own model on People keeps it. Team leads, repairs,
  reviews and feedback stay on the main model.
- [x] C4 Rows never grow (house rules): one fixed-height row per setting, the picker opens below.

## D. Voice, on the owner's account

Voice is how the person talks to Restless, so it belongs to the account (Settings), not to each
company. A company that needs voice for its own work uses an account as a tool, like any other.

- [ ] D1 Settings › Voice: **Speech to text**: "Your browser" (default, today's behaviour) or a
  connected provider (an OpenAI or Deepgram key added in Accounts). With a provider, the Dictate
  button records, the plane sends the audio to the provider with the account's credential through
  the broker, and returns text. Audio is never stored.
  _Partly done (3 Nov): Settings › Voice ships with "Your browser" (default, working); OpenAI
  and Deepgram are listed as not available yet. The plane has no transcription route, so the
  provider path (record, broker, return text) is still to build._
- [x] D2 **Text to speech**: off by default. When on, a play button on Exec and teammate messages
  reads them aloud with the chosen provider and voice (or the browser's own voice). Optional
  "read new replies aloud while this tab is open".
  _Browser voice only (3 Nov); no provider voice yet, and the optional auto-read is not built._
- [x] D3 Members use their browser's speech by default; account voice providers apply to the
  owner's own sessions only unless the owner shares one.
  _Voice choices are per-person preferences, so a member starts on the browser (3 Nov)._

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

- ~~An everyday or per-tier model~~ (asked for on 3 Nov; built in C).
- Voice calls or always-on listening.
