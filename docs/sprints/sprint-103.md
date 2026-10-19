# Sprint 103: Purge outcome standards

Owner direction (2026-10-20): "completely purge the fast, complete, exceptional etc. concepts from
the codebase, I'll need to have a think of that one a bit more down the line."

## What goes

The outcome standard (`fast`, `thorough`, `exceptional`, `frontier`) and its source
(`company_default`, `owner_override`, `owner_language`) were an owner-visible ambition promise carried
on company config, teams, goals, Work and owner messages, and injected into Exec, lead and Staff
prompts. It never changed authority, routing or spend. Sprint 102's model routing now decides how
much intelligence a turn gets, so the promise has no remaining live effect worth its weight.

Removed:

- the `OutcomeStandard`/`OutcomeStandardSource` types and every column, Postgres type and parameter
  that carried them (one migration drops them; existing data is discarded);
- the company config field (an existing `outcome_standard` line in a company file is ignored and
  dropped on the next write);
- prompt text, CLI flags, owner API fields, cockpit controls and generated bindings;
- tests, dogfood reports and sprint tickets that only exercised the concept are left in history; live
  docs stop describing it.

Nothing replaces it in this sprint. The owner will decide later whether an ambition setting returns.

## Acceptance

- `git grep -iE "outcome.?standard|OutcomeStandard"` finds nothing outside migration history and
  dated sprint/dogfood records.
- Engine, OrgIntel, CLI and owner suites pass; `npm run check` and the web unit tests pass.
- A `_test` company created before the migration starts, migrates and completes the bakery scenario.

## Checklist

- [x] Rust types, columns and migration
- [x] Prompts, CLI and owner API
- [x] Cockpit and generated bindings
- [x] Live docs
- [ ] Upgrade smoke on a `_test` company
