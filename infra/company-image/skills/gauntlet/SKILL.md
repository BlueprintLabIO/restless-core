---
name: gauntlet
description: Reach a hard quality bar by splitting an outcome into independently gradeable parts, building each, and passing it only through a blind review by a different actor. Use for consequential, creative or repeatedly failing outcomes where "good enough" keeps slipping; the Restless form of the public gauntlet-loop skill.
---

# Gauntlet

The public gauntlet-loop method (split, build, blind critic, repeat against a bar the builder cannot
argue around) expressed with Restless primitives. Nothing here simulates several agents inside one
session: every builder and every critic is an accountable actor with attributable Work.

## The bar comes first

Before anything is built, the accountable lead writes the bar into the Work itself:

- the exact outcome and the real audience;
- concrete pass criteria a stranger could check;
- one or more reference exemplars the critic compares side by side (a live product, a rendered
  page, a playable build), not "make it amazing";
- the native ReviewTarget the critic will operate.

If the bar cannot be written concretely, stop and resolve that with the owner or Exec. A vague bar
is the most common way a gauntlet degrades into taste arguments.

## Split

Split only where a part can be judged on its own and a stable ownership seam exists. One coherent
outcome stays one part. For each part the lead commissions builder Work:

```sh
restless work add --owner <domain>-<craft> --role <role> --title "<part>" \
  --outcome "<part outcome, audience, bar and references>" \
  --expected-artifact <native ReviewTarget> [--skill <relevant skill>]
```

## Blind critic

For each part, commission review Work owned by a *different* actor that did not see the draft. It
depends on and revises the builder's Work, so it receives the exact candidate version:

```sh
restless work add --owner <domain>-critic --role critic --title "Blind review: <part>" \
  --outcome "Judge the exact candidate against the bar and references side by side. Pass only if it beats the bar; otherwise request changes with the smallest decisive reasons." \
  --expected-artifact <review report path> \
  --requires <builder-work-id> --revises <builder-work-id>
```

The critic operates the real artifact (browser, player, rendered document), never only its source,
and never grades on the builder's own report.

## Repeat

A `changes_requested` review sends the part back into a new revision automatically. Before the next
review the lead reassigns the critic Work to a fresh critic that has not seen earlier drafts
(`restless work assign --work <critic-work-id> --owner <another>-critic --reason "blind retry"`), then
resumes it. The builder never grades its own work, and a critic that watched a previous draft never
grades the retry.

## Close

The lead integrates the passed parts and judges the whole outcome natively. Loop counts, a timer or
`/loop` are not completion; the bar and the review evidence are. Stop early and escalate if the bar
itself proves wrong: repairing the bar is a charter decision, not a critic's.
