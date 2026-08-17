# Skill trust, quality, and evaluation

## Finding

A public skill is third-party instruction and sometimes third-party executable code. Restless should
treat discovery as open, activation as deliberate, authority as unchanged, and continued use as
conditional on outcome evidence.

## The adoption loop

### 1. Start from an observed outcome gap

State the failed or expensive outcome first. "The landing page was not visually credible after two
revisions" is a candidate need. "We do not have a design skill installed" is not.

Name the smallest hypothesis:

> Giving the frontend actor this visual-direction and browser-comparison method will reduce owner
> revisions and produce a page the owner accepts.

### 2. Find candidates broadly

Search official publisher repositories, skills.sh, ordinary GitHub repositories, and applicable
domain sources. Registry position is only a way to create the candidate list.

### 3. Inspect before execution

Review:

- publisher and repository history;
- license;
- the full `SKILL.md`, including instructions that fetch remote content;
- scripts, dependencies, binaries, network calls, and write targets;
- references and assets;
- tool expectations and compatibility;
- hidden expansion, such as a script that downloads the real implementation; and
- overlap or contradiction with Restless doctrine and project instructions.

Instruction-only skills still create prompt-injection and misrouting risk. Script-bearing skills add
ordinary supply-chain and execution risk.

### 4. Pin the exact candidate

Record a Git commit or content digest before testing. A moving branch or remote guideline fetch makes
the attempt non-repeatable. Updates should be reviewed as diffs and tested as new candidates.

### 5. Probe in a test company

Never test an unproven skill against a live company. Run it in a `_test` company with the smallest
realistic permissions and real tools required for the evaluation. Keep external effects simulated or
pointed at explicit test targets.

### 6. Compare against a baseline

The baseline can be the same actor without the skill, the current company method, or a simpler
candidate. Hold the Work brief and success criteria stable. Record:

- accepted outcome or objective score;
- owner interventions and requested revisions;
- elapsed time and model/tool cost;
- tool failures and authority requests;
- artifacts used for review; and
- any new failure mode.

One good run supports limited use, not a universal expertise claim.

### 7. Use narrowly and observe

Load the skill only for matching Attempts. Preserve the pinned version in the Attempt inputs or its
referenced execution brief if repeated runs show this is valuable. Watch for provider changes,
dependency drift, stale instructions, and tasks that only superficially match the trigger.

### 8. Debrief against evidence

A 2021 [meta-analysis of after-action reviews](https://pubmed.ncbi.nlm.nih.gov/32852990/) combined 61
studies and reported a substantial average improvement across training criteria. The most consistent
benefits came when the review matched the individual or team and used objective review media.

The transferable lesson is not to create a mandatory meeting. It is to review the finished page,
paper, browser trace, receipt, or other concrete evidence, then change the next brief, skill, team, or
tool choice. A prose debrief that changes nothing is not organisational learning.

## A candidate skill card

This can initially be a Markdown block, not a database entity:

```text
Candidate: frontend-design
Need: Aris landing page lacked visual direction and failed owner review
Source: <repository URL>
Pin: <commit or digest>
Publisher claim: <short claim>
Contents: instructions | references | assets | scripts
Runtime needs: browser, screenshots, repo tools
Authority needed: none beyond the Work's existing grants
Test: same brief, with and without candidate
Pass: accepted live desktop/mobile outcome with fewer revisions
Disposition: candidate | accepted for <scope> | rejected | superseded
Evidence: <paths, URLs, Work/Attempt refs>
```

## What validation can and cannot prove

| Check | Can establish | Cannot establish |
|---|---|---|
| `skills-ref validate` | Format and metadata conformance | Correctness, safety, or usefulness |
| Publisher badge | Package provenance | Current integrity or effectiveness |
| Static security scan | Detected code patterns | Safe model behaviour or clean dependencies at runtime |
| Semantic scan | Suspicious instruction patterns | Absence of subtle manipulation or bad judgement |
| One test-company run | Plausible fit for that scenario | General reliability |
| Repeated comparable runs | Local performance distribution | Performance after unreviewed updates |
| Owner acceptance | Fit to owner judgement for that outcome | Objective correctness in every domain |

## Emerging security and quality research

Recent 2026 work is useful as a warning, not settled truth:

- [Agent Skills in the Wild](https://arxiv.org/abs/2601.10338) reports security weaknesses across a
  large public sample and finds script-bearing skills riskier than instruction-only packages. It is a
  preprint using an automated classifier, so the exact prevalence should not become a Restless
  invariant.
- [Agent Skill Security](https://arxiv.org/abs/2607.13987) proposes threats across discovery,
  retrieval, planning, execution, and updates. The lifecycle framing is more durable than its initial
  benchmark numbers.
- [What Keeps Agent Skills from Being Reusable?](https://arxiv.org/abs/2608.08453) reports widespread
  routing and packaging defects in public `SKILL.md` files. It was published only days before this
  review and has not earned replication. Its plausible, testable lesson is that correct packaging and
  routing should be linted, not assumed.
- [From Anatomy to Smells](https://arxiv.org/abs/2607.01456) similarly finds many authoring smells.
  A broad "smell" taxonomy can overcount harmless stylistic differences, so it is a source of review
  questions rather than a quality score.

The enduring conclusion does not depend on the exact numbers: packages that can alter model
behaviour or execute code need lifecycle review, version pinning, bounded permissions, and outcome
testing.

## Knowledge-base implications

Do not turn every useful online page into permanent actor context. Knowledge has at least three
different lifetimes:

- **company doctrine:** a small, reviewed canon such as `AGENTS.md` and architecture/spec files;
- **reusable method:** a pinned skill or process note, loaded when relevant; and
- **current task evidence:** source material and working files referenced by the Work or Attempt.

This is compatible with progressive disclosure and with `orgintel` §6.1, which already allows actor
profiles, process templates, experiments, and knowledge claims to begin as files with indexes and
references.

## Risk dispositions

| Risk | Disposition | Reason |
|---|---|---|
| Malicious or compromised skill executes code | **Guarded** | Inspect, pin, test in isolation, retain Kernel authority boundary |
| Low-quality skill wastes a test run | **Accepted** | Reversible and provides evidence if the test is bounded |
| Marketplace rank is mistaken for quality | **Guarded** | UI/docs should label it as popularity only |
| Skill becomes stale | **Pending fix after evidence** | Pin now; add update machinery only when maintenance recurs |
| Provider follows skill differently | **Accepted for probe, then measured** | Cross-provider behaviour cannot be inferred from format compliance |
| Skill metadata grants effect authority | **Invariant: never** | Authority remains a Kernel concern |

