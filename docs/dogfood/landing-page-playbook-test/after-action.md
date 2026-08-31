# Landing-page playbook dogfood after-action

Status: dogfood execution complete; exact local candidate is awaiting owner outcome review.

## What worked

- The accountable lead formed a quality-complete five-person team: product/copy strategy, art
  direction, Astro/React motion engineering, independent comparative criticism, and the lead.
- The shared playbook and repository supplement were present in actor skill roots, while the
  Restless Cloud repository and Cult UI source were pinned to exact commits.
- The lead held creative work behind a rendered product/incumbent baseline rather than accepting
  source files or prose as a visual reference.
- When the first product screenshots were blank, the lead rejected them and kept downstream work
  blocked. After the account-plane process disappeared, durable recovery preserved valid
  uncommitted work and resumed it in an attributable attempt instead of discarding it.

## Defects found and debugged

### 1. Reference proxy forwarded browser identity headers unchanged

Symptom: the first product-reference manifest recorded zero body-text characters, eight HTTP 403
resource responses per profile, a dynamic-import failure, and blank screenshots of roughly 6 KB.

Cause: the loopback proxy rewrote the request destination but forwarded the browser's `Origin` and
`Referer` headers for port 4310 to the cockpit on port 7788. Dynamic application assets were
therefore rejected.

Repair: rewrite `Origin` to `http://127.0.0.1:7788` and rewrite the `Referer` host before forwarding.
Independent recapture then returned HTTP 200, substantive body text at both viewports, and zero
console/page errors. The producer's final evidence was committed at
`9c766cb1f40e8a30c93bad8aeef9be34f9049b0c`.

Process change: a rendered-reference gate must assert nonblank pixels, nonzero body text, successful
critical resources, and zero consequential console/page errors before any dependent Work starts.

### 2. Baseline Work mixed too much capture and narrative work in one model session

Symptom: the first baseline session reached 144,620 context tokens and the auth gateway returned
HTTP 413 `Payload Too Large`, despite the configured model reporting a larger nominal context size.
The same failure reproduced in the direction study at only 102,287 context tokens, after the actor
had already committed the substantive study and created clean replacement captures. The operative
limit is therefore both materially lower than the advertised model context and not a reliable fixed
threshold. It later affected the long-lived accountable lead at 218,595 tokens while selecting from
the completed study, before a production Work was durably created, and the full-site producer at
99,994 tokens after implementation and repeated matched browser captures but before bounded content
feedback could be applied. The baseline's immutable retry also could not start from the original
source because an attributable draft had already been committed.

Repair: the lead preserved the exact draft commit, abandoned the exhausted Work, and created a
bounded completion Work rooted at that commit for only the missing rendered-product evidence.

Process change: split high-volume browser capture from synthesis. A capture Work should emit compact
manifests and selected images; a fresh narrative Work should consume those artifacts and write the
baseline. Treat any taste-critical Work that mixes implementation, repeated screenshots, long
browser manifests, narrative, and revision as too large even if the nominal model context says it
fits. Rotate the accountable lead into a reconstructed single-purpose wake after baseline acceptance
and again after direction acceptance; durable coordination does not require keeping the entire run
transcript in provider context. Nominal model context is not the operative auth-gateway request
ceiling.

### 3. Same-Work repair conflicts with immutable source after a mid-Work commit

Symptom: both baseline and direction recovery first attempted to resume the original Work after the
producer had made an attributable commit. Runtime correctly refused because the preserved workspace
no longer matched the Work's immutable requested source; after two Attempts the node could not be
resumed at all.

Repair: retire the exhausted Work and create a minimal continuation rooted at the exact substantive
commit, with the preserved worktree attached as recovery evidence and dependencies rewired to the
new completion node. This preserved attribution without regenerating the concepts.

Process change: `work repair` should either support an explicit attributable rebase-to-produced-
commit transition, or immediately recommend/create the minimal continuation pattern when source
integrity makes same-node retry impossible. Do not spend a second Attempt on a deterministic source
mismatch.

### 4. The default account-plane daemon disappeared while a producer was active

Symptom: the company container, Chromium, browser broker, and reference proxy remained healthy, but
the default `restlessd` socket was stale and the producer process vanished without a semantic result.
Only a separate experiment plane was still running.

Repair: restart the same default account plane. On startup the daemon emitted
`attempt_process_ended` with `semantic_result: unreported`, attached a recovery capsule containing
the changed workspace, and woke the lead. The lead inspected the preserved changes and repaired the
same Work for a second attributable attempt, which committed cleanly.

Process change: account planes need explicit lifecycle isolation and ownership so an unrelated run
cannot displace the default plane. Active companies also need a visible daemon-health sentinel and a
first-class resume command. A surviving company container is not proof that orchestration is alive.

### 5. Development chrome contaminated visual evidence

Symptom: the first direction captures included Astro's development toolbar. The concepts themselves
were usable, but the screenshots were not admissible evidence of the public experience.

Repair: rebuild and recapture from a production preview. Add an exact toolbar detector rather than a
text search; an early broad detector also misidentified ordinary page links as framework chrome.

Process change: taste-critical review must use a production-equivalent preview. Evidence gates should
identify known framework UI by stable selectors or structure, not by common visible words.

### 6. Public output inherited a retired cockpit interaction

Symptom: a direction exposed `EXEC / ATTENTION`, even though the owner had explicitly retired the
Chat/Attention split. The stale affordance came from treating the rendered product as an undifferentiated
reference rather than separating durable design language from known product defects.

Repair: remove the interaction and record it as a negative constraint in the product-reference packet.

Process change: a source-of-truth capture must contain both an inheritance list and an exclusion list.
Visual-language fidelity does not authorize resurrecting superseded controls.

### 7. Acceptance gates were declared after scheduling had begun

Symptom: one bounded correction was interrupted and abandoned before its writes could count because
the exact acceptance gates were still being changed around an already scheduled Work node.

Repair: create a replacement continuation from the exact clean commit only after the complete gate set
was active. The replacement then verified the native build, article depth and measure, residue, links,
visibility, overflow, runtime health, and matched browser records without changing the source tree.

Process change: freeze the full gate contract before scheduling a producer. A lead may add a stricter
downstream independent review, but must not mutate the producer's admission contract during execution.

### 8. Supervised review targets must be proved live, not merely described

Symptom: earlier site iterations produced links that remained on “Opening the outcome…” because the
recorded URL did not resolve to the supervised process that actually served the candidate.

Repair: the verification continuation registered the exact candidate under the container supervisor,
restarted it, and probed the reserved loopback endpoint before attaching the ReviewTarget.

Process change: a ReviewTarget gate requires a supervised process, exact commit/tree coordinates, an
HTTP success probe, and the owner-facing projected URL. A localhost string in prose is not a review
artifact.

## Product-reference caveat

The rendered current cockpit is the visual-language source of truth, not an instruction to copy all
of its current product decisions. The captured product still visibly contains an `Attention` tab and
linked-attention affordance that the owner had previously asked to purge. Public-site work should
inherit its spatial canvas, typography, geometry, material, and semantic-color language while
excluding that stale interaction model.

## Final evaluation

- All three direction pairs were inspected before Company Atlas was selected.
- Candidate source `f0ef039e50475070f892997c807ffe1ab1f57325` (tree
  `6790053318056283598dcc7e7cd9d42ae5c68a3e`) passed its native build and exact verifier.
- The independent critic operated every required route at `1440x900` and `390x844`, supplemented by
  full-page and reduced-motion review, and returned the only admissible winning verdict: `better`.
  Its preserved report commit is `f8ae8a8f18e8fdbeb5d22cda66453c17d9b56cd6` (tree
  `812977cd27060b4aaa0bb68322d3ba93267ba094`).
- The exact candidate is supervised and live at container loopback port 4354. The owner-facing review
  projection was created and independently returned HTTP 200 before handoff.
- The accepted history is preserved on host branch `dogfood/landing-playbook-candidate` at
  `/Users/yao/Learning/restless/site-candidates/landing-playbook-candidate`. A clean host install,
  Astro build, and exact evidence verifier all passed.
- No publish, push, deployment, hosting change, or replacement of `main` occurred. Owner acceptance
  closes only the isolated local candidate outcome.

The critic's non-blocking cautions are to avoid another reuse of the Atlas motif, keep the mobile
encounter's meaning close to the first view, and never reduce the small mono metadata further. Its
declared gaps are Safari, Firefox, high-DPI rasterisation, screen-reader speech, production hosting,
and viewports beyond the two required profiles.
