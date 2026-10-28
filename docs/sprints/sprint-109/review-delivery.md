# Review delivery and the displayed candidate

Current-state audit after the first deployments:

- `owner_review::OwnerReviewInput` accepts only `decision` and `feedback`.
- `attention.ts::reviewAction` sends those two fields; `InboxDetail.decideReview` does not carry
  the version the reviewer actually saw.
- `OrgIntel::decide_review_as` already locks Work and handoff and checks reviewer assignment,
  running Attempts, and the authored brief's fingerprint against current source/Work revision.
  Preserve these checks. They reject a stale server brief, but do not prove a client displayed
  that current brief or candidate before pressing Accept.
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

No A4 completion is claimed by this audit. The current server-side checks are useful existing
machinery to retain and extend, not a replacement for displayed-version acceptance.
