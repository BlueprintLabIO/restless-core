# Coordination lab

Local-only experiment for the seven proposed OrgIntel coordination commands. It uses the existing
Cosmon repository at exact commit `514b7b3`, isolated ACP sessions, a scratch SQLite store, real Git
worktrees, and the host OMP model credential broker. It does not write production OrgIntel state.

Generated state lives under ignored `workdir/`.

The completed v0/v1 experiment and its evidence are recorded in [`RESULTS.md`](./RESULTS.md). Do not
reuse this harness as production infrastructure: v1 proved that opening a bind-mounted SQLite WAL
from both the macOS host and the Linux container is unsafe, and concurrent processes also corrupted
the direct JSONL trace. The report describes the single-writer replacement to test next.

```sh
cargo build --release --manifest-path experiment/coordination-lab/Cargo.toml
experiment/coordination-lab/run-lab.sh preflight
experiment/coordination-lab/run-lab.sh prepare v0
experiment/coordination-lab/run-lab.sh run v0
```

The runner exposes seven typed mutations (`send`, `commission`, `redirect`, `report`,
`request_judgement`, `decide`, `schedule`) plus one read-only state projection. `claim_ready` and
wake dispatch remain substrate operations.
