# S11-T5 · Project one chronological executive conversation

**Layer:** Owner gateway projects active ACP events; owner cockpit renders them beside OrgIntel's
durable messages.

**Observed friction:** The owner saw a generic waiting block while text arrived only at completion,
and tool calls were grouped separately from prose. A long turn therefore looked stuck and its real
sequence was hard to follow. History anchoring and reconnect behaviour were implicit.

## Outcome

After send, the owner's message anchors at the top of the transcript region and the active response
grows beneath it. Safe assistant updates and tool events interleave chronologically, with restrained
working motion and elapsed time. Completion collapses to a compact expandable record backed by the
same durable conversation.

## Acceptance

- A text → tool → text → tool fixture renders in that order during streaming and after reconnect.
- Unicode offsets, truncation and multiple tools at one text boundary preserve order.
- Reconnect never duplicates text or tool rows; durable messages remain the sole history source.
- Draft preservation, older-history loading and post-send scroll anchoring work at 390, 768 and 1440
  CSS pixels.
- Working, delegated waiting, failed and complete states are distinct without permanent subtitles or
  decorative eyebrow labels.
- No hidden reasoning, system prompt, credential, raw secret or unbounded tool output is rendered.

## Deletion

Makes the generic composer instruction row, separate tool-first trace, static waiting placeholder and
any second client-side transcript store deletable.
