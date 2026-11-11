# EXP-11 frictions

This file separates product failures, evaluator limits, and Restless harness defects. Mixing them
would make the experiment impossible to learn from.

## Product frictions

1. **The endpoint is not independently legible or reliable.** R19 reached the rendered route-end
   instruction, exited twice, repositioned in the destination structure, and attempted to unload
   without completion. The pixels establish the experience blocker; they do not establish its
   internal cause.
2. **Seat, parcel, and unload interactions compete for one key.** Several valid arms selected the
   driver seat when trying to recover or unload the parcel. The action model is learnable only after
   trial and error.
3. **The destination geometry does not communicate the valid placement volume.** Being visibly among
   the yellow posts is not sufficient evidence that the forward drop point is valid and off the truck.
4. **Rendered progress and allowed action have contradicted one another across revisions.** Earlier
   arms showed `40 / 40 m` before the host accepted route-end exit; the final revision improved this,
   yet the complete native journey still did not expose a successful unload.
5. **Mouse-look through synthetic native input is inconsistent.** Exact Sol often navigated with
   keyboard-only corrections. The final blocker did not depend on mouse look, but the weakness makes
   spatial play slower and less representative.

## Evaluator frictions

1. **Vision navigation is expensive.** R19's player used USD 21.1921 across both Work nodes; the
   36-minute full-journey Attempt alone cost USD 16.188112, mostly on first-person positioning and
   interaction recovery.
2. **Filenames are not evidence.** Several optimistic filenames described completion while their
   pixels still showed an incomplete journey. Direct image inspection correctly overruled them.
3. **Transient UI requires atomic capture.** OMP tool-turn latency exceeded the 3.5-second rejection
   banner. R18/R19 fixed the measurement by making input, a 0.50-second delay, and capture one native
   action.
4. **Freshness is more than a new prompt.** A valid independent run needs a fresh model-process
   capability, fresh native processes, exact window identity, and current-Attempt evidence links.
5. **Withheld context needs runtime enforcement.** R19's dependency edge automatically attached seven
   upstream artifact locators to the second Attempt. The player reported not reading them, but a
   future blind role must not receive them at all.

## Harness defects found and repaired

1. **Exact model selection was not exact.** A launch flag could differ from ACP session selection.
   Restless now selects the exact advertised session option and verifies the confirmation.
2. **Provider errors were inferred from ordinary model prose.** Words such as `unauthorised` or
   status-like numbers caused false cooldowns. Classification now requires a provider-error envelope.
3. **Parallel daemons shared ports and OMP profiles.** `RESTLESS_PORT_OFFSET` now namespaces broker,
   gateway, relay, coordinator, ingress, owner, review, Airwallex, and per-company OMP profiles while
   preserving default behavior.
4. **A superseded Attempt consumed bounded allowance.** Superseded snapshots are now excluded from
   claim and resume attempt counts.
5. **A concrete repaired blocker could not resume a one-Attempt Work.** An explicit `resume` now grants
   exactly one attributable successor when the previous bounded allowance is exhausted and records
   `attempt_limit_extended`.
6. **Cooldown caused repeated supervisor wakes.** Team-lead admission now checks the full model policy
   and stays quiet only when every exact candidate is cooling.
7. **Native-image routing was hard-coded to an unavailable delegated tool.** Exact vision models now
   ingest PNGs through OMP's ordinary `read` path; the optional delegated inspection tool is not a
   mandatory route.
8. **The wrong X11 surface could be captured.** The protocol now maps numeric IDs to titles, raises
   the exact CLIENT ID, queries its geometry, captures that region, and rejects pixels that do not
   identify CLIENT.

## Harness defects still open

1. **No aggregate experiment budget.** Each replacement company enforced its own ceiling, while total
   EXP-11 spend reached USD 170.887523. A parent experiment envelope is required.
2. **Dependency evidence leaks into blind Work.** A `requires` edge automatically adds upstream
   artifacts even when the downstream Work explicitly forbids them. Add an evidence-flow policy such
   as `ordering_only` versus `inputs`.
3. **Evidence output ownership is not seeded safely.** R19's read-only candidate was correct, but its
   output directory was root-owned. A referee volume should create a writable evidence root and a
   separately read-only candidate at startup.
4. **Long GUI launches are coupled to tool-call deadlines.** A background launch hit the 300-second
   command deadline and killed both processes. Use durable process handles with start, status, and
   stop events rather than a long foreground command.
5. **Terminal messages can wake Exec twice.** R19 produced two equivalent lead-to-Exec terminal wakes
   after the final verdict. Both stopped correctly, but the duplicate spent unnecessary budget.
6. **Unrelated inbound projection noise is global.** The isolated daemon logged a missing `aris`
   company warning every five seconds for hours. Reconciliation should be scoped to configured
   companies or exponentially quiet after a stable missing-config result.
7. **Capability expiry is a hidden work timeout.** Long ACP work can outlive a 45-minute capability,
   invalidating the next reasoning or terminal-decision call. Grants need transparent renewal or
   process-lifetime validity with revocation.
8. **Provider pause destroys useful native state.** Stopping a company to halt internal retry tears
   down the GUI and can turn a useful observation into blocked Work. Provider cooldown needs a
   cancellable pause that preserves actor and child-process state.
9. **Terminal artifact linking is overly manual.** R19 issued many individual links after gameplay.
   A bounded evidence-manifest primitive should atomically attach an immutable directory plus named
   critical files to the current Attempt.
10. **Token usage is not canonically additive.** ACP records include cumulative session snapshots and
    per-wake cost deltas. Spend is trustworthy; simple token sums are not. Publish canonical per-turn
    input, output, cache-read, cache-write, and reasoning usage.
11. **Live database tests race on the shared Authority schema.** The daemon suite passed 175 tests
    when run serially, but three tests concurrently attempting first Authority-schema creation failed
    on PostgreSQL's namespace uniqueness constraint. Test setup needs one migrated fixture or an
    idempotent advisory-lock boundary so the normal parallel suite is reliable.

## Process mistakes retained as evidence

- R0 claimed visual success without reading any screenshot.
- R1-R4 assumed a delegated image tool that OMP deliberately hides for native vision models.
- R7 captured HOST pixels from a CLIENT-titled overlapping surface.
- R12 proved that title identity alone can still contradict rendered pixels.
- R13 tested the wrong physical condition for shortcut rejection.
- R16 recognized its immediate evidence ambiguity and stopped.
- R17 separated action from capture long enough to miss the transient banner.
- R18 proved the right shortcut behavior but was invalidated by late feedback and attempt accounting.
- R19 retained a setup failure, used its single permitted repair, and then reached a conclusive product
  blocker without contaminating the player with source or logs.

These are not erased. They explain why the final protocol is strict and why future experiments should
reuse the protocol rather than rediscover it.
