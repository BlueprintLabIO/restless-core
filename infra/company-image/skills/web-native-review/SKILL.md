---
name: web-native-review
description: Prepare or independently review a rendered website against its outcome contract and a mature ground-truth reference. Use for landing pages, product sites, blogs, documentation sites, or other browser-native outcomes; do not use to judge source code alone or turn mechanical layout facts into a taste score.
---

# Web native review

The browser outcome is the evidence. Source, build logs and screenshots support review but do not
replace operating the live site.

## Producer: prepare the exact candidate

1. Read the outcome contract, the product's own rendered design/source dossier and any external
   ground-truth reference before choosing page structure. When a real product already exists, it is
   the primary source of visual language; an external reference calibrates maturity only and a
   component library supplies mechanics only.
2. Serve the candidate as a durable project service under `/company/services/supervisor`, with
   `autorestart=true`. A detached shell process is not a prepared ReviewTarget. Run `supervisorctl -c
   /etc/supervisor/conf.d/restless.conf reread`, `update`, and `status`, then probe the exact loopback
   URL.
3. Capture every primary route with:

   ```sh
   restless-web-review \
     --url http://127.0.0.1:<port>/ \
     --output /company/outputs/<work>-web-review \
     --route / \
     --route /product/ \
     --reference-url https://<ground-truth-reference>/ \
     --reference-route /
   ```

   Add every route named by the outcome. The tool scrolls through each page before capturing it and
   records deterministic browser facts in `manifest.json`, including computed typography, palette,
   geometry, motion, reading-surface width and visual-element counts; it never approves design
   quality or calculates a similarity score.
4. Inspect the images and manifest yourself. Repair observable breakage before requesting review.
   Link the exact live URL as the ReviewTarget and the capture directory/manifest as supporting
   evidence. Do not attach an annotated URL or implementation note as the target.

## Independent critic: buy an informative judgement

Receive the exact outcome contract, live candidate, capture manifest/images and dated reference
dossier. When the product exists, also receive representative rendered product states and exact design
sources. Compare the candidate to the product before comparing it to an external maturity reference.
Do not request or read the producer's rationale, private reasoning or persuasive completion summary
before forming the critique.

Operate the live site at desktop and mobile widths. Inspect at least:

- first-view thesis and product comprehension;
- the complete scroll narrative and whether each section earns its place;
- real product demonstration versus decorative effects;
- typography, line length, hierarchy, rhythm and edge alignment;
- fidelity to the product's type, semantic colour, geometry, material and motion grammar;
- primary navigation, keyboard focus, mobile navigation and link destinations;
- loading/reveal/count-up behaviour, reduced motion and content without animation;
- standalone content routes and related navigation;
- footer information architecture and closing conversion path; and
- the exact deterministic observations in `manifest.json`, including overflow, off-viewport controls,
  invisible authored content, computed reading width/design evidence, console errors and broken
  internal links.

Compare product fidelity first and maturity second. Name where the candidate follows or contradicts
the product, then where it closes or misses the external reference's standard of narrative depth,
proof, responsiveness and finish. Reject invented marketing identities, copied references, fake
metrics, component-library collage, short pages that leave the visitor's question unanswered, and
visual spectacle standing in for product truth.

Return one verdict: `accept`, `changes_requested` or `reject`. For changes, consolidate only
consequential defects with exact route, viewport and observed evidence. Do not edit the candidate.

## Accountable lead: judge the whole outcome

Treat a clean manifest and green build as checkable evidence, not taste approval. Inspect the exact
post-revision live site and critic report. If it misses the outcome, create or revise attributable
producer Work. Escalate to owner review only when the site is healthy, the critic's consequential
findings are resolved or explicitly disagreed with from rendered evidence, and the remaining question
is genuinely owner taste or publication judgement.
