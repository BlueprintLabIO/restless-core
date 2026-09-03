# S38-T0 — Freeze the appliance corpus

**Serves:** Daily-use reliability must be judged against real machine and owner journeys rather than
daemon uptime or a collection of unit tests.

## Work

- Freeze macOS version, user-service identity, stable and dev roots, ports/sockets, startup/recovery
  bounds and uninstall ownership.
- Freeze schedule cases for ordinary due time, duplicated wake, daemon death, sleep/resume, overdue
  backlog, cancellation, DST and forward/backwards clock movement.
- Freeze three native owner journeys: embedded web, Swift Arrival native launch and Company Computer
  fallback, including denial and cleanup cases.
- Record baseline friction: steps from login to usable Restless, manual daemon interventions, missed
  schedules, artifact launch steps and current residue.

## Acceptance

The corpus has exact inputs, expected Authority/OrgIntel records, user-visible result, process/resource
state and cleanup probe for every counted case. Test fixtures use only `_test` companies and cannot
resolve stable resources.

## Makes deletable

Ad hoc launch commands, verbal schedule expectations and uptime-only acceptance.
