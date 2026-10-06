# S61-T4 — Show each check can fail

**Layer:** Verification. **Serves:** acceptance 2. A smoke that has never failed is not yet
evidence (CLAUDE.md, "A check that happens to pass is not evidence").

For assertions 1, 3, 5, 7, 8, 11 and 12, make a throwaway local patch that breaks exactly the
guarded behaviour. Examples: skip the first-contact check, leak the token into the Runtime
environment, drop the company filter on the gateway path. Run the smoke and confirm that assertion,
and ideally only that assertion, fails. Record patch, assertion and output in this ticket. Do not
commit the patches.

**Deletes:** nothing.

## Results (6 October 2026)

Each patch was applied to the worktree at `5d4edc7`, built, run with `--only` and reverted. Every
patch failed exactly its target assertion and nothing else. Evidence: `t4-results.json` and one
`t4-<patch>/evidence.json` per patch.

| Patch | What it broke | Assertion | Observed |
|---|---|---|---|
| P1 | redirect URI one path segment off the callback route | 1 | connection stayed `awaiting_sign_in` |
| P3 | gateway refuses callers on the Docker bridge | 3 | `tools/list` 403 from the company computer |
| P5 | declared parties ignored | 5 | first contact sent without approval |
| P7 | a lost response treated as a refusal | 7 | lost send reported as a plain failure, not unknown |
| P8 | freeze ignored | 8 | send succeeded while frozen |
| P11 | access token written into the Runtime | 11 | `/tmp/.leak` found by the token search |
| P12 | gateway stops checking the capability's company | 12 | cross-company call 200/200 |
| P-refresh | pooled sessions ignore token expiry (the defect T3 fixed) | 10 | read failed after the provider's token expired |
