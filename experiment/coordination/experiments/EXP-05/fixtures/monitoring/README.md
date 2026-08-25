# EXP-05 fictional monitoring corpus (_test only)

The corpus contains 280 dated documents across 40 fictional entities. Staff owns disjoint entities
and returns one locally complete alert per entity with `entity`, `event_code`, `severity`, `source_ids`,
`uncertainty`, and `follow_up_trigger`. Prefer authoritative late evidence. Rumor, stale contradiction
and noise are evidence to reject, not separate alerts. The product is an alert feed/index; there is no
summary memo or cognitive fan-in.
