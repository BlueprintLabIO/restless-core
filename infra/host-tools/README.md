# OMP Responses namespace bridge

The pinned `@oh-my-pi/pi-ai` 18.3.2 auth gateway omits Codex Responses
`type: "namespace"` tools. This patch carries namespace definitions and
`{ namespace, name }` calls through the gateway, provider request, response,
and history replay. It also accepts namespace tools in a Responses Lite
`additional_tools` input item. Same named children in different namespaces
remain distinct on the wire.

`npm --prefix infra/host-tools ci` runs `apply-pi-ai-patch.mjs` after install.
`scripts/restless-dev` checks the patch again when it reuses an existing
host-tools install. The script requires the pinned package version and a clean
patch context, and is safe to rerun. It uses the system `patch` command.

For an offline protocol smoke after installation:

```sh
infra/host-tools/node_modules/.bin/bun infra/host-tools/smoke-namespace.ts
```

The smoke exercises the installed gateway and provider conversion code with
sanitized protocol items. It does not make a model request. A live actor MCP
call through Restless Core is still required before claiming deployed use.

When upgrading OMP, inspect upstream namespace support, remove this patch if
the gateway and providers preserve the namespace pair natively, and rerun the
smoke against the new pinned version.
