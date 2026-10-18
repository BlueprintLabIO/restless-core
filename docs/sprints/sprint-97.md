# Sprint 97: setup that becomes Exec's rail, models connected in place, leaner sessions, three skills

Owner-approved scope (2026-10-18). Follows S96 (setup inside the cockpit).

## S97-01 The setup chat morphs into Exec's rail (cloud)

- On "Let's do it" the rail appears exactly over the setup chat's box (same position, size and
  scroll position) and its width animates into the right rail, text reflowing, never scaled. On a
  phone the setup chat becomes the full-screen Exec chat.
- The setup page and the rail render messages with the same component, so only the width changes.
- Exec's opening lines are real messages: when a company is created, Core writes Exec's welcome
  ("Hi, I'm Exec…", the name question) into the Exec conversation, and the name the owner settles
  on is recorded as a short owner message, so the rail and every device show the whole setup.

## S97-02 Connect a model where it is missing (cloud)

One connect card, used in the setup Model step, the empty Exec rail, the model chip and Company
Health: use a connection the account already has (one click), paste an API key (provider, key,
model), or sign in with a ChatGPT or Claude plan in a new tab. The card waits in place until Exec's
route is ready, then the surface carries on. No navigation to Intelligence.

## S97-03 Compaction: idle and early, by a cheap model, latest chat kept (core)

- Restless compacts, not the harness: after inactivity (default 30 min, configurable) and when a
  session passes ~50% of its context window.
- A cheap model (the cheapest on the company's connection, Haiku-class) writes the summary:
  decisions, open threads, and pointers to the notes, charter, brand and Work that hold details.
- The next turn starts a fresh session from durable context + the summary + the latest N turns
  verbatim. Recent chat is never summarised.
- Measured on scenario tasks before and after: tokens per turn and whether answers still use what
  the notes hold.

## S97-04 Built-in skills activate when needed (core)

Built-in skills ship in every company image and are always available; an agent activates one when
its task calls for it, with no owner toggle. Owners can still switch a skill off.

## S97-05..07 Research, competitor research and GTM skills (cloud)

Skills in `infra/company-image/skills`, public skills used as-is or as reference (license
checked, scripts read before use), each with a before/after evaluation:

- **research**: question → plan → primary sources first (company browser) → brief where every
  claim carries a source and date; confidence per claim; unconfirmed things marked; no invented
  numbers; open questions; a budget per run; a Library document.
- **competitor-research** (uses research): direct, indirect and substitute competitors; a profile
  each from their own site, pricing, changelog and reviews; a comparison sheet; positioning gaps;
  optional scheduled refresh.
- **go-to-market** (uses both): from the charter and brand: ideal customer, positioning, channels,
  a pricing hypothesis, a launch plan, a 30-day experiment list with metrics; hands off to
  landing-page-design and presentation-deck; nothing leaves the company without approval.
