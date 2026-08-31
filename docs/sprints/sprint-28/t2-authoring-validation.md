# S28-T2 — Make accountable authorship revisable and checkable

**Layer:** Actor context + source write paths.

**Serves:** Sprint 28 criteria 5, 6, 7, 8, 9 and 11.

**Depends on:** S28-T1.

**Observed friction:** Prompt guidance correctly assigns writing judgement to the accountable actor,
but prompt compliance is variable and objective structural defects are discovered only after the
content reaches the cockpit.

**Makes deletable:** Anonymous rewriting, duplicated surface-specific prompt lists and browser-time
discovery of missing actions, provenance or consequences.

## Outcome

One shared authoring discipline reaches every actor at the point it writes an owner-visible brief,
Work summary, important artifact label or consequential message. Objective contract failures are
rejected at that write boundary and returned to the same author for revision.

## Scope

- Consolidate the existing `present-to-owner`, `converse-with-owner` and
  `writing-what-the-owner-reads` guidance around the sprint's semantic discipline without turning it
  into a long formatting manual.
- Name which user-visible fields an actor is writing at each CLI/tool call.
- Require the author to distinguish observation, interpretation, recommendation, owner need and
  material uncertainty before submission.
- Validate required semantic roles, non-blank optional values, source/action identifiers,
  consequences, choice composition, fingerprints and supported transitions.
- Return a precise correction to the same accountable actor. Preserve authorship across revision.
- Keep the exact machine contract unchanged and separately addressable.
- Add behavioural prompt fixtures for varied domains; do not assert exact prose.

## Acceptance

- Context assembly tests prove the same concise discipline reaches Exec, leads and relevant Staff
  authoring paths.
- Write-path tests reject every objective defect named in scope and accept materially different valid
  prose forms.
- No validator uses word count, reading grade, keywords, regex/`contains`, sentence count or semantic
  similarity to decide quality.
- No BFF/browser model call or anonymous rewrite is added.
- Behavioural samples preserve all source facts while improving the reader's ability to identify the
  outcome or ask; final proof remains T6's blinded review.

## Non-goals

- deterministic proof that prose is good;
- forcing bullets or a fixed number of sentences; or
- requiring structured annotations on ordinary conversation.

