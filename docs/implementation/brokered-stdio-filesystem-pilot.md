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

The worker is a fresh Docker container for discovery or each call. The owner
sets `RESTLESS_STDIO_MCP_IMAGE_ID` to the `sha256:<64-hex>` ID of a locally
available reviewed Runtime image. Core runs it with `--pull=never`,
`--network none`, a read-only root, no Linux capabilities, no new privileges,
PID and memory limits, numeric host user/group identity, a 16 MiB temporary
directory, and only the bundle and selected data directory mounted read-only.
The provider has a 120-second in-container lifetime cap, and startup has a
30-second bound. It does not inherit Core's bearer, OAuth state, home
directory, Docker socket, or network. Missing images and Docker failures fail
closed. Core reports only a fixed stderr class, byte count, and worker exit
status on handshake failure, never arbitrary provider stderr. Each invocation
has a unique Docker name; Core removes only that exact container after startup
failure or normal child exit because Docker's `--rm` can leave a container in
Created state if startup is interrupted. After failed startup, an exact-name
watch continues for the worker lifetime in case Docker finishes creating the
container after its CLI exits. Normal child exit needs one cleanup pass. The selected
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
tools, and included `read_text_file`. With the original bubblewrap arguments,
`read_text_file` returned an exact 6,342-byte copy of a chosen Markdown file.
Calling the upstream `write_file` tool directly reported an error and left the
read-only mounted file unchanged. A separate TCP connect attempt in the same
namespace failed with `ENETUNREACH`. In the live systemd AppArmor context,
however, bubblewrap failed before MCP initialization: `loopback: Failed
RTM_NEWADDR: Operation not permitted`. A disposable Rust client using the same
MCP transport reproduced that failure. The same client then initialized the
published provider and listed its 14 tools through the Docker worker with the
restrictions above. A canceled trial left no labeled worker container in
`docker ps -a`. These observations validate the provider and revised worker
startup; they do not prove that a company Staff Attempt has invoked the new
Core path. That Attempt and grant-revocation smoke remain release checks.

The npm package manifest points to the official `modelcontextprotocol/servers`
repository and says `SEE LICENSE IN LICENSE`; the published tarball did not
include that license file in this smoke. The repository's
[LICENSE](https://github.com/modelcontextprotocol/servers/blob/main/LICENSE)
describes an MIT to Apache-2.0 transition. Review license packaging before
redistributing a staged provider bundle.
