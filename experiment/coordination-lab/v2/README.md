# Coordination lab v2

Scratch-only successor to the v0/v1 experiment in [`../RESULTS.md`](../RESULTS.md).

V2 keeps the seven model-facing commands and replaces the failed substrate:

- one host process owns SQLite, outbox mutations, and chronological trace writes;
- container MCP servers are thin TCP clients and never mount/open the database;
- each Work gets a persistent, workspace-scoped execution cell across retries;
- actor + Work + revision + Attempt + lease token are validated on callbacks;
- redirects against running Attempts request cancellation before changing Work;
- idempotency keys are caller-supplied and atomically bind request to result;
- a worker that ends without a terminal callback remains explicitly `unknown`, with its workspace preserved;
- only `integration-lead` can hold the single integration lease.

Run deterministic substrate faults before any model scenario:

```sh
cargo build --release --manifest-path experiment/coordination-lab/Cargo.toml
bash experiment/coordination-lab/v2/run.sh fault-test faults
```

Then prepare and run the same fixed-seed scenario:

```sh
bash experiment/coordination-lab/v2/run.sh prepare v2
bash experiment/coordination-lab/v2/run.sh run v2
```

Generated databases, cells, workspaces, prompts, and traces live under ignored `v2/workdir/`.

Before a GPT-5.6 coordination comparison, prove one real Terra producer can return an exact commit and
terminal callback without treating the probe as an organisational arm:

```sh
bash experiment/coordination-lab/v2/run.sh first-party-callback-probe exp01-terra-handoff
```

## V21 artifact-led comparison

V21 keeps the v2 execution substrate and changes only the coordination architecture. It compares:

- `single_agent` — the Sonnet-class model owns the canonical candidate directly;
- `graph_control` — the same model leads through the existing Work/integration graph; and
- `artifact_led` — the same model is a persistent Game Product Lead with a writable canonical
  candidate and `/company/project-state.md`, while free OpenRouter specialists return exact commits.

In `artifact_led`, the Work graph is only the sparse map of cross-actor responsibilities. Worker
completion wakes the lead; the lead integrates and verifies the native candidate directly. Every
worker Attempt receives the owner directive, current project state, exact Work, recovery diff, and
relevant upstream artifacts.

First prove those seams without model inference:

```sh
cargo build --release --manifest-path experiment/coordination-lab/Cargo.toml
bash experiment/coordination-lab/v2/run.sh architecture-test v21-architecture
```

Prepare the matched three-arm smoke experiment. Pin the exact Sonnet model and worker pool. Before a
team arm spends a lead wake, every worker is checked twice: the live OpenRouter catalogue must still
show zero prompt/completion price, text input, and tool support; then the exact Company Runtime must
refresh its model registry and complete a tiny inference through the credential broker and dedicated
gateway. Catalogue-only availability is not treated as connectability.

```sh
bash experiment/coordination-lab/v2/run.sh experiment-prepare v21-smoke \
  --lead-model anthropic/claude-sonnet-4-5 \
  --worker-pool cohere/north-mini-code:free,poolside/laguna-s-2.1:free \
  --spend-ceiling 6 \
  --wall-clock-seconds 1800

bash experiment/coordination-lab/v2/run.sh experiment-run v21-smoke
```

The per-arm spend ceiling applies independently. The experiment is not a success unless the
artifact-led arm advances one clean playable candidate, integrates at least two delegated
contributions, passes executable checks, and receives independent artifact review without ordinary
owner intervention. Work count, tokens, wall time, provider failures, candidate evidence, project
state, and exact model allocation remain inspectable in each run directory.
