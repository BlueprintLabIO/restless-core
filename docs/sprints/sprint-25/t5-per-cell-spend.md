# S25-T5 — Per-cell spend ledger with plane rollup

Give each cell its own ledger; compute owner totals by reading up.

**Observed friction:** one `spend.jsonl` per installation with a `companyId` field on every row — the
same shared-store-with-a-company-column pattern as schema tenancy, and it fails the cross-layer
contract §1.4 rule that no store may span companies as source of truth.

**Layer:** Authority Plane writes per-cell; account plane aggregates.

**Deletion target:** the installation-wide ledger file and company-column filtering.

## Scope

- Ledger per cell, alongside that cell's state.
- Owner rollup is a computed projection, never a second writer.
- Ceiling enforcement stays in the account plane, beside the credential — a cell must not be able to
  raise its own ceiling by writing its own ledger.

## Acceptance

Deleting one cell's ledger does not corrupt another's. The owner total equals the sum of cells.

## Closure evidence

- `SpendLedger` holds one `SpendStore` per cell under `cells/<company>/spend/`, opened on first use.
- `each_cell_ledger_holds_only_its_own_history` proves a cell takes only its own rows out of the
  legacy shared spool and keeps its accumulated total. Starting a migrated company from zero would
  silently raise its budget, because the ceiling is enforced against history.
- The shared spool is left in place for operator verification.
- A cell whose ledger cannot be opened reports `MeteringUnknown`, never "spent nothing" — an
  unopenable ledger must not read as a full budget.
- Two tests that asserted the installation-wide spool path were updated to the cell layout; that
  assertion was the shared-store assumption written down.
