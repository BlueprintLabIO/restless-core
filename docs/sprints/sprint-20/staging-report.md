# Sprint 20 — staged publication report

**Recorded:** 26 August 2026
**Terminal state:** `staged_candidate`, not `accepted_publication`

**Candidate revisions:** `fc8f97d1e1507489023f93574ccf15257a61bf55` constructs the publication;
`befb4b8429774fcfa1e3c05b03e9b1336f445d52` adds corpus traceability enforcement.

## What was observed

| Check | Input | Observed result |
| --- | --- | --- |
| Static quality and type check | `cd site && npm run verify` | The corpus guard and quality gate passed; `astro check` reported 0 errors, 0 warnings and 0 hints; the static build produced 18 routes. |
| Candidate routes | Local `astro preview` on `127.0.0.1:4324` | `/`, `/product/`, `/how-it-works/`, `/research/`, `/research/corpus/`, `/journal/`, one article route and `/compare/` each returned content. |
| Corpus truth | `/research/` local response | It reports `6 of 7` publication coverage and names the EXP-07 result as deferred. |
| Legacy continuity | `/findings/` and a historical findings article route | Each returns the explicit static redirect document to `/journal/`; no prior content is served. |
| Source hygiene | Site source search and deleted imports | No remaining import referenced the obsolete bridge/matrix components, config or findings collection. |

## Publication shape

The staged candidate contains the product explanation, one outcome walkthrough, a research index,
corpus map, dated category comparison and five source-linked journal articles. It deliberately makes
EXP-06's inconclusive comparison and EXP-07's sealed blind decision visible rather than converting
them into product proof.

The corpus guard verifies that each article's source locator exists, that EXP-01 through EXP-07 each
have one coverage record and that every published home resolves to a journal article. It protects the
evidence map without trying to judge the prose or replace the required fresh peer review.

## Removed implementation

- The old findings collection, its `ArticleList` template and four obsolete Markdown entries.
- Unreferenced bridge/matrix decorative components and the old public-site config.
- The old editorial stylesheet and duplicate public visual identity.

The retained `/findings/` routes are compact compatibility redirects, not a second implementation.

## Not observed and not claimed

- No model writer, publication lead or fresh peer reviewer ran; no article has a S20 acceptance
  attribution.
- No connected interactive browser was available for the required rendered desktop/mobile review.
  Browser-guidance selection reported no available browser connection, so this report does not call
  the visual/native-review gate passed.
- No owner inspected the prepared native target, chose a publication posture, or granted a deployment
  effect.
- No deploy, hosted-site replacement, branch promotion, push, announcement or distribution occurred.
- No spend ceiling is recorded for the actual S20 model run, and none is assumed from another sprint.

## Required next governed action

Set an exact S20 model spend ceiling, then run the frozen publication organisation in an isolated
company: one non-producing lead, one calibration writer, a fresh peer reviewer with source-only
context and a rendered candidate review. Only after that evidence and an owner-native review may a
separate deployment effect be prepared.
