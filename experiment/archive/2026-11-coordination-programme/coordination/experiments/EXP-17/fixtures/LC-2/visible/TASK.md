# Outcome: make unattended inbound work safe to enable

An operator needs this isolated service to admit simulated business signals, create durable work and
publish exactly one current outcome receipt per case. Own the coupled path end to end.

Required behaviour:

- a repeated `signal_id` has no second semantic effect, including after restart;
- a higher requirement version supersedes unfinished older work for the same case;
- stale work can never publish a terminal outcome after a material requirement change;
- completing the same work twice is idempotent;
- one case exposes one keyed current receipt, updated only by work for the current requirement;
- persistence is atomic and survives constructing a new service instance.

Preserve the public API. Run the visible tests, add proportionate regression tests and write `RESULT.md`
with the diagnosis, invariants and exact commands/results. Do not inspect paths outside this fixture,
use network access or perform an external effect.
