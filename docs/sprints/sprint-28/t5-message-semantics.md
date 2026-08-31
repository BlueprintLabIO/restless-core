# S28-T5 — Keep messages expressive while declaring consequential meaning

**Layer:** OrgIntel messages + owner cockpit.

**Serves:** Sprint 28 criteria 9, 10 and 12.

**Depends on:** S28-T1 and S28-T2.

**Observed friction:** Natural conversation varies appropriately, but a consequential agent reply can
bury its outcome, next owner, artifact or request inside arbitrary prose. The renderer must either
leave it buried or guess from Markdown.

**Makes deletable:** Markdown inference for completion, next ownership and owner need; duplicated
status-summary prose where declared meaning already exists.

## Outcome

Ordinary conversation remains ordinary prose. When a message reports a consequential outcome or asks
for an owner act, the emitting actor may attach a small typed semantic receipt that the cockpit can
present without interpreting the message body.

## Scope

- Start from the existing message intent receipt and `details`/attachment seams.
- Add only fields proved by the corpus, expected to include some subset of outcome, next step, next
  accountable actor, owner need and artifact references.
- Keep the visible answer authored and append-only.
- Treat the semantic receipt as a projection aid, not as Work completion, approval, direction or
  authority.
- Render declared information compactly only when present; do not display empty status scaffolding on
  ordinary messages.
- Preserve supporting technical evidence under the existing optional detail disclosure.

## Acceptance

- A greeting, question and exploratory discussion need no receipt and render unchanged.
- A completion reply can identify the outcome, linked artifact and next ownership without repeating a
  “Status Summary” in prose.
- An owner-needed reply names the need but cannot resolve an Attention item or perform an authority
  action.
- A receipt that contradicts current Work/source state is rejected or visibly treated as an authored
  claim; it is never promoted to source truth.
- The UI contains no keyword classifier or Markdown parser that decides whether a message is
  completion, warning or request.

## Non-goals

- a universal message taxonomy;
- mandatory cards for every reply;
- rewriting message history; or
- replacing natural narrative with key/value status reports.

