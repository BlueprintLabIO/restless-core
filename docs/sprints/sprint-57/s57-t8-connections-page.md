# S57-T8 — Company → Connections

**Layer:** Cockpit. **Serves:** the owner had no place to add a service, see what it offers or decide
how each tool is governed.

`/[company]/company/connections` lists connections with their observed status and tool count, and
suggests known endpoints (GitHub, Linear, Notion, Stripe, Sentry). The probe decides whether a
suggestion works. Add takes a URL (optionally a Vault token name), a local command or a plugin Git
URL. Expanding a connection shows its tools grouped Reads freely / Acts with a receipt / Asks you
first, with the class changeable in place, one grant for every agent, Freeze, recent receipts, and
Sign in when the server asks for it. A URL added without auth becomes an OAuth connection when the
owner signs in.

**Evidence:** a Playwright run against the Vite cockpit with the connection API stubbed. It added
GitHub with a Vault token, changed a class, granted, froze, and checked there was no horizontal
overflow at 390 px. Not yet run against a live owner gateway.
