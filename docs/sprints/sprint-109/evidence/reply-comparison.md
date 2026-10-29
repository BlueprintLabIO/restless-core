# Plain owner replies: matched live comparison

Run: 30 October Sydney time, 19:31:46–19:33:21 UTC on 29 October.
This addresses D2's reply comparison. It does not complete A0's whole-task cost/quality requirement.

## What was compared

Seven frozen questions, each answered once with the pre-Sprint-109 reply contract and once with
the short contract, using the same facts, model and effort. The first three owner messages are
verbatim from the Reno-scale seed; four more cover the sprint's model-status, review, feedback
and email concerns. Pair order alternates before/after and after/before.

- Exact admitted route: `openai-codex/gpt-5.6-sol`, medium effort, pinned OMP 18.8.6.
- Before: `811519c6^:crates/restless-engine/prompts/converse-with-owner.md`.
- After: prompt at `2e71d2bd2f91a445aa78fa421cb89c46a3230d14`, SHA-256
  `0a5287b820600673f685f50e7063dd00150b40fe5246c40746465a4402a86f6c`.
- No tools, extensions, discovered skills/rules, title generation, saved sessions or model fallback.
- The facts and completed-action receipts are explicitly synthetic fixture inputs. This checks
  writing from supplied evidence; it does not prove that an agent executed those business actions.
- The output protocol matches the production requirement for one hidden intent marker, with
  an exact supplied UUID when linking the existing approval. Automated checks cover its shape
  and reference, not the semantic correctness of intent classification.

The broad short-contract rewrite was already in the deployed code. This change adds its matched
measurement and clarifies that the hidden marker must end the response; it does not introduce
the whole measured rewrite anew.

## Final paired results

| Observed measure, seven replies per variant | Before | After |
| --- | ---: | ---: |
| Input tokens | 12,307 | 6,329 |
| Output tokens, including reported reasoning | 2,010 | 1,430 |
| Total reported tokens | 14,317 | 7,759 |
| Cached-read / cached-write tokens | 0 / 0 | 0 / 0 |
| Visible whitespace-separated words | 268 | 256 |
| Total elapsed time, including CLI startup | 51.092 s | 43.797 s |
| Structurally valid hidden routing markers | 7 / 7 | 7 / 7 |
| Pinned-library API-equivalent cost estimate | US$0.089428 | US$0.053916 |

Each pair used 854 fewer input tokens with the short contract. Visible wording fell only modestly
overall and was longer in two cases. Output/reasoning and timing are single-run observations, not
stable performance estimates. The cost row is the library's tariff estimate for these completions,
not a subscription invoice, quota reduction or production saving. Admission/setup diagnostics are
not included in this final paired table.

Both variants reported no cache reuse in this run. The full Exec system prompt, long histories,
images, tool loops, retries and provider switching are outside this reply-only comparison. An 854-token
contract reduction is not a claim of halving total Exec context or company AI spend. Usage and cache
counts should be interpreted from the response, as described in the
[official OpenAI caching guidance](https://developers.openai.com/api/docs/guides/prompt-caching).

## Review against the frozen facts

The visible answers were reviewed by the coding agent (GPT-6), separately from the tested GPT-5.6
model, against the case rubrics written before the calls. This is a small model-assisted review,
not an independent human acceptance trial. No exact-answer matching was used to judge language.

| Case | Visible words before / after | Assessment |
| --- | ---: | --- |
| Empty Charter | 38 / 36 | Both confirm the completed save and link the page. The new answer unnecessarily says “correction”; neither hands the completed edit back to you. |
| Archive old sites | 35 / 27 | Both retain recoverability and the current sites. The old answer overstates that the Library keeps **only** those two entries; the fixture did not establish exclusivity. The new answer avoids that claim. |
| Raised budget | 44 / 35 | Both explain that existing Work is running on the temporary Sonnet fallback, distinguish the Opus preference and use the observed budget. No duplicate Work or redundant permission request is claimed. |
| Model status | 42 / 66 | Both distinguish configured preference from failed access and link settings. The new answer is longer. Both could name restoration of Reno's grant more precisely rather than generic reconnect/update wording. |
| Review preview | 40 / 34 | Both bring the existing card/preview and the observed photos/phone check. Neither claims approval. The new answer avoids the old closing approval-status line. |
| “Looks good” | 26 / 9 | Both preserve the explicit approval boundary. The new answer simply acknowledges feedback, without repeating the unchanged Accept/status explanation. |
| Email sent? | 43 / 49 | Both correctly say it was not sent, explain missing recipient approval, link the existing draft/card and avoid asking for another key. Both routing markers reference the supplied existing approval UUID. |

The short answers retained the decision-bearing facts in these seven cases. This is evidence for
the narrowed reply contract, with the imperfections above; it is not proof of broad model-quality
equivalence. The required no-send statement remains when the person actually asks about sending.

## Diagnostic failures and the marker clarification

The earlier five-question writing replay also contains a final short reply without a marker:
four of five after replies, versus five of five before replies, contain one. Its old runner checked
visible writing and did not reject missing routing metadata. Those historical records remain intact.

An initial fixture omitted a handoff UUID; another omitted the production's explicit final-marker
wording. Those setup runs are not the final comparison. Once the common protocol was made explicit,
the short contract still produced an archive answer without a routing marker. The failure is retained
in [the diagnostic log](owner-replies-seven-diagnostic.txt). The captured visible answer was ordinary
archive prose, with no hidden marker, rather than a parser rejecting a valid marker.

The short contract now says “End with exactly one intent marker in the existing format, as hidden
routing metadata.” Its visible answer remains unrestricted prose. The final seven after replies all
kept that marker and the existing approval reference. Seven successes do not prove a zero failure rate.
The runner records all protocol errors across a run and exits non-zero if any occur; it never silently
retries or drops a failed completion from the final report.

## Reproduction and remaining work

- [Frozen cases and quality rubrics](owner-replies-seven-cases.json)
- [Recorded replies, usage and hashes](owner-replies-seven-results.json)
- [Bounded runner](../../../../scripts/measure-owner-replies.mjs): `node scripts/measure-owner-replies.mjs --run`
  uses the already configured local broker, isolates `s109-ai-bench`, and writes private temporary
  output under `/tmp/s109_ai_bench_test`. It makes 14 live requests, sequentially, with a two-minute
  per-request bound. Stop on an unavailable exact route rather than silently choosing another model.

The runner claims a fresh disposable profile exclusively, removes it after its processes exit,
and has a termination/kill fallback after the session deadline. No provider token is printed or
copied into the repository. The measured run's profile was removed after completion; the
pre-existing broker remains owned by its original launcher.

A0 still needs matched productive outcomes, real provider cost/quality, larger histories/media,
no-change wakes and route-specific cache-retirement verification. Inspection found the generic
one-hour retirement window differs from metered Anthropic's five-minute request markers; native
subscription routes can use a different lifetime and must not be changed blindly. No cache-retirement
change or production cost reduction is claimed by this comparison.


## Earlier five-question writing replay

The earlier run and its original writing conclusions are retained below. Its runner did not validate
hidden routing markers; the new paired report adds that check. Reproduce that historical runner
from commit 811519c6; the single current runner above replaces it.

### Reno reply comparison, 28 October 2026

Real inference through the already connected local broker, `openai-codex/gpt-5.6-terra`.
Five identical Reno questions, identical frozen scenario facts, no tools or company writes.
These are replay scenarios derived from the triage, not claims about current production state.
`measure-owner-replies.mjs` records the complete style prompt, its hash, replies, timing and usage.

| Check | Original prompt | Final simplified prompt |
| --- | --- | --- |
| Visible words, five answers | 175 | 161 (8% fewer) |
| Core facts consistent with the supplied snapshot | 5/5 | 5/5 |
| Routine "No email was sent" statements | 3 | 1 |
| Structured Needs you callouts | 2 | 0 |
| Correct supplied map/draft link destinations | 2/2 | 2/2 |
| Unsupported claims of taking an action | 0 | 0 |

Accuracy rubric: distinguish sending support from an included Resend account; recognise the key
is already supplied and the binding mismatch is a product fault; state that the Brand approval is
absent from Inbox without inventing an approval; link the supplied map; describe the actual two
email changes and link the supplied draft. Inspect the raw visible replies against these facts.

The first shortened draft produced a placeholder Brand link and a file:// draft link. It is retained
as `owner-replies-after-first.json`. The final prompt explicitly preserves supplied destinations
and uses company-file Markdown links. The final run corrected both problems. The key-dispute answer
still says no email was sent; the routine repetition is reduced, not eliminated across every answer.

This small replay supports removal of the fixed reply scaffolding and preserves factual accuracy
on these five examples. It does not establish a general accuracy improvement, provider equivalence,
end-to-end tool behavior, or production quality. Those require the sprint's live smoke evidence.
