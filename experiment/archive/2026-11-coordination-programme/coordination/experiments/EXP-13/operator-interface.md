# Thin visual operator scratch contract

Build the smallest ordinary Runtime tool that makes these four commands ergonomic. Reuse installed
Linux process, X11, capture and hashing tools. A shell script plus ordinary files is preferred over a
service or database unless live evidence proves it insufficient.

## `attach`

- Accept a session name, exact title matcher and optional already-started process/window hint.
- Select exactly one visible target or return an explicit ambiguous/not-found result.
- Record numeric window ID, title, geometry, process observation, display and generation.
- Raise, focus and calibrate the exact target.
- Keep target state independent of a model-turn process.
- Reattach by session name after an actor process restarts.

## `observe`

- Re-resolve and focus the recorded exact target.
- Query current geometry.
- Capture only that region into a monotonically named PNG.
- Record timestamp, target identity, geometry, capture hash and prior action receipt.
- Reject stale, missing or wrong-target state mechanically where observable.
- Print the capture and receipt paths for model-native image reading.

## `act`

- Accept one bounded keyboard, pointer or controller action batch.
- Capture a before state, focus the exact target, perform the batch and capture the resulting state in
  the same invocation.
- Action duration is explicit. Result observation should prefer visible change/stabilisation over a
  blind task sleep, with a bounded safety deadline returning `unknown` rather than `failed`.
- Record exact requested actions, exit state, before/after hashes and paths.
- Never decide which action is useful.

## `export`

- Produce one bounded manifest for the session containing target identity, ordered action receipts,
  selected captures and hashes.
- Accept a player-authored terminal report path and include it without rewriting it.
- Exclude secrets, raw unbounded video and unrelated files.
- Make the exported evidence immutable after successful creation.

## Non-goals

Do not interpret pixels, classify severity, create Work, inspect product source, manage budgets,
implement a browser DOM API, add a daemon database, or create a universal action language. Preserve
native tools as escape hatches.

