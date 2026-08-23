# Coordination failure taxonomy

Use the smallest applicable set of codes in each run report. A code attributes the observed bottleneck;
it does not prescribe a fix.

## S — Specification and allocation

| Code | Meaning |
|---|---|
| **S1** | Outcome or acceptance target is ambiguous |
| **S2** | Work is assigned at the wrong organisational altitude |
| **S3** | Delegated branch is too coupled to the lead or another branch |
| **S4** | Actor/model/tool capability does not fit the responsibility |
| **S5** | Duplicate or contradictory responsibility exists |

## C — Context and common understanding

| Code | Meaning |
|---|---|
| **C1** | Necessary owner intent or current objective was lost |
| **C2** | Current artifact/runtime truth was absent or stale |
| **C3** | Too much irrelevant context displaced useful attention |
| **C4** | Private knowledge was required at integration but never transferred |
| **C5** | Shared context correlated errors or destroyed reviewer independence |

## M — Communication and alignment

| Code | Meaning |
|---|---|
| **M1** | Narration or status traffic consumed attention without changing state |
| **M2** | A message was mistaken for assignment, kickoff, completion or authority |
| **M3** | Interface or dependency change reached the receiver too late |
| **M4** | Consensus diluted better evidence or the strongest contributor |
| **M5** | Communication channel could not express or deliver the needed event |

## I — Integration and shared state

| Code | Meaning |
|---|---|
| **I1** | Multiple actors wrote conflicting canonical state |
| **I2** | Contributions were locally valid but failed together |
| **I3** | Integration required rediscovery or reimplementation of delegated work |
| **I4** | Artifact provenance or exact input version was lost |
| **I5** | Coordination status eclipsed observable outcome truth |

## V — Verification and termination

| Code | Meaning |
|---|---|
| **V1** | Agent narration substituted for native evidence |
| **V2** | Reviewer shared enough model/context/incentive to become an echo |
| **V3** | Missing callback or process end was misclassified |
| **V4** | Declared proof could not be reproduced in the evaluation environment |
| **V5** | Review occurred before the candidate or dependency existed |

## R — Runtime, provider and harness

| Code | Meaning |
|---|---|
| **R1** | Provider availability, quota or model failure stopped inference |
| **R2** | Runtime identity, cwd, workspace or tool capability was wrong |
| **R3** | Timeout/cancellation/result race corrupted the organisational outcome |
| **R4** | Harness telemetry omitted or distorted the material event |
| **R5** | Evaluator or fixture changed the candidate or manufactured evidence |

## E — Executive and organisational health

| Code | Meaning |
|---|---|
| **E1** | Exec remained occupied after dispatch and could not accept new work |
| **E2** | Department-level work escalated to Exec or owner unnecessarily |
| **E3** | Global mandate, safety, budget or ethics disappeared inside local work |
| **E4** | Repeated evidence failed to change routing, process or architecture |
| **E5** | Coordination cost exceeded the value of specialisation or parallelism |

The categories align at a high level with the MAST research taxonomy while retaining Restless-specific
runtime, evidence and executive-availability failures. Add a code only after two reports cannot be
described accurately with the existing set.
