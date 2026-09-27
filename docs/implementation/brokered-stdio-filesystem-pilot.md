# Brokered local stdio: filesystem read pilot

This is the first reviewed `broker_stdio` profile. Core exposes only the
published filesystem MCP server's `read_text_file` tool. The owner chooses one
host directory to mount as `/data`; Core accepts a call only for one existing
regular file within that directory, with a 256 KiB source-file limit and the
gateway's 1 MiB result limit. Tool name and arguments are checked before the
provider starts. MCP annotations are not treated as authority. The provider's
other tools are never advertised by Core or accepted for invocation.

The owner stages a bundle directory containing an executable `node` file and
the [official filesystem server](https://github.com/modelcontextprotocol/servers/tree/main/src/filesystem),
`@modelcontextprotocol/server-filesystem@2026.8.31`, with its dependencies at
`node_modules/@modelcontextprotocol/server-filesystem/dist/index.js`. The
package name and version are checked at install and call time. The bundle and
data directory must be absolute, existing directories; Core stores their
canonical paths. No bundle files,
provider credentials, or host filesystem path reach the company Runtime. In
an isolated company with a proposed or blocked Work owned by the named Staff
actor, install with:

```sh
restless local-mcp --company <company> install-stdio-read \
  --name local-docs-read --bundle <absolute-bundle-directory> \
  --read-root <absolute-data-directory> --actor <actor> --work <work-uuid>
```

Installation starts the provider in the same sandbox used for calls, discovers
the exact tool, and pins its server version and tool definition digest. A
fresh Attempt receives only Core's HTTP MCP route and an expiring grant bound
to company, actor, Work, Attempt, and connection. Core rechecks the live
Attempt, current connection, tool pin, file path, and result bound. The owner
can disable the connection with the existing `local-mcp disable` action.

The worker is a fresh `bubblewrap` process for discovery or each call, with
an empty environment, separate user/mount/PID/network namespaces, the bundle
and data directory mounted read-only, `/tmp` as temporary storage, and only
system binaries/libraries otherwise mounted read-only. It does not inherit
Core's bearer, OAuth state, home directory, or network. If bubblewrap or user
namespaces are unavailable, installation and calls fail closed. The selected
data directory is intentionally readable by the assigned actor through the
tool; owners must choose a directory that contains no secrets they do not
want that actor to see. This pilot does not isolate same-UID actor processes
from stealing a different live Attempt's Core grant file; see the capability
limit in [Core-owned MCP broker](core-owned-mcp-broker.md).

Core's common broker path appends started and terminal call receipts with
company, actor, Work, Attempt, tool, outcome, contract digest, and policy
revision. The stdio path still needs a real Staff Attempt to verify those
receipts end to end. The bundle is owner-managed and this pilot does not hash
its entire dependency tree; a hostile host user able to mutate it could change
the provider's answers without changing its advertised MCP contract.

## Isolated provider smoke, 2026-09-28

The official published `@modelcontextprotocol/server-filesystem@2026.8.31`
was staged in a disposable bundle with Node 24.19.0. Its MCP handshake
reported `secure-filesystem-server` version `0.2.0`, exposed 14 upstream
tools, and included `read_text_file`. With the production bubblewrap arguments,
`read_text_file` returned an exact 6,342-byte copy of a chosen Markdown file.
Calling the upstream `write_file` tool directly reported an error and left the
read-only mounted file unchanged. A separate TCP connect attempt in the same
namespace failed with `ENETUNREACH`. These observations validate the real
provider and sandbox envelope; they do not prove that a company Staff Attempt
has invoked the new Core path. That Attempt and grant-revocation smoke remain
release checks.

The npm package manifest points to the official `modelcontextprotocol/servers`
repository and says `SEE LICENSE IN LICENSE`; the published tarball did not
include that license file in this smoke. The repository's
[LICENSE](https://github.com/modelcontextprotocol/servers/blob/main/LICENSE)
describes an MIT to Apache-2.0 transition. Review license packaging before
redistributing a staged provider bundle.
