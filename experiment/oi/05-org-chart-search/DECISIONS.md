# Decisions the plan left open

Each of these was settled one way in code so the suite could be built. None is accepted yet. The owner decides.
The plan itself was not changed.

1. **Research tasks are questions over invented, frozen documents, not live web questions.**
   Why: METHOD.md asks for frozen, hashed inputs and an answer key that stays right. A live web answer can change between runs.
   Cost: the tasks test careful reading and cross-checking, not web search. Other choice: real questions over a saved set of real pages, which needs someone to collect and license the pages and write keys by hand.

2. **A research answer passes only if its list of names equals the key exactly.**
   Recall and precision are recorded as well. Other choice: pass at some recall and precision threshold.

3. **A code task's hidden tests are the tests the real change shipped with.** No extra tests were written.
   Cost: a correct solution with different names fails. To soften that, the brief shows the lines the tests use to load the code.

4. **A code brief is the real change description with its paragraphs about checks removed.**
   Cost: some briefs name the solution's parts. Other choice: rewrite each brief as a bare request, which is prose the owner or a separate worker would write and then check.

5. **10 of the 30 code tasks are Rust, and their tests were never run here.** The machine has no `cargo`.
   The other 20 were run before and after the real change. 7 reserves, all runnable with `node`, are in `code/selection.json`.
   Other choice: use 30 JavaScript tasks and lose the Rust side of the product from the suite.

6. **In two code tasks that touch Rust or scripts and the web app, only the JavaScript tests grade.** (`code-17`, `code-27`)

7. **Operations jobs are the bank's resale and email tasks, referenced, not copied.** A bank task has four versions. All four stay on one side of the split. Which version an arm sees is for the run design.
   Note that experiment 2 uses the same bank. If its runs come first, those tasks are not new to anyone tuning org charts by hand, though the search itself has not seen them.

8. **Forecast slots: 4 categories by 5 slots, each price band once and one band twice.**
   Open problem: an auction ends within a week. Stage 2 and stage 3 run at different times, and the eight candidates in one stage should face the same auctions. So a slot can hold one auction per run window, and all candidates in that window must forecast it before it ends. Scores wait for the auctions to close. The owner should confirm this reading, or drop to fewer forecast tasks per window.

9. **The split is random within each family, not ordered by time.**
   Result: for 17 of 18 search code tasks, the starting snapshot already contains at least one held-out change (54 pairs). The held-out tests are not in any snapshot's hidden part and the brief is not there, but the held-out solution code is, as ordinary repository code.
   Why accepted for now: a search that tunes org charts does not read task solutions, and a held-out task still starts from its own earlier snapshot, where its solution is absent.
   Other choice: split by time, with held-out tasks all later than search tasks. That removes the overlap but makes held-out tasks differ from search tasks by date, and it cannot also be balanced by difficulty as freely.

10. **The split is a draft until calibration.** The plan wants it balanced by difficulty. `split.py --labels` does that; `validate.py --final` refuses a draft.
    With labels, hard tasks are spread 60/40 per family as closely as whole tasks allow. Research worlds move as pairs, so the balance there is by pair.

11. **Split sizes per family are 18/12, 18/12, 12/8, 12/8.** The plan says 60/40 overall and does not say per family.

12. **This stage was done by one worker.** The pieces share one validator and the machine had about 2 GB of memory free, so no parallel staff were used.
