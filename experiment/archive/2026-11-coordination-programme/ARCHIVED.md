# Archived: coordination programme (July to October 2026)

**Archived on 12 November 2026.** Everything in this folder is a historical record. Nothing here is
current guidance, and no result here should be quoted publicly without the caveats below.

The current programme is [Organisational Intelligence](../../README.md).

## Why it was archived

- The questions moved on. The old programme asked how Restless should route work inside one company
  (when to add Staff, how a lead supervises). The new programme asks what makes an AI organisation
  smart, and publishes the answers.
- The models moved on. Most runs used GPT-5.6 Sol or older models. Results about cost and coordination
  depend heavily on the model generation.
- Several results were very small samples (often one run per condition) and were written up in
  internal jargon that is hard to check from the outside.

Files keep their original contents. Links inside them were rewritten for the new location. Scripts are
kept as records; any path they compute assumes the old `experiment/` location, so they may not run
unchanged.

## What is still useful

- **The operating discipline.** Freeze tasks and graders before the first counted run, grade blind,
  keep harness and provider failures apart from organisational failures, and publish negative results.
  The new [method](../../oi/METHOD.md) keeps all of this.
- **Harness lessons from EXP-17.** Reviewer bundles were unreadable because of file permissions,
  concurrent reviewers sharing one identity broke metering, and blind review cost more than the runs
  being reviewed. Plan for all three.
- **The coordination canon** (`coordination/CANON.md`) as a list of hypotheses, not facts.

## EXP-17: what it does and does not show

EXP-17 is quoted in the Core README, `CLAUDE.md`, `ARCHITECTURE.md` and the public site as
"supervision matched quality at 2-3x the cost". A review on 12 November 2026 found that claim is wider
than the evidence.

What was measured: four pairs, one run per side per kind of work, the same GPT-5.6 Sol worker alone or
under an Exec and a lead.

| Kind of work | Alone: time, spend | Supervised: time, spend | Supervisor's own spend | Worker's spend, alone vs supervised |
| --- | --- | --- | --- | --- |
| Small coding fix | 112 s, $0.33 | 326 s, $1.01 | $0.48 | $0.33 vs $0.53 |
| "Large" coupled coding | 239 s, $0.68 | 423 s, $1.19 | $0.43 | $0.68 vs $0.76 |
| Independent account work | 175 s, $0.46 | 513 s, $1.52 | $0.60 | $0.46 vs $0.93 |
| Changing requirements | 229 s, $0.70 | 505 s, $1.79 | $0.53 | $0.70 vs $1.26 |

Problems with the wider claim:

1. **One run per condition.** No range or repeat is possible. The spec itself says the thresholds "are
   product decisions, not significance claims".
2. **No room to show value.** The solo worker finished every task in 2 to 4 minutes for under $1, and
   every result scored 97 to 99.8 out of 100 and passed every check. A supervisor can only add value by
   catching or preventing mistakes, and these tasks had none to catch. The result says "on tasks one
   worker already aces, supervision adds overhead", which is expected, not "supervision does not help".
3. **The supervisor was a ceremony.** The lead took 2 to 3 turns per run (commission, then accept). It
   never had to intervene, so this measured routing overhead, not supervision.
4. **The ratio is mostly a fixed cost.** The supervisor's own spend was about $0.43 to $0.60 per run
   regardless of task. On a $0.33 task that looks like 3x; on a $20 task it would be about 3%. Quoting a
   multiple hides this.
5. **The worker itself cost more under Restless.** Even excluding the supervisor, the same worker spent
   1.1x to 2.0x more inside Restless. Between 14% and 51% of the extra cost came from the worker, not
   the supervisor. That points at a runtime or briefing difference between the two sides, which
   confounds the comparison. It is also a product problem worth fixing on its own.
6. **Token counts are missing for every supervised run**, so supervised spend cannot be checked
   against tokens.
7. **Graders were the same model family as the worker** (two GPT-5.6 reviewers). There was no human
   grading.

**What can honestly be said:** "In four small tasks a single worker already did perfectly, adding an
Exec and a lead added about $0.50 and 3 to 5 minutes per task and did not change the result." That
supported the October decision to stop paying for routine supervision turns. It does not support any
general statement about whether supervision is worth it. The new programme tests that properly in
[Do AI reviewers actually check?](../../oi/01-do-reviewers-check.md) and in stage 1 of
[Searching for the best AI org chart](../../oi/05-org-chart-search.md).
