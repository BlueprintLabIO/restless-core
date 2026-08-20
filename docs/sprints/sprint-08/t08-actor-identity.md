# S08-T8 · Give every Staff actor one durable organisational identity

**Layer:** OrgIntel + owner projection. Actor identity remains OrgIntel-owned.  
**Serves:** Sprint 08 criterion 15.  
**Depends on:** nothing. T9 depends on this identity contract.  
**Observed friction:** current live rows mix `site-validation-lead`, `centre-critic-live`,
`copy-critic`, `prospect-research-live`, `release-impl` and `staff-centre-critic`. Actor ids encode
kind, mutable team position, one environment or one attempt even though actor kind, team leadership,
Work and revision already own those facts. The People surface then renders the slug as the person's
name and repeats its craft as a subtitle.  
**Makes deletable:** `staff-`, `-lead`, `-live`, revision/stage/retry actor naming, id-as-display
rendering, assignment-shaped actor creation and any fuzzy post-hoc merge intended to repair them.

## Outcome

Every newly created Staff actor has one stable machine identity for a durable business domain and
craft, one separate human-readable colleague identity for the owner surface, and one separate role.
Changing team, becoming lead, receiving new Work, revising an Attempt or moving from test to live
does not change the actor id.

## Identity contract

For new Staff:

```text
actor_id = {domain}-{craft}
display  = stable human-readable colleague name
role     = organisational craft/responsibility
team     = current relation, stored elsewhere
```

Examples:

```text
centre-critic
copy-critic
release-build
prospect-research
```

The Staff id has exactly two non-empty kebab-case segments. It contains no `staff` prefix, mutable
team position, environment, Work title, version, stage, retry, model or implementation mechanism.
Reserved singleton/system ids such as `exec`, `owner`, `world` and `daemon` are outside this Staff
grammar.

`display` is the primary People label and is chosen once as a colleague identity; it is not the slug
reformatted with spaces and not another copy of `role`. The actor id remains visible in technical
details, links and provenance where exact identity matters. Role belongs in the profile or a tooltip,
not as a permanent second roster line.

## Scope

1. Enforce the Staff grammar in the OrgIntel creation boundary used by every CLI/API caller, not only
   in Svelte or `--help` text.
2. Reject a non-conforming id before any actor, team membership or Work is written. A third segment
   such as `centre-critic-live` is invalid rather than an available collision escape.
3. Exact domain/craft uniqueness remains the database identity constraint. On collision, return the
   existing actor and require the caller to reuse it or choose a genuinely different domain/craft
   with the already-required reason. Do not implement fuzzy automatic merging.
4. Keep `display`, `role`, actor kind and team relation separate through the cockpit projection.
   Lead moves and role evolution must not rewrite the actor id.
5. Do not automatically rename or merge historical actor ids. They carry Work and message
   attribution. Report non-conforming active rows and allow explicit, attributable display repair or
   retirement; preserve their original ids as history.
6. Choose the compact People column around display names. Exact ids may truncate visually only where
   a tooltip/detail reveals the full value; the primary colleague label must remain legible.

## Verification

- Create `centre-critic`, assign it Work, make it a lead, move it to another team, run a revision and
  return it to member status. The same `actor_id` owns every Work/Attempt and message.
- Reject `staff-centre-critic`, `site-validation-lead`, `centre-critic-live`, `copy-critic-v2`,
  `release-build-retry` and a duplicate `centre-critic` before any partial write.
- Create two genuinely different identities with explicit domain/craft differences and confirm the
  reason is recorded without inventing a near-collision suffix.
- Project actor id, display, role, kind and team separately. The roster renders display as primary;
  exact provenance and technical detail still expose the id.
- Produce a live-company report of legacy non-conforming ids. The verification must show no automatic
  rename, merge or deletion.

## Risks

- **Two genuinely different specialists want the same domain/craft — accepted:** force the
  organisational distinction to be named in domain or craft rather than hidden in `2` or `live`.
  Revisit only after real cases repeatedly cannot name the difference.
- **A human-readable display identity implies a human legal person — guarded:** the profile continues
  to state that this is an agent actor and exposes role/model on request; the roster optimises for a
  legible organisation, not anthropomorphic concealment.
- **Legacy ids remain inconsistent — accepted:** stable provenance outranks cosmetic uniformity.
  Repair display or retire explicitly; never rewrite history automatically.
- **Identifier policy becomes a product-wide naming bureaucracy — guarded:** the rule applies only to
  durable Staff ids at creation. Work, teams, files and providers keep their own native names.

