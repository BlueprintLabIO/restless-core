# S06-T3 · Durable specialist actors

**Layer:** OrgIntel. Actor identity is OrgIntel's concept (`cross-layer-contract` §3.1).
**Serves:** `orgintel` §5.1 — *durable actors, replaceable sessions* — which the live companies
currently contradict.
**Depends on:** nothing. Lands before S06-T4 because grouping throwaway actors under a lead organises
throwaways.
**Makes deletable:** the per-Work/per-revision actor-minting paths, once the same runs complete with
reused actors.

**Implementation evidence:** tracked in the Sprint 06 checklist and
[`run-report.md`](run-report.md), not in a second ticket-status field.

---

## The friction

Live Aris shows thirteen actors with three families of the same defect — a numeric suffix
(`copy-critic2`), a variant suffix (`centre-critic-live`), and a revision suffix
(`staff-email-writer-v9`). The `aris_feedback2_test` company shows the same thing at full scale.
`restless people --company aris_feedback2_test` returns **26 actors**, of which 24 are variants of
three specialists:

```
email writer  (10)  staff-email-writer · -stage3 · -v4 · -stage4-fix · -v5 · -v6
                    · -v6-corrections · -v7 · -v8 · -v8-retry · -v9
english critic (7)  staff-english-critic · -v2 · staff-plain-english-critic-v3 · -v4
                    · -v5 · -v6-check · -v7
commercial     (7)  staff-commercial-critic · -v2 · -v3 · -v6-check
                    · staff-commercial-evidence-critic-v1 · -v2
```

Three people. Twenty-six organisational identities. Every one of them is a new durable actor created
to carry one attempt at one piece of work — and `-v8-retry` and `-stage4-fix` show it happening for a
*retry* and a *fix*, not even for new work.

`orgintel` §5.1 says an actor is a durable organisational identity and the session is the replaceable
part. In practice the actor is being replaced along with the session. Consequences already visible:

- **Evidence does not accumulate.** The People surface shows `copy-critic` with 1 landed and 1
  commitment. The critic that actually reviewed most of Aris's copy is spread across five rows.
- **"Who did what" is unanswerable**, which is the exact question `orgintel` §6.3.1 added role and
  model heterogeneity in order to make answerable.
- **Any grouping is meaningless.** Teams over these rows would be teams of ghosts.

The cause is mechanical, not a judgement failure. `work-add` calls
`add_actor_with_model(owner, role, owner, model)` with the actor id supplied by the caller, and the
caller composes a fresh id per Work. Nothing in the path offers "reuse the specialist you already
have", so composing a new name is the path of least resistance.

## Scope

1. **An actor is addressed by durable identity, not by assignment.** Assigning Work to an existing
   actor must be at least as easy as creating one — the write path takes an existing actor id and
   reuses its role, model and accumulated record.
2. **Discovery before creation.** The staffing surface used by the Exec and an appointed team lead
   answers "which durable specialists exist, with what role and model, and what have they done"
   before it answers "create one". Creating a near-duplicate should require having seen the existing
   actors. A lead assembling a team reuses a suitable actor where possible; if no actor buys the
   needed difference, it creates one stable specialist identity, never an identity for one revision.
3. **Revisions never mint actors.** A `revises` edge increments the producer's Work revision and
   reassigns the *same* actor. There is no legitimate reason for `-v9`.
4. **Retirement is explicit.** An actor no longer in use is marked retired rather than abandoned in
   place, so the People surface can be honest about a twelve-row list that is really five people and
   seven ghosts. Retirement never deletes the record.
5. **A migration for the live companies** that neither invents history nor loses it: existing actors
   stay exactly as recorded, and the report says which look like variants of one identity. Merging is
   an owner/Exec judgement, not an automatic string-similarity rule.

**Not in scope:** actor "profiles", skills, capability catalogues, or performance scores. Role, model
and the Work record are what exist; this ticket makes them stick to one identity.

## Verification

A `_test` company runs one Work through producer → critic → `changes_requested` → producer revision 2
→ critic re-review. Expected: exactly two staff actors exist at the end, the producer's Work is at
revision 2 under the same `owner_id`, and `list_actors()` grew by two, not four.
Against live Aris: the report lists the suspected variant families above, and no run creates a new
suffixed actor.

## Risk disposition

- **Reused actor carries stale context into a new assignment** — *guarded*. §6.3.1 makes narrow,
  deliberately different context the feature; a reused specialist gets a fresh brief, not a fresh
  identity. If context bleed is observed, it is a context-assembly bug, not an argument for minting.
- **Retirement hides an actor the owner wanted to see** — *accepted*. Retired is a filter, not a
  delete; the row is one control away.
- **The suspected-variant report merges two genuinely different critics** — *guarded*: it reports,
  it never merges.

## Landed evidence

Migration `0008_durable_actors.sql` adds explicit retirement without deleting history. The ordinary
CLI now creates, lists, and retires stable actors with a reason and attribution; retired actors are
excluded by default and cannot be silently resurrected. `work add` requires an existing active
actor and validates its recorded role/model instead of upserting an arbitrary caller-composed id.

The configured-Postgres scenarios prove actor reuse across Work revisions. In live Aris, the lead
reused `staff-centre-critic` and `staff-prospect-research-live`; the Exec retired six dormant legacy
variants with explicit reasons and no history loss, and no `-v2`/retry actor was created by the run.
