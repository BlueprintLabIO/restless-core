# Sprint 20 — frozen corpus and publication boundary

**Recorded:** 26 August 2026
**State:** staged construction evidence; not an accepted publication run

## Frozen starting point

| Item | Frozen reference | Why it matters |
| --- | --- | --- |
| Integration parent | `2bd5513c18a6c21bb582be462813d446247180d4` | The S16 completion checkpoint immediately before the staged site work. |
| Visual candidate | `experiment/exp-07-candidate-a` at `4e454bc2a36eab3afbf7d91a070e732cf657a847` | The founder-specified editorial direction. Its independent static build is a visual reference, not code imported into `site/`. |
| Staged candidate | `fc8f97d1e1507489023f93574ccf15257a61bf55` | The Astro publication construction checkpoint that receives this corpus and preserves legacy findings URLs as redirects. |
| Corpus integrity guard | `befb4b8429774fcfa1e3c05b03e9b1336f445d52` | The follow-up checkpoint that makes missing source locators or EXP coverage drift fail the site quality command. |
| Public boundary | Public experiment conclusions and exact repository locators only | No prompts, private reasoning, credentials, browser state, model transcripts or unsealed blind-candidate identity enter the site. |
| Effect boundary | Local build and preview only | No deploy, replacement, external message, Git push or public claim is authorised by this record. |

## Evidence map

| Record | Frozen source revision | Public home | Publication disposition |
| --- | --- | --- | --- |
| EXP-01 | `aa4eb2b0b8c609c7760db1cb0b89504a0d850a6f` | `/journal/one-worker-is-a-default-not-a-rule/` | Scoped provisional evidence. |
| EXP-02 | `aa4eb2b0b8c609c7760db1cb0b89504a0d850a6f` | `/journal/one-worker-is-a-default-not-a-rule/` | Scoped provisional evidence. |
| EXP-03 | `8b97fa94827ded27b2f305f155bfea9578dc7e3c` | `/journal/one-worker-is-a-default-not-a-rule/`, `/journal/a-lead-is-a-consequence-window/`, `/journal/an-evaluator-can-be-wrong-about-real-work/` | Accepted direction at the stated controlled-work scope. |
| EXP-04 | `8b97fa94827ded27b2f305f155bfea9578dc7e3c` | `/journal/capacity-has-an-arrival-shape/` | Scoped provisional evidence. |
| EXP-05 | `394f4b3ab70db8496f919aea4f3c804fe5856b2a` | `/journal/capacity-has-an-arrival-shape/`, `/journal/a-lead-is-a-consequence-window/`, `/journal/an-evaluator-can-be-wrong-about-real-work/` | Controlled `_test` evidence with its limits retained. |
| EXP-06 | `91ec7393b4786d750e0320041bb8adf4fbb31db6` | `/journal/a-comparison-needs-a-cleaner-start/` | Inconclusive comparison published as an open question. |
| EXP-07 | `d284487403a503b76c917087a91329a76f73458f` | `/research/corpus/` | Deferred: its blind owner decision and arm identity are sealed. The method is public; no winner is claimed. |

The article front matter in `site/src/content/journal/` keeps the exact source locators and the
scope each locator supports. The runtime corpus projection is `site/src/data/corpus.ts`.

## Editorial and model envelope

The S20 specification requires GPT-5.6 Sol with no fallback and an exact recorded spend ceiling for
the actual company run. No model run has started under this freeze, so no monetary ceiling is inferred
from another experiment and no reviewer or lead acceptance is attributed here.

## What this makes deletable

The staged site removes the obsolete bridge/matrix public-surface components, the old `findings`
content collection and its rendered article templates. Legacy URLs remain only as direct redirects to
the research journal; they do not preserve a second content system.
