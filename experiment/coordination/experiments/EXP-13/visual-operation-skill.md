---
name: visual-operation
description: Operate and review a prepared native game, website or desktop application from rendered pixels and ordinary input without reading implementation details.
---

# Visual operation

Treat the rendered application as the world. A process exit, filename or successful input command is
not evidence of what the application showed.

1. Identify the exact visible target by numeric window ID, title and geometry. Remove or minimise
   covering windows and preserve the target at its prepared size.
2. Capture and read a fresh initial image before acting. If pixels contradict the intended target,
   repair focus once or return `invalid`.
3. Choose one small action that discriminates between plausible states. Focus the exact target,
   perform the action and capture the resulting state before reasoning further.
4. Keep an ordered action and observation trace. Separate what pixels directly show from inference.
5. Explore naturally, including one ordinary recovery when the goal requests it. Do not replay a
   hidden script or inspect source, logs or tests.
6. Preserve decision-relevant before/after captures. Avoid repeated screenshots that add no fact.
7. Stop on visible success, a reproduced product blocker, an invalid observation path or the assigned
   budget. Do not keep trying merely to produce a pass.
8. Write a concise report containing target identity, completed checkpoints, exact reproduction,
   material findings, uncertainty and one verdict: `pass`, `revise`, `blocked` or `invalid`.

For transient feedback, perform the input and resulting capture in one native command when necessary.
For long-lived targets, prefer a durable background process and reattach rather than coupling the
application lifetime to a model tool call.

