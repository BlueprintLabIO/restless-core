# S57-T5 — Host-side local MCP worker

**Layer:** Gateway/Runtime. **Serves:** local MCP servers ran inside the Runtime with whatever
credentials the Runtime held.

A `local` connection runs as `docker run --rm -i` from the company image with all capabilities
dropped, no new privileges, a tmpfs `/tmp` and a per-company package cache volume. Credentials are
passed by name only; their values exist in the Docker CLI process environment, never in argv or the
Runtime. Sessions are pooled per connection revision and closed when idle.

**Evidence:** not yet run against a real local server; T10 acceptance 3 covers it.
