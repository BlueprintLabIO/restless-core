# Sprint 28 reader corpus

**Frozen:** 31 August 2026

This corpus tests whether a low-context reader can understand consequential Restless output without
knowing its internal vocabulary. The machine-readable fixture is
[`web/fixtures/sprint28-attention.json`](../../../web/fixtures/sprint28-attention.json). It uses the
real Attention wire shape and real source action semantics, but every item is controlled `_test`
content. It records no external effect.

## Reader questions

For each first view, ask the reader before opening evidence:

1. What changed?
2. Why does it matter?
3. What does the company recommend?
4. What, exactly, does the owner need to do?
5. What will each control cause?
6. What happens if the owner does nothing?
7. What is still uncertain?

A pass requires correct plain-language answers supported by the fixture. Product terminology such as
Work, handoff, source plane, ReviewTarget or revision is not required.

## Case A — bounded external approval

**Reader need:** Decide whether four already-reviewed emails may be sent once.

**Facts that must survive:** Four recipients and drafts were checked; nothing has been sent; approval
allows this exact campaign once; decline closes it; waiting sends nothing; demand is not confirmed.

**Appropriate form:** Two plain context facts, one recommendation and two mutually exclusive controls
with their immediate and next effects. This is not multi-select because the source authorises the
campaign as one bounded unit.

## Case B — native outcome review

**Reader need:** Inspect one prepared page, then accept it or request exact changes.

**Facts that must survive:** Browser checks passed; acceptance completes the Work but does not publish;
changes start another revision; conversation alone does not decide the review.

**Appropriate form:** One inspect control followed by a preview of the two real decisions. Evidence is
secondary to the native outcome.

## Case C — irreducible human last mile

**Reader need:** Understand why the company cannot complete a provider identity check and open the
exact prepared page in the owner's normal browser.

**Facts that must survive:** Restless cannot perform the identity check; opening the page does not
complete it; email remains restricted while waiting; unrelated Work can continue; the provider may
request another document.

**Appropriate form:** One explicit human-step control with its non-effect and expected next source
observation. Conversation remains available but cannot resolve the handoff.

## Case D — decision continuation

**Reader need:** Reconstruct what the owner chose, what that released, who owns the next step and what
has actually been observed since.

**Facts that must survive:** Approval covered four reviewed invitations; delivery tracking is now
owned by the validation lead; two deliveries are observed; two results remain pending. Internal
attempt or revision numbers are not required for this account.

**Appropriate form:** A causal sequence in Decision history. It is not copied status prose and it
does not upgrade pending provider results into success.

## Case E — completed Work and important output

**Reader need:** Understand what was delivered and whether it is ready to inspect without first
reading implementation instructions.

**Facts that must survive:** The page is complete; desktop, narrow-screen and keyboard checks passed;
it remains private; publishing is a separate owner decision; the native page is available to open.

**Appropriate form:** The observed result first, followed by recognisable linked output and observed
availability. The exact execution contract and run/accountability records remain unchanged inside an
explicit technical disclosure.

## Case F — ordinary conversation

**Reader need:** Have a natural exchange without being forced through a status template.

**Facts that must survive:** The owner wants to discuss the launch later; nothing changed and nobody
needs to act now.

**Appropriate form:** The authored reply only. The internal intent summary may support routing, but no
empty Outcome, Next or Needs-you block is rendered.

## Case G — consequential agent reply

**Reader need:** See the direct answer, observed outcome, next owner and exact request without finding
them inside a process narrative.

**Facts that must survive:** The page passed browser checks and remains private; the owner needs to
review it; Mara owns any requested revision; the message itself does not accept or publish anything.

**Appropriate form:** A short natural answer plus the optional semantic receipt. The receipt is an
authored navigation aid, not source authority.

## Negative cases

- Do not manufacture checkboxes from the four campaign recipients; the source has no per-recipient
  operation.
- Do not show internal Work titles instead of the authored Attention headline.
- Do not hide a control's consequence in hover text.
- Do not rewrite or shorten the evidence record.
- Do not make an ordinary message render an empty Outcome/Next/Needs-you block.
