# OMP Responses namespace bridge

The pinned `@oh-my-pi/pi-ai` 18.3.2 auth gateway omits Codex Responses
`type: "namespace"` tools. This patch carries namespace definitions and
`{ namespace, name }` calls through the gateway, provider request, response,
and history replay. It also accepts namespace tools in a Responses Lite
`additional_tools` input item. Same named children in different namespaces
remain distinct on the wire.

`npm --prefix infra/host-tools ci` runs `apply-pi-ai-patch.mjs` after install.
`scripts/restless-dev` checks the patch again with a working Bun when it reuses
an existing host-tools install. The launcher selects `omp-source` as the broker
executable. This is necessary because npm's `.bin/omp` runs a prebuilt CLI
bundle containing its own copy of the old, namespace-blind gateway; changing
the installed `pi-ai/src` files does not change that bundle. The source CLI
imports the patched gateway through the package exports. The script requires
the pinned package version and a clean patch context, and is safe to rerun.
It uses the system `patch` command.

An already installed first revision of this patch is upgraded in place.

The gateway currently does not forward Codex's `tool_search` tool. Namespaced
children therefore become directly visible: their incoming `defer_loading`
flag is omitted from the provider request. This keeps native calls usable when
the caller supplies deferred children without a search tool. Tool-search
discovery itself remains a separate integration task.

The Bun resolver tries `RESTLESS_BUN_BIN`, then a working `bun` on `PATH`, then
the host-tools shim and installed native Bun packages. It checks each candidate
with `--version`; this also avoids an executable but unusable npm shim after
an install with lifecycle scripts disabled.

For an offline protocol smoke after installation:

```sh
infra/host-tools/smoke-namespace
```

The smoke starts the source CLI and exercises the installed gateway and
provider conversion code with sanitized protocol items. It does not make a
model request. A live actor MCP call through Restless Core is still required
before claiming deployed use.

When upgrading OMP, inspect upstream namespace support, remove this patch if
the gateway and providers preserve the namespace pair natively, and rerun the
smoke against the new pinned version.
