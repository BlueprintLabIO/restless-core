---
title: "Keep the work graph sparse"
deck: "A useful graph records real responsibility and returned evidence. It does not need to mirror every thought, tool call or local step."
publishedAt: 2026-08-26
order: 4
readTime: "4 min"
run: "EXP-01 to EXP-05"
finding: "Coordination state earns permanence when another actor accepted responsibility or returned evidence that changes a decision."
status: "Provisional"
---

Early coordination systems often turn the plan into a graph. Every task becomes a node. Every node
gets states, dependencies, retries and an owner. The diagram looks rigorous. The company spends its
time maintaining a second version of the work.

Our experiments pushed in the other direction.

## Files carry the work

Code, research, drafts, browser state and tests already live in the company computer. Git records
meaningful checkpoints. Provider records establish outside facts. Rebuilding those materials as graph
entities adds custody without adding capability.

The graph becomes useful at the point where accountability crosses minds. A node should answer a
small set of factual questions:

- who accepted this outcome;
- which contribution another actor owns;
- what evidence came back;
- who judges the combined result;
- which material fact changed the responsibility.

Local steps can stay in the actor's working files and model session. They do not need a durable
organisational ceremony.

## Large work does not require a large graph

A coherent product outcome may be large and still belong to one lead with one end-to-end worker. A
small sales account may deserve its own Staff owner because it closes independently and can be
accepted without a batch rewrite.

The boundary follows accountability closure. Apparent task size is a weak signal.

This is also why a list of subtasks should not automatically become a team. Another mind brings
specialisation, parallel capacity or independent evidence. It also charges briefing, context loss,
communication and integration. If the contribution cannot be judged on its own, the graph records a
handoff without buying much accountability.

## Events should change responsibility, not decorate it

Ordinary progress messages create a busy graph and little information. Material events are different.
A policy change, exact gate failure, harmful condition or contradiction can change what should happen
next. Those facts belong on the responsibility line because another actor may need to redirect,
repair or stop the work.

This produces a small operating substrate: Actor, Work, Attempt, addressed messages, artifacts and
process callbacks. No tested workload has earned a blackboard, shared conversational history,
universal queue engine or durable workflow system.

## The deletion test

When coordination machinery grows, inspect a real run and ask which path improved the outcome. Delete
the rest.

This is a lower bar than defending each abstraction in prose. Git keeps the deleted implementation.
The live company pays for every surviving concept on every future request.

The sparse graph remains a provisional answer. A repeated recovery failure may show that one more fact
deserves durable form. Until then, files and intelligent communication should carry the detail.
