# Sprint 28 blinded-reader exercise

Use a reader who has not seen the implementation or the source fixture. Do not explain Restless
terms before the exercise. Record their words, including mistakes, before opening evidence or the
source-answer sheet in [`reader-corpus.md`](reader-corpus.md).

## Setup

1. Start the controlled reader fixture and point the web app at it:

   ```sh
   cd web
   node scripts/serve-reader-fixture.mjs
   RESTLESS_OWNER_URL=http://127.0.0.1:7787 npm run dev
   ```

2. Open `http://localhost:5173/restless_cloud_quality_enforcer_test` at desktop width.
3. Repeat the focused item at 390px width and using keyboard navigation only.
4. Do not activate a decision. The fixture is read-only and cannot perform a source operation.

## Uncoached questions

Ask these questions for the approval, outcome review and provider identity-check items:

1. What changed?
2. Why does it matter?
3. What does the company recommend?
4. Does the owner need to do anything? If so, what exactly?
5. What will each visible control do?
6. What happens if the owner does nothing?
7. What is uncertain?

Then open **Company → Decision history** and ask:

8. What did the owner decide?
9. What did that release?
10. Who owns the next step?
11. What result has actually been observed, and what remains pending?

## Record

| Case                  | Width/input    | Orientation time | Decision-account time | Reader's account | Mistakes or missing facts | Pass |
| --------------------- | -------------- | ---------------: | --------------------: | ---------------- | ------------------------- | ---- |
| Approval              | Desktop        |                  |                       |                  |                           |      |
| Approval              | 390px/keyboard |                  |                       |                  |                           |      |
| Outcome review        | Desktop        |                  |                       |                  |                           |      |
| Outcome review        | 390px/keyboard |                  |                       |                  |                           |      |
| Human last mile       | Desktop        |                  |                       |                  |                           |      |
| Human last mile       | 390px/keyboard |                  |                       |                  |                           |      |
| Decision continuation | Desktop        |                  |                       |                  |                           |      |

Pass only when the reader's account agrees with the source facts in `reader-corpus.md`. Product terms
are not required. A wrong consequence, missed owner need or upgraded pending result is a failure; do
not coach it away. Five seconds is the orientation target and ten seconds is the decision-account
target.
