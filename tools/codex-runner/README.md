# GPT-6 Sol tool catalog compatibility

The company image pins Codex CLI 0.155.1. Its bundled model catalog predates
`gpt-6-sol`, so Codex falls back to generic metadata and omits `apply_patch`
from its code-mode tool list. For exact `openai-codex/gpt-6-sol` sessions, the
runner loads `gpt-6-sol-0.156.1-model-catalog.json` via `model_catalog_json`.
The image sets the catalog path; other models and hosted runners do not use it.

The catalog is the `gpt-6-sol` entry from [Codex 0.156.1's official model
catalog](https://github.com/openai/codex/blob/rust-v0.156.1/codex-rs/models-manager/models.json),
wrapped as `{"models":[...]}` and formatted with two-space JSON indentation.
Its SHA-256 is `04187020317177396a94b9a9e599a29118257bd93ff4e802ee5c7a2616972d23`.
The runner checks this digest before launch, and Core checks both the runner
and catalog digests before accepting local Sol session readiness.

Codex advertises a namespaced custom `functions.exec` tool in a Responses Lite
`additional_tools` item. Its description includes `apply_patch`, and the
existing OMP namespace bridge preserves its raw input and call namespace.
The offline smoke at `infra/host-tools/smoke-namespace` covers that wire path.

When updating the Codex CLI pin to a version with native `gpt-6-sol` metadata,
remove this catalog override after verifying the model-visible tool list and
a live edit through the model gateway.
