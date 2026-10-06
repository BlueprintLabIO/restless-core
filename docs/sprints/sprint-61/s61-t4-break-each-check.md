# S61-T4 — Show each check can fail

**Layer:** Verification. **Serves:** acceptance 2. A smoke that has never failed is not yet
evidence (CLAUDE.md, "A check that happens to pass is not evidence").

For assertions 1, 3, 5, 7, 8, 11 and 12, make a throwaway local patch that breaks exactly the
guarded behaviour. Examples: skip the first-contact check, leak the token into the Runtime
environment, drop the company filter on the gateway path. Run the smoke and confirm that assertion,
and ideally only that assertion, fails. Record patch, assertion and output in this ticket. Do not
commit the patches.

**Deletes:** nothing.
