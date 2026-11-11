# EXP-13 material frictions

These are harness, operator and controller findings unless explicitly labelled as product evidence.

## F1: the original B2 interface was undiscoverable

The public help did not disclose a usable `attach` invocation. The clean-room G1 player stopped instead
of guessing, with zero gameplay actions, captures or decisions. This invalid attempt was excluded from
the gameplay comparison.

## F2: the G1 controller selected the wrong kind of window

The first controller material named the overlapping DEBUG host while rendered evidence identified the
CLIENT as the player surface. Two narrow repairs made help usable and supplied a CLIENT-only matcher.
Those repairs enabled a valid G1 run but made the treatment dependent on game-specific controller
knowledge.

## F3: delivery feedback was semantically ambiguous

B0 saw an explicit outside-zone rejection. B1 remained at `DELIVERY IN PROGRESS`. Repaired B2 reset to
`CRATE free` and `OBJECTIVE: PICK UP THE CRATE` after a destination-zone drop. No arm displayed explicit
delivery acknowledgement. This is the one retained game/product evidence limitation. Follow-up stills
captured delayed transitions, so it did not justify temporal history.

## F4: Chromium needed runtime recovery

Fresh Chromium initially failed with `No usable sandbox!`. B1 passed after the documented
`--no-sandbox` compatibility launch and one fresh-profile recovery. B2 used the same compatibility
retry. These are runtime conditions, not website defects. The B1 mobile run observed a 508px outer
window rather than proving an exact 390px viewport.

## F5: cross-target identity was the decisive limit

W1 B2's only permitted title-neutral attach returned ambiguity across 10 and then 14 windows without
candidate titles or IDs. The player had no clean-room-safe route from “launch browser” to “attach that
browser.” This repeated the causal target-discovery seam behind F1 and F2. The frozen stop rule fired;
a third W1-specific repair was abandoned, and D1, B3 and Wave 3 were not run.

The structural lesson is that target identity cannot be left half in the controller, half in public
help and half for the model to infer. A future operator should receive an opaque, verified target handle
from the same launch/attach authority. That is still ordinary Runtime tooling and needs no new Work,
Actor or OrgIntel ontology.

## F6: experiment accounting was incomplete at arm level

The company-level Restless meter worked and ended at USD 15.644 of USD 120. Individual actors were
accounted, but the experimental reports did not receive canonical per-arm provider receipts or complete
wall times. Future native experiments should bind usage and elapsed-time receipts to the Attempt
automatically rather than asking players to infer them.

## F7: raw native evidence grows too quickly

One website arm alone produced roughly 175 MB because its evidence root also contained browser
profiles. Raw captures, profiles and traces are useful during adjudication but poor permanent knowledge.
EXP-13 therefore retains only terminal synthesis, metrics, checksums and frozen instructions; all raw
PNG and transient runtime artifacts were purged after synthesis.

## Other observed harness friction

- Long game wrappers killed targets at foreground command deadlines even though the task itself had not
  reached a semantic terminal state.
- Docker control-plane queries temporarily stalled while several independent companies were active;
  live ACP work continued. This was observational friction, not a reason to restart Docker or disturb
  other companies.
- The matched human-reference run never existed. Historical founder feedback is useful context but
  cannot support an 80 percent human-equivalence statement.
