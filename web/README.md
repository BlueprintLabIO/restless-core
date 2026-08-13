# web — the control-plane SPA

The operator's surface: a calm main work surface plus a right-hand executive chat that can
focus, explain, and act on it.

> **Status: unwired.** There is no server, no data loading, no API, and no authority path.
> Every surface renders from `src/lib/fixtures/`. This is the design and the component
> layer, standing on its own so it can be reviewed before anything is connected to it.

```
pnpm install
pnpm dev        # http://localhost:5180
pnpm check      # svelte-check — must be clean
pnpm build      # static build via @sveltejs/adapter-static
```

## Where it came from

Ported from the prior control plane's SPA at its 2 August state — the build the founders
selected. `docs/SALVAGE.md` calls that SPA "the strongest non-Rust salvage". Design canon is
that system's `docs/design-language.md`, codenamed **Bridge**: the dark cockpit, where the
default state is quiet and annunciation therefore means something.

Two things were deliberately changed in the lift:

- **Brand-neutral.** `v3-*` / `v2-*` class prefixes became `bridge-*`, and every display
  name comes from `src/lib/brand/brand.ts`. Nothing else names the product.
- **Components take view models, not a database projection.** In the source, surfaces took a
  raw `CompanyDeskView` and called mappers inline. Here they take already-mapped props from
  `src/lib/model/view.ts`. That puts the seam exactly where wiring will happen.

## Layout

| Path | What it is |
| --- | --- |
| `src/lib/design/` | The Bridge stylesheet, split along its own section boundaries. **Import order is the cascade** — see `index.css`. |
| `src/lib/primitives/` | The small pieces: `MatrixGlyph`, `HoldApprove`, `Composer`, `PaneHeader`, `Hint`, `Markdown`, `KanbanCard`, and their pure logic modules. |
| `src/lib/components/` | The composed surfaces: `AppShell`, `ExecutiveRail`, `CommandPalette`, `StartModal`, `OpsSurface`, `PeopleSurface`, `MissionSurface`. |
| `src/lib/model/` | View-model types plus the pure composers lifted wholesale — work board, authority board, cost attribution, runway forecast, market view, vendor reputation, asset renderer, markdown parser. No framework imports. |
| `src/lib/fixtures/` | Cosmon, the reference company (ARCHITECTURE.md §10). Fixture data only. |
| `src/routes/` | The surfaces. |

## How the unwiring works

Reads come from fixtures. **Writes are callback props that default to null**, typed
`(...) => Promise<string | null>` — an error message, or null on success. Where a callback is
absent the affordance either stays inert or hides itself; nothing posts into a void, and
nothing pretends to have saved.

That shape is not a stub. Every write in this product is a governed change that has to travel
an authority path and land on the record. A component that called `fetch()` itself would be a
second write path. The surface states the intent; the caller owns the authority.

Two props exist for the same reason and must never be defaulted to something optimistic:

- `executiveConnected` — whether the executive has a live runtime.
- `connections` — what is actually wired, with `status` verbatim from the check.

Both must come from a **live probe**. "Never checked" is a real answer and a different claim
from "working"; the connections pane renders it as such.

## Wiring it later

Replace `$lib/fixtures/cosmon` with a client that returns the same `DeskView`, and pass real
callbacks for the writes. No component changes.
