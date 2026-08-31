# Sprint 22 - Ground-truthed native web outcomes

**Status:** Owner rejected revision 1 on 28 August 2026; superseded by Sprint 23

**Date:** 28 August 2026

**Depends on:** Sprint 17's non-producing lead and native owner review path; Sprint 19's
scenario-evidence precedent; the Cloud-owned public source; the observed failed candidate at
`restless-cloud@627d8ca0a7c42e5d8deda1413a00fd192bcd7867`.

**Spec refs:** `ARCHITECTURE.md` §4, §5, §7, §11 and §16;
`docs/COORDINATION_THEORY.md`; `docs/specs/evaluation-dogfood.md`;
`docs/FRONTEND_DESIGN_REFERENCES.md`; Restless Cloud `docs/specs/public-surface.md`.

## Why this sprint exists

The first Restless-produced Cloud redesign built, served six routes and attached twelve screenshots,
yet the screenshots themselves showed a colliding headline, a collapsed orbit diagram, clipped mobile
navigation, counters frozen at zero, invisible reveal content, large blank regions, index-only Blog
links and an implementation-led footer. The review target also accepted prose appended to a URL and
its detached preview died before owner review.

The failure was not absence of evidence. It was evidence that no independent judgement meaningfully
consumed. Restless optimised artifact closure and route reachability while the owner asked for a
mature, substantial website. This sprint closes that exact gap without asking deterministic software
to judge taste.

## Outcome

Restless gains one reusable, source-first web review capability that:

- captures the complete rendered candidate after exercising its scroll and interaction states;
- records desktop/mobile layout facts and route/link failures without pretending to score taste;
- places a mature ground-truth reference beside the candidate for a fresh reviewer;
- keeps the producing context out of that review;
- keeps the preview alive through the Runtime's existing process supervisor; and
- proves the improvement by rebuilding the Cloud landing page and wider public site to a Neon-calibre
  maturity bar while retaining a distinct Restless identity.

The final owner ReviewTarget is the live local Cloud candidate. The capture manifest, reference dossier,
independent review, exact commit and gates are supporting evidence. No deployment, push or public
replacement is authorised by this sprint.

## Ground-truth reference contract

The primary maturity reference is [Neon](https://neon.com/) as observed on 28 August 2026. It is a
quality and completeness reference, not a visual template. The team studies:

- a decisive hero with a concrete product surface;
- sustained narrative depth rather than one visual per route;
- alternating product demonstration, technical explanation, proof and trust;
- clear primary and secondary conversion paths;
- complete responsive navigation and a useful multi-column footer; and
- polish that survives the entire page, not only the first viewport.

Restless must not copy Neon's branding, copy, illustrations, layout sequence or product claims. The
reference dossier names the principle transferred and the Restless-specific expression chosen.

## Frozen organisation

```text
owner
  -> available Exec
      -> Web Product lead - non-producing integrator and final native judge
          -> Web producer - owns the complete Cloud candidate
          -> independent web critic - fresh Work, withheld producer reasoning
```

The critic receives the outcome contract, ground-truth dossier, exact live candidate and generated
capture evidence. It does not receive the producer's rationale or private implementation context. It
may accept, request changes or reject. Any content-changing repair returns to the producer through the
ordinary `revises` edge; the critic and lead never repair the candidate privately.

## Success contract

### Restless capability

1. `restless-web-review` is installed in the company image and can capture a supplied route set at
   desktop and mobile widths with a versioned manifest.
2. Capture scrolls through each page before the full-page screenshot, waits for fonts and settled
   layout, preserves console/page errors, and records horizontal overflow, off-viewport interactive
   elements, invisible authored content, internal link destinations, response status and page height.
3. The tool reports deterministic observations only. It never converts page height, heading count,
   colour, component count or similarity to a reference into an acceptance verdict.
4. The companion skill requires a fresh reviewer to inspect the rendered candidate against the
   reference and outcome contract, including desktop, mobile, navigation, loading, reduced-motion,
   footer and standalone-content behaviour.
5. A web ReviewTarget is served by a named configuration under `/company/services/supervisor`, is
   live-probed before handoff and returns after a supervised restart without owner repair.
6. Review-target URL validation refuses annotations and surrounding whitespace; the owner ticket
   opens the exact live candidate or returns an actionable failure instead of an indefinite loader.

### Cloud outcome

7. The current defective candidate is captured as the before-state and the critic identifies the
   visible defects above from rendered evidence rather than source narration.
8. The rebuilt homepage has a complete narrative arc: thesis, operating loop, actual product
   encounter, capability stories, real workflow examples, evidence/trust, comparison or objection
   handling, Blog/Research entry points and a closing conversion section.
9. Product, How it works, Research, Blog and Compare each have an independently useful page purpose
   and sufficient content/visual variation to answer that page's visitor question. Page depth is
   judged from narrative completeness, never a minimum pixel height.
10. Every Blog entry has a standalone readable route with coherent long-form typography and related
    navigation. The public vocabulary is Blog, not Journal.
11. The footer is a professional information surface with product, resources, company/contact and
    legal/status destinations. Source-library attribution moves to a discreet credits record and does
    not dominate customer-facing chrome.
12. Desktop and mobile captures have no hidden authored sections, frozen placeholder counters,
    headline collisions, collapsed diagrams, clipped primary navigation, horizontal overflow, broken
    internal route or console/page error.
13. A fresh independent critic reviews the exact final candidate and may force at least one bounded
    revision. The accountable lead then inspects the whole native site before preparing owner review.
14. The final exact Cloud commit passes clean install, Astro checks/build, internal-route checks and
    the new capture run. The owner receives one live candidate and one bounded accept/request-changes
    decision.

## Evidence package

1. frozen Core and Cloud starting revisions plus the defective before captures;
2. dated Neon reference dossier with transferred principles and explicit non-copy boundary;
3. web-review tool manifest and desktop/mobile screenshots for every primary route;
4. deterministic build, route, link, overflow, visibility, console and supervised-preview results;
5. producer Work/Attempt, critic Work/Attempt, `requires`/`revises` lineage and lead judgement;
6. exact final Cloud commit and live ReviewTarget; and
7. after-action separating candidate, model judgement, Runtime, OrgIntel and owner-surface friction.

## Layer slices

| Concern | Owner | Sprint responsibility |
| --- | --- | --- |
| Review responsibility and independence | OrgIntel | Use ordinary Work/Attempt and revision edges; preserve a fresh critic boundary and exact acceptance evidence |
| Capture, browser, project service and files | Company Runtime | Add one reusable web-review skill/tool and use the imported supervisor for the live candidate |
| URL and owner ticket reliability | Runtime Bridge / owner surface | Preserve strict bare targets, health-probe tickets and actionable unavailable state |
| Website source and content | Restless Cloud | Rebuild the public outcome in the Cloud-owned repository |
| Taste and final publication decision | Owner | Judge the exact native candidate; no public effect is implied |

No new Kernel entity, design score, screenshot custody lifecycle, workflow engine, CMS or universal
review state machine is introduced.

## Risks and dispositions

| Risk | Disposition | Treatment |
| --- | --- | --- |
| Reference use turns into a Neon clone | **Guarded** | Transfer named maturity principles; require a distinct Restless signature and explicit non-copy critique |
| Mechanical checks become a fake taste score | **Guarded** | Tool emits observations only; fresh reviewer and owner retain judgement |
| Same-model reviewer echoes production | **Guarded** | Withheld producer context, adversarial mandate, exact rendered target and rejection authority; correlation is reported |
| Richer becomes text-heavy again | **Guarded** | Narrative depth comes from product encounters, diagrams, proof and varied structures, not repeated prose blocks |
| Preview dies after handoff | **Guarded** | Existing mature supervisor owns the project service and restart behaviour |
| Public replacement happens during local improvement | **Invariant at effect boundary** | No push, deploy or external replacement without a separately authorised effect |

## Ticket checklist

Status lives only here.

- [x] [S22-T0 - Freeze the failure and reference dossier](sprint-22/t00-freeze-reference.md)
- [x] [S22-T1 - Add scroll-aware web native review](sprint-22/t01-web-native-review.md)
- [x] [S22-T2 - Prove independent review and supervised handoff](sprint-22/t02-independent-review.md)
- [x] [S22-T3 - Rebuild the Cloud public site](sprint-22/t03-cloud-rebuild.md)
- [x] [S22-T4 - Critique, revise and verify the exact candidate](sprint-22/t04-critic-verification.md)
- [x] [S22-T5 - Prepare owner review and after-action](sprint-22/t05-owner-review.md)

## Entry, stop and exit gates

**Entry:** founder direction in this thread authorises local Core improvement and local Cloud candidate
work. The Core worktree's unrelated dirty files remain owner-owned. Cloud begins clean at `d3c405a`.
Model spend requires an explicit company ceiling before a Restless run. No public effect is authorised.

**Stop:** stop for source/private-data uncertainty, an unavailable exact model route with no admitted
fallback, uncontrolled public effect, branch corruption, inability to preserve the clean Cloud baseline
or founder stop. Reviewer rejection and candidate revision are expected evidence, not stops.

**Exit:** the capability is installed and exercised; the before candidate fails for the observed
reasons; one Restless-run Cloud candidate meets every success item with exact evidence; a supervised
live ReviewTarget opens; and the owner receives the final bounded judgement.

## Completion audit — 28 August 2026

| Contract | Authoritative current evidence | Result |
| --- | --- | --- |
| 1–4. Reusable factual web review and fresh human judgement | Installed `/usr/local/bin/restless-web-review`; installed skill; both final manifests declare `restless.web-review/v1`, scroll exercise, font wait, three profiles and `acceptanceVerdict: none; deterministic observations only` | Proven |
| 5. Supervised live outcome | `restless-cloud-public` returned to `RUNNING` after a deliberate supervisor restart and the in-company target returns HTTP 200 | Proven |
| 6. Exact openable review URL | Strict bare-loopback validation tests pass; repeated-attempt selection regression passes; the isolated rebuilt owner surface projects `runtime-web / available`; an issued opaque ticket fetched the homepage with HTTP 200 | Proven |
| 7. Failure frozen and consumed | Reference dossier and `s22-before-web-review` preserve the old candidate; the independent brief names the visible mobile, capture, route and footer failures | Proven |
| 8–11. Mature complete public surface | Exact Cloud commit `7e17ef0`; 8,763px desktop homepage; distinct Product, How it works, Research, Corpus and Compare routes; five standalone Blog routes; credits/legal records and multi-column footer | Proven |
| 12. Responsive/rendered integrity | Both producer and lead manifests contain 45 candidate plus two reference states: zero bad status/navigation, overflow, off-viewport controls, invisible content, console/page errors or failed internal links | Proven |
| 13. Independent review and accountable lead | OrgIntel preserves producer `6d31ba3a`, critic `83295716`, stabilization `fc1665bc`, `requires`/`revises` edges and critic verdict `accept`; lead did not rewrite the accepted commit | Proven |
| 14. Exact build and owner handoff | Host Cloud is exactly `7e17ef0`, one local commit ahead of origin; portable verification builds 21 pages with zero Astro diagnostics; handoff `08a0f84e` contains one current accept/request-changes judgement | Proven |

The outcome remains deliberately unpublished. A pending owner verdict is product state, not unfinished
implementation: the sprint prepared and opened the exact bounded decision but did not decide it for
the owner.

## Owner review correction — 28 August 2026

The owner rejected the exact candidate after native review. The deterministic evidence above remains
true, but it did not prove the design outcome. Three consequential gaps survived:

1. the site still depended too heavily on text and lacked enough graphical, kinetic and interactive
   product encounters;
2. standalone Blog content retained an unnaturally narrow editorial column; and
3. the cream/ink/acid marketing identity contradicted the actual Bridge Light product design system,
   which should have been the primary visual source of truth.

OrgIntel records `request_changes` on handoff `08a0f84e-e684-4269-a1d7-b855f3972ab7` and revision 2
of the stabilization Work. [Sprint 23](sprint-23.md) owns the corrective outcome. Neon is demoted from
primary reference to completeness calibration; the rendered Restless product is now authoritative for
visual language.
