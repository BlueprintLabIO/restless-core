# Restless research: Organisational Intelligence

This folder holds Restless's research into **Organisational Intelligence (OI)**: what makes a group of
AI agents, working for a person, produce better outcomes than its members could alone, and what makes
it worse.

Start with the [charter](oi/CHARTER.md).

## Files

| File | What it is |
| --- | --- |
| [`oi/CHARTER.md`](oi/CHARTER.md) | The idea, the questions, the rules, and how OI shapes the product and its voice |
| [`oi/METHOD.md`](oi/METHOD.md) | How every experiment is run so its results hold up |
| [`oi/IDEAS.md`](oi/IDEAS.md) | The programme, the backlog and the evidence behind the choices |
| [`oi/01-do-reviewers-check.md`](oi/01-do-reviewers-check.md) | Do AI reviewers actually check? |
| [`oi/02-telephone-game.md`](oi/02-telephone-game.md) | How much of the owner's request survives each layer? |
| [`oi/03-good-news-travels-up.md`](oi/03-good-news-travels-up.md) | Does bad news reach the owner? |
| [`oi/04-forecasts-the-world-scores.md`](oi/04-forecasts-the-world-scores.md) | Do teams predict real auctions better? |
| [`oi/05-org-chart-search.md`](oi/05-org-chart-search.md) | Searching for the best AI org chart |
| [`archive/`](archive/) | Earlier experiments, kept as records, not current guidance |

## Shared task bank

Experiments 1, 2, 3 and 5 draw on one task bank, built once and reused: realistic requests in four
kinds of work (code, research, resale appraisal, customer email), anonymised, each with frozen
starting files and a grader. It lives in `oi/tasks/` when built. Experiment 2 builds it first.

## Rules

1. No counted run starts before its plan is committed and the owner has approved it.
2. Experiments run in sandbox companies on restless-dev. Nothing contacts a real person, spends real
   money or changes a live company without separate approval.
3. Generated run state (databases, model homes, workspaces, traces) stays out of git. Results, small
   data files and analysis scripts are committed.
4. Results are never rewritten. A later result can replace a conclusion, and says so.

## Archive

[`archive/2026-11-coordination-programme/`](archive/2026-11-coordination-programme/) holds the July to
October 2026 coordination programme (EXP-01 to EXP-17, the coordination lab and design studies).
[Read the archive note first](archive/2026-11-coordination-programme/ARCHIVED.md): it explains why it
was archived and the limits of the often-quoted EXP-17 result.
