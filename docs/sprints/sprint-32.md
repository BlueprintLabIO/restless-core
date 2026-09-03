# Sprint 32 — Make company voice human across channels

**Status:** Implemented and locally verified; matched live-model quality gate provider-blocked

**Date:** 31 August 2026

**Programme:** [Company Identity](./company-identity-programme.md)

**Depends on:** Sprint 31 accepted identity release and compiler.

## Why this sprint exists

AI-written company work converges on recognisable anti-patterns: abstract noun chains, false contrasts,
repeated triplets, prestige language, over-explained metaphors and copy that no named person would
comfortably sign. A single “brand voice” prompt also makes newsletters, support replies, UI and sales
email sound implausibly alike.

## Outcome

From one verified fact packet and one released company identity, Restless produces a founder email,
newsletter passage, support reply, transactional email, product UI message and standalone Blog passage.
Each is factually consistent and recognisably from the same company, while independent readers judge
the channel and named author credible rather than homogenised.

## Voice contract

Voice is evidence, boundaries and attributed judgement—not a bag of adjectives. It may include:

- approved passages with author, audience, channel and reason for inclusion;
- rejected passages and the concrete failure they demonstrate;
- vocabulary and product terminology bound to company truth;
- expression principles such as directness, warmth, formality and energy;
- named-author evidence that may vary within company bounds;
- channel dialects describing reader need and information order; and
- heuristic anti-pattern warnings that trigger judgement but never become a phrase blacklist.

## Success contract

1. The same facts remain stable across all six channels.
2. The founder, support, product and institutional voices are distinguishable without leaving the
   company identity.
3. Every public or customer-facing piece has a named or accountable author; anonymous synthetic
   omniscience is not the default.
4. A deliberately plain control candidate participates in every consequential copy selection. A more
   ambitious candidate cannot win by sounding more elaborate.
5. Writing begins from a human situation: person, attempted action, friction, changed outcome and proof.
6. Claims sit adjacent to evidence or are explicitly framed as belief, opinion or future intent.
7. An independent copy desk can reject fluent work for abstraction, unsupported claims, repetition,
   channel mismatch or implausible authorship.
8. Copy review operates on the rendered native artifact, not prose alone.
9. Owner edits may create attributed learning proposals but cannot silently change released voice.
10. No word-count target, readability scalar, banned-word gate, phrase-replacement bot or single
    canonical cadence defines success.

## Channel acceptance

- **Newsletter:** one real observation, one pointed conclusion, sufficient reasoning and a credible
  byline; not a long landing page.
- **Founder email:** personal and decisive, with no institutional throat-clearing.
- **Support:** acknowledges the actual problem, states what is known and gives the next action.
- **Transactional:** status, consequence and required action are visible immediately.
- **Product UI:** concise, stable terminology and an actionable recovery path.
- **Blog:** standalone context, deeper reasoning, raw observations where useful and no internal-file
  dependency.

## Slice per layer

**Authority / OrgIntel.** Promote Voice evidence through the Sprint 31 release path; retain author,
channel, positive/negative evidence, copy-desk verdicts and edit proposals without inventing a content
lifecycle.

**Runtime.** Compile the channel dialect, render native artifacts and bind review to exact candidate
identity. It does not rewrite prose to satisfy a phrase linter.

**Exec / accountable lead.** Select credible authorship and independent review in proportion to
consequence; no channel or Outcome Standard implies fixed team topology.

**Cockpit.** Review effective evidence and scoped learning proposals within Company Identity. It is not
an email composer, Blog CMS or automated brand approval queue.

## Salvage

No unverified salvage lift. Existing Blogs and company messages may enter T0 only after their authorship,
truth, channel context and owner judgement are revalidated.

## Ticket index

| Status | Ticket | Outcome |
| --- | --- | --- |
| [x] | [S32-T0](./sprint-32/t0-channel-corpus.md) | Freeze human, AI-pattern and channel-transfer examples |
| [x] | [S32-T1](./sprint-32/t1-voice-evidence.md) | Admit approved/rejected and named-author voice evidence |
| [x] | [S32-T2](./sprint-32/t2-channel-dialects.md) | Compile one identity into distinct channel contracts |
| [x] | [S32-T3](./sprint-32/t3-copy-desk.md) | Separate drafting from blinded human-copy evaluation |
| [x] | [S32-T4](./sprint-32/t4-native-rendering.md) | Review email, UI, Blog and newsletter in native form |
| [x] | [S32-T5](./sprint-32/t5-learning-proposals.md) | Turn edits into reviewable observations, never auto-policy |
| [x] | [S32-T6](./sprint-32/t6-dogfood-and-purge.md) | Prove human consistency without homogenisation |

Expected order: **T0 → T1/T2 → T3/T4 → T5 → T6**.

## Measures

Retain factual defects, unsupported claims, blind company recognition, channel fitness, named-author
signability, owner edits, concepts removed without meaning loss, context bytes and independent copy-desk
reversals. Do not report a human-sounding percentage.

## Terminal decision

- **Pass:** blinded reviewers recognise one company and six credible contexts for evidence-bearing
  reasons, with no factual drift and materially fewer owner rewrites than the baseline.
- **Revise once:** repair one bounded retrieval, channel or editor-contract defect.
- **Stop negative:** if consistency requires uniform cadence or a growing phrase blacklist, retain the
  corpus and do not promote Voice into the final constitution.
