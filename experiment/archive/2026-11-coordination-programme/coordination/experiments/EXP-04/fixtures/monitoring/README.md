# Fictional competitive-monitoring corpus (_test only)

Search the 84 frozen documents. Each worker owns exact fictional entities and writes one JSON array to
`alerts/<actor>.json`. Return one locally complete alert per owned entity with `entity`, `event_code`,
`severity`, `source_ids`, `uncertainty`, and `follow_up_trigger`. Prefer authoritative late evidence;
do not count rumor, duplicate or superseded material as separate events. Run the declared verifier,
commit and report. There is no summary memo.
