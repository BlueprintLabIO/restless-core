# S08-T9 · Make People an honest contact and inspection surface

**Layer:** Owner surface + OrgIntel projection.  
**Serves:** Sprint 08 criterion 16.  
**Depends on:** S08-T8.  
**Observed friction:** Sprint 06's live surface groups `exec`, `world` and `daemon` beneath `Not
people`; uses `['exec', 'world', 'daemon']` in Svelte to infer that grouping; gives every team member
equal button/chat weight; opens a transcript before explaining that only the Exec and leads are
contacts; repeats role subtitles and `READY` on nearly every row; and renders a long Work instruction
as compact current focus.  
**Makes deletable:** `isStandingActor`, the `Not people` section, actor-id personhood/contact lists,
equal-weight member buttons, refusal-at-the-bottom conversation presentation, universal idle `READY`,
duplicate role subtitles and `focusWork?.outcome` as compact roster copy.

## Outcome

People answers two owner questions without pretending they are the same interaction:

1. Who can speak accountably for this company or team?
2. Who is doing the work, and what are they doing?

The Exec is the distinct company-wide contact at the top. Each team lead is its accountable contact.
Members remain visible and selectable for inspection under that lead, but do not wear conversation
affordances they cannot fulfil. Message-provenance principals remain queryable records and never
appear as colleagues.

## Actor semantics

- `world` and `daemon` remain actor rows because message foreign keys and transcript provenance need
  stable senders. Creation/repair classifies both with `kind=system`; their ids remain unchanged.
- The People projection excludes `kind=system`. It does not name current or future pseudo-actor ids.
- `exec` remains `kind=exec`, is contactable and has a singleton treatment before team groups. It is
  not a peer row under a negated section and is not counted as an ordinary team member.
- Staff contactability is derived from actual team leadership. No role string or id suffix confers it.
- Unassigned Staff remain visible and inspection-first; their accountable route is the Exec.

This ticket revises Sprint 06's `Not people` presentation and the owner-cockpit claim that every
employee is a direct chat target. The owner retains direct access to company accountability through
the Exec and leads, and retains inspection of every Staff member and their Work.

## Surface behaviour

1. **Exec first.** Give the singleton Exec a visually distinct top treatment with company-wide
   conversation access. Do not place it inside a team or a generic remainder group.
2. **Contacts look contactable.** Exec and lead rows retain full weight, button chrome and conversation
   selection. Team headers may select their lead.
3. **Members look inspectable.** Member rows are smaller, denser and visually subordinate. They remain
   keyboard/mouse selectable to inspect current Work and recent evidence, but have no hover or chrome
   suggesting that selection opens chat.
4. **Inspection is useful immediately.** Selecting a non-contact leads with their current Work,
   meaningful title/focus, status and available output. Promote `Talk to <lead> for <team>` as the
   primary route near the top; do not first render an empty conversation transcript and put the
   refusal in its footer.
5. **Remove duplicate labels.** The primary row is the human-readable display identity. Role and lead
   relation are available in tooltip/profile/detail rather than permanent subtitles.
6. **Show only exceptional availability.** `working` and `cooling down` are visible. Idle readiness is
   the absence of an exception, so rows do not repeat `READY`.
7. **Use compact Work meaning.** The roster/focus card uses the Work title or a bounded source-owned
   current-focus summary. Full `outcome`/instructions remain in Work detail and cannot spill into the
   compact card as “is working on”.
8. **Preserve honest degraded state.** If OrgIntel/team data is unavailable, do not infer contacts or
   hierarchy. Preserve the last observed read only where the existing cockpit contract permits it
   and label unavailability.

## Contract correction

Update the owner-cockpit People/chat sections and current decision that currently promise direct chat
with every employee. The new default is:

```text
Exec and accountable leads: direct conversation
Staff members: Work/evidence inspection + route to accountable lead
system principals: provenance only, absent from People
```

This is an owner-attention and organisational-accountability decision, not a permission boundary.
OrgIntel messages may still exist between actors; the cockpit chooses the accountable human-facing
route.

## Verification

- Seed a `_test` company with `owner`, `exec`, `world`, `daemon`, two teams, their leads/members and
  one unassigned Staff actor. Ensure inbound/daemon messages still retain their exact sender ids.
- API/projection: `world` and `daemon` are `kind=system` and absent from People; no client id list is
  required to achieve it. Exec, Staff kind, team membership and lead relation remain source-owned.
- Headless UI: assert the order is Exec, team groups with lead before members, then unassigned Staff;
  there is no `Not people`, role subtitle or idle `READY` text.
- Interaction: selecting Exec or a lead opens a composer. Selecting a member opens inspection-first
  detail, exposes their current Work and presents a working route to the exact accountable lead.
  Selecting unassigned Staff routes to Exec.
- Focus: seed a short title and a long instructional outcome. The compact card renders the title/
  bounded focus and the Work detail retains the full outcome.
- Browser review at the actual People width confirms full primary display names remain legible, member
  rows are materially denser/lighter than contacts, contact and inspection affordances are visually
  distinct, and keyboard focus remains accessible.

## Risks

- **Filtering system actors destroys provenance — invariant:** rows and messages remain; only the
  People projection omits `kind=system`.
- **A fourth pseudo-actor leaks into People — invariant:** filtering uses actor kind, never an id list.
- **The owner needs direct worker conversation in a real case — accepted:** route through the lead or
  Exec first. Record repeated owner friction before restoring peer-level chat affordances.
- **Member inspection becomes a hidden dead end — guarded:** Work/evidence appears first and the
  accountable contact action is promoted, not buried.
- **Human-readable display hides exact technical identity — guarded:** profiles, URLs and provenance
  retain actor id; only the compact roster prioritises display.

