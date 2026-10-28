# Reno reply comparison, 28 October 2026

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