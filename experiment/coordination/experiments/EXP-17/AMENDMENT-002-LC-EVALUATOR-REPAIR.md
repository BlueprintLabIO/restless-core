# Amendment 002: LC-2 evaluator repair and one symmetric replay

**Recorded:** 30 August 2026  
**Authority:** the frozen evaluation-infrastructure replay rule  
**Scope:** LC-2 only

The first LC-2 pair is evaluation-infrastructure-invalid. Its sealed test required the exact response
`version-mismatch` for a wrong completion version and `already-completed` for a repeated completion.
Neither response convention appears in the owner brief or starting public behaviour. Both candidates
used the equally valid `requirement-version-mismatch` response, and both preserved the semantic state
requirements. Requiring one undocumented string and one undocumented acceptance convention is an
implementation preference, contrary to the frozen hidden-case rule.

The visible fixture is unchanged at commitment
`c6902eed8afd8b1d4e30caa35bae2e238fab2ba95bb580ef5aa30fe8fbcae097`. The repaired sealed fixture:

- requires wrong-version completion to be rejected and leave receipts empty;
- permits either return convention for a repeated completion;
- requires the original terminal outcome and completed Work state to remain unchanged; and
- retains the supersession, stale-publication and restart cases unchanged.

Its new hidden SHA-256 is
`a6b67f6fdd66e7b8a0a9b846a4a139104d3b3fbce421e482dd1eb6ba58ecf41c`.

One symmetric replay is frozen before execution:

| Order | Opaque label | Arm | Ceiling | Safety envelope |
| ---: | --- | --- | ---: | ---: |
| 1 | `EMBER-42` | sealed | USD 12 | 2,400 seconds |
| 2 | `GLASS-17` | sealed | USD 12 | 2,400 seconds |

The exact Sol/high runner, runtime image, task bytes, visible gate, rubric and review protocol remain
unchanged. Reviewers receive no original candidate, score, process evidence or arm mapping. There is no
second replay. A further evaluator defect closes LC-2 as unknown.
