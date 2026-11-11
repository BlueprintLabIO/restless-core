# EXP-13 terminal results

## Disposition

**`model-or-policy-limited`**, scoped to the tested thin visual-operator and clean-room controller
contract. This is not a verdict against Swift Arrival, the website, computer use generally, or
multimodal models generally.

The repaired four-command operator materially improved the game journey, but the gain did not
generalise to the website. The website player could not reach pixels because `attach` could report an
ambiguous target without exposing a safe way to identify the intended window. This repeated the target
discovery seam already repaired twice for the game and triggered the frozen repeated-harness-failure
stop. A proposed third target-specific repair was abandoned rather than repairing the benchmark until
it passed.

## Findings

### G1: Swift Arrival

| Arm | Validity and observed result |
| --- | --- |
| B0 current loop | Valid. Reached pickup, deliberate error and recovery, driving, route 40/40 and seat exit. An outside-zone unload was visibly rejected; no completion appeared. |
| B1 playbook | Valid. Reached the destination, but four delivery attempts were rejected or left the stable state at `DELIVERY IN PROGRESS`; no acknowledgement appeared. |
| Original B2 | Invalid harness evidence. Public `attach` usage was undiscoverable, so it observed no target pixels and performed no gameplay actions. |
| Repaired B2 | Valid. Reached the destination and a post-drop reset using 17 action batches and 17 decisions, with no target loss, wrong-window event, focus repair, tool failure or visibly rejected interaction. No explicit delivery acknowledgement appeared. |

The blind G1 adjudicator found that valid B2 materially beat B1 on observable progress,
recovery/manual-rescue burden and invalid-interaction burden. It reduced decisions from 34 to 17 and
recorded zero target, focus and rejected-interaction failures versus B1's target loss, wrong window,
two focus repairs and six invalid interactions. The result does not establish completed delivery,
accepted feedback quality or per-arm cost efficiency.

### W1: frozen Restless website

W1 B1 passed after one browser-process recovery. It completed the rendered desktop journey, opened
Research without URL entry, opened the named article, returned to Product and found the stated
boundary. A fresh 508px-wide rendered run also reached Research. This is not proof of an exact 390px
viewport.

W1 B2 was blocked before rendered interaction. A title-neutral attach matched 10 and then 14 visible
windows, but returned neither candidate titles nor identifiers. The clean-room player correctly did
not guess or use a prohibited second discovery channel. It produced no page capture, URL observation,
action receipt or export. This is operator/controller evidence, not a website defect.

## Stops and gaps

- D1, B3 and Wave 3 did not run after the predeclared stop fired.
- B3 was not activated because follow-up stills captured the delayed game states; no missing temporal
  fact was demonstrated.
- No tester-to-repair-to-fresh-regression product loop ran.
- No matched human trace exists, so every human-reference dimension and the provisional 80 percent
  profile remain unsupported.
- Per-arm provider receipts are absent. The Restless company accounted for **USD 15.644** against the
  USD 120 ceiling; the separately observed provider-reported admission request cost USD 0.029638.
- No G1 arm showed an explicit visible delivery acknowledgement.

## Decision

Purge this experimental operator and do not repair, rerun or promote it inside EXP-13. The demonstrated
game-control benefit justifies a new design, not promotion of this one.

The smallest next design should make target identity part of the launch/attachment contract: launch a
surface and return an opaque handle, or have the controller supply a uniquely verified target handle.
It must not depend on model guesswork, hidden target-specific titles, or a separate forbidden discovery
tool. Test that contract unchanged across at least a game and browser before reopening spreadsheet or
repair-loop claims, and collect per-arm usage receipts from the start.

## Evidence retention

The terminal synthesis, [friction register](FRICTIONS.md), [machine-readable metrics](metrics.json),
frozen protocol, Work IDs and checksums are the durable record. Raw PNGs, browser profiles, native
process state, action traces and scratch operator code were deliberately kept outside the repository
and purged after terminal synthesis. No screenshot is required to support the retained decision.
