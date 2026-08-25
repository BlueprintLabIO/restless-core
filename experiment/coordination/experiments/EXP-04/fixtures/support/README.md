# Fictional support queue (_test only)

Own only the case IDs named in Work. Write `resolutions/<actor>.json`, one unit per case, with `id`,
`policy_version`, `action`, `customer_safe_draft`, `system_action_plan`, and `next_state`. Nothing is sent
or applied. During the run, the supervisor may provide a material policy update; any Attempt started
before it must reconcile preserved work and use the newer policy before handoff.
