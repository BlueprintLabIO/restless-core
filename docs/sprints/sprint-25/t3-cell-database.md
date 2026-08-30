# S25-T3 — Give each cell its own database and role

Move OrgIntel from schema-per-company inside one shared database to a dedicated database with its own
role per cell.

**Observed friction:** `0001_init.sql:1` — "Applied once per company schema; the company name is the
schema name, set via search_path." One database, one role, `search_path` separation. A single
connection can read every company's schema, so the isolation is a convention the code politely
observes rather than a boundary the database enforces.

**Layer:** OrgIntel (cell). The store is company-scoped, so its credential must be too.

**Deletion target:** shared-role schema separation and cross-schema reachability.

## Scope

- One database and one role per cell; the cell receives only its own connection string.
- Migrations apply per database rather than per schema.
- **Prove the wake path first.** `0002_notify_triggers.sql` / `0003_notify_table_schema.sql` derive the
  company from `TG_TABLE_SCHEMA` and publish it on `LISTEN/NOTIFY`. That is the load-bearing wake
  mechanism, and it becomes per-database. Verify on one company before migrating any other.
- Migration path for existing companies: dump schema → restore into new database → verify wake →
  cut over → drop schema.

## Acceptance

Connect with cell A's credentials and fail to read cell B's tables. A wake triggered in cell A reaches
its lead exactly as before.

## Closure evidence

Provisioned and verified on an isolated home with one throwaway company:

- `restless_cell_celltest_test` database and role created; `cells/<company>/database.url` written at
  `0600`; migrations applied inside the `celltest_test` schema **within that database**.
- **The company name stays the schema name inside its own database.** Flattening to `public` would
  have broken wake attribution: the triggers derive the company from `TG_TABLE_SCHEMA`, so every wake
  would have claimed to come from a company called "public". Isolation comes from the database and
  role boundary.
- `REVOKE CONNECT ... FROM PUBLIC` on each cell database — Postgres grants it by default, so without
  it any other cell's role could open its neighbour.
- **Isolation observed:** as the cell role, `select count(*) from aris.actors` →
  `ERROR: permission denied for schema aris`.
- **Wake path proven, not assumed.** A raw `LISTEN restless_orgintel` on the cell database received
  `{"company":"celltest_test","kind":"message",...}`, and the scheduler then logged `cell wake
  received` for the same payload — delivery confirmed end to end.
- The single admin-connection listener was replaced by **one listener per cell, multiplexed**. This
  was the load-bearing risk: one listener on the admin connection would have heard nothing and
  silently degraded every wake to the 5s scan.
- Legacy import is non-destructive and rolls back on failure — a half-restored schema would otherwise
  look complete to the next boot and run the cell on partial history.
