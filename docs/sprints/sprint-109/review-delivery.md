# Review delivery and the displayed candidate

Current-state audit after the first deployments, updated for displayed review versions:

- `owner_review::OwnerReviewInput` now requires the displayed handoff input fingerprint, Work
  revision and selected artifact reference. Old clients and changed versions receive a conflict
  without recording a decision. Acceptance also refuses a missing candidate reference.
- `InboxDetail` holds the displayed review until the owner explicitly chooses “Review new version”.
  A background refresh does not replace its explanation or reset an interactive preview. That
  control reserves its final geometry. Work summaries link to the candidate in Inbox instead of
  offering a second, unseen Accept button.
- `OrgIntel::decide_review_as` already locks Work and handoff and checks reviewer assignment,
  running Attempts, and the authored brief's fingerprint against current source/Work revision.
  These checks remain. The new displayed token is compared inside those same locks. Candidate
  publication takes the Work lock too, so it cannot replace the selected reference during a decision.
- Runtime review tickets have their own exact target/open path. A ticket currently answers with
  only its URL and expiry. Mutable file/URL contents need explicit consideration as well as a
  newer Work revision: a path alone does not identify the version that was read.

A4 acceptance must extend the existing path, not introduce a second artifact lifecycle:

1. A newly delivered review has a reachable, renderable candidate for that Work/Attempt. Missing
   targets return to the author for preparation. Existing broken reviews are repaired too.
2. The displayed candidate and decision share an explicit version. A different revision, replaced
   candidate or refreshed brief refuses an old tab's acceptance and asks for another review.
3. An inaccessible candidate says why and preserves the decision; a raw link or evidence fold is
   not evidence that the output actually rendered.
4. Exercise documents, runtime text/files and interactive pages, including two clients, a changed
   file at the same path, a newer Attempt, an unavailable computer and revoked access.
5. Feed the same version guard into C1's delayed acceptance; the window must not silently approve
   a candidate that changed while Undo was available.

The real PostgreSQL escalation suite exercises refreshed explanations, wrong revisions, replacement
references, publication waiting for the decision lock, wrong reviewers, concurrent acceptance and
returning a missing candidate for changes. The real two-client browser scenario is
`web/scripts/verify-review-versions.mjs`; desktop and phone retain the displayed interactive state,
the old version receives 409, the explicit refreshed version accepts once, and an old tab cannot
replay it. This reuses the existing handoff token and artifact reference; there is no new custody
lifecycle or durable acceptance entity.

A4 remains open: a mutable file at the same locator can change without replacing its artifact
reference; candidate reachability and rendering still need to be enforced during delivery, and
existing broken cards need repair. Native document bodies have their own review-version path and
need the same full scenario audit. C1 remains dependent on carrying the displayed version through
its durable Undo window. None of those requirements is established by the narrower checks above.
