# Editorial charter

## Objective

Maintain one pointed research article about the tested boundary of autonomous team capacity. Change
the article only when material evidence changes the useful conclusion.

## Truth boundary

- Controlled signals are `_test` evidence.
- Separate observation, inference, owner decision and unknown.
- Preserve negative results and model/provider conditions that change the conclusion.
- Never infer adoption, revenue, customer demand or universal performance.
- A source can justify only the claim and scope it actually contains.

## Article bar

The article must have one sharp thesis, a narrative of expectation versus observation, exact evidence,
the best current explanation, practical consequence, counter-case and limits. It should read as an
argument rather than an experiment log. Public prose contains no em dash.

## Event handling

Every observed source event is recorded once in `publication/event-ledger.json` with:

- `source_ref` - stable idempotency identity;
- `observed_at` - source observation time supplied by the event;
- `disposition` - `publish`, `update`, `defer`, `no_op_duplicate` or `no_op_irrelevant`;
- `reason` - bounded factual judgement;
- `artifact` - repository-relative article/research-note path, or `null` for a no-op; and
- `accepted_commit` - exact accepted commit when an artifact changed, otherwise `null`.

Duplicate delivery may be represented by one additional `no_op_duplicate` ledger row, but it must not
create another article, research note, review or content-changing commit.

## Review and promotion

For `publish` and `update`, a Staff writer owns the content end to end and a different fresh Staff peer
reviewer inspects the exact draft against the source. The reviewer writes an attributable decision in
`publication/reviews/`. The lead judges the native Markdown article and evidence, then may
mechanically fast-forward the exact accepted commit to `main`. The lead does not rewrite.

`defer` may create one bounded research note explaining the conflict and next evidence needed. An
irrelevant or duplicate event creates no substantive artifact.
