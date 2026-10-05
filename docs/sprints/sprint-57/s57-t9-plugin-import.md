# S57-T9 — Plugin bundle import

**Layer:** Gateway/Cockpit. **Serves:** acceptance 4. A public plugin should bring its servers and
skills without hand transcription.

`POST /tool-connections/plugins {url}` clones an https Git URL on the host. It reads the
`.codex-plugin` or `.claude-plugin` manifest and `.mcp.json`, then adds each server as a connection
awaiting probe and grant. A credential must be a `${NAME}` placeholder naming a Vault secret;
literal secrets and paths that leave the bundle are refused. Skills belong in the company
computer, so Exec is asked to add each one with `restless skill add <url>#<path>`, and they reach
Skills as candidates.

**Evidence:** `connections::tests::plugin_bundles_declare_servers_and_skills_without_leaving_the_bundle`.
The Git clone and Exec's skill step have not run live.
