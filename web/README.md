# web — the control-plane SPA

The operator's surface: a calm main work surface plus a right-hand executive chat that can
focus, explain, and act on it.

> **Status: primary cockpit wired.** Attention, Work, People, Authority, the situation strip,
> and the executive rail read the company-scoped owner APIs. Superseded fixture-backed owner
> routes were removed rather than retained as a second product.

```
npm install
# once per checkout; ~/.local/bin is already on the development PATH
ln -s "$PWD/../scripts/restless-dev" "$HOME/.local/bin/restless-dev"
cargo install --locked --path ../crates/restless --root "$HOME/.local"
restless-dev aris
                    # live daemon + company computer + Vite at http://localhost:5173/aris
restless doctor -c aris
                    # read-only check of every local boundary
npm run check   # svelte-check — must be clean
npm run build   # static build via @sveltejs/adapter-static
```

Do not start Vite alone for the live cockpit: its shell can render while every `/api` request is
unavailable. `restless-dev` keeps Vite on the owner gateway's same-origin API contract.

## Where it came from

Ported from the prior control plane's SPA at its 2 August state — the build the founders
selected. `docs/SALVAGE.md` calls that SPA "the strongest non-Rust salvage". It is now the
**Bridge Light** cockpit: full-screen work panes over the dot matrix, restrained glass depth,
and stable semantic colour for conversation, feedback, direction, authority, and outcomes.
The prior dark palette and speech-bubble chat are not fallback designs.

Two things were deliberately changed in the lift:

- **Brand-neutral.** `v3-*` / `v2-*` class prefixes became `bridge-*`, and every display
  name comes from `src/lib/brand/brand.ts`. Nothing else names the product.
- **Components take view models, not a database projection.** In the source, surfaces took a
  raw `CompanyDeskView` and called mappers inline. Here they take already-mapped props from
  `src/lib/model/view.ts`. That puts the seam exactly where wiring will happen.

## Layout

| Path                  | What it is                                                                                                                                                                           |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/lib/design/`     | Bridge Light. `tokens.css` owns colour, geometry, spacing and depth; `cockpit.css` owns the live shell and four primary surfaces. **Import order is the cascade** — see `index.css`. |
| `src/lib/primitives/` | The small pieces: matrix glyphs, bounded confirmation, Markdown, attachments and the composer.                                                                                       |
| `src/lib/components/` | The one shell, situation strip and persistent executive transcript.                                                                                                                  |
| `src/lib/model/`      | The narrow owner-surface contract, generated OrgIntel rows and company-scoped API clients.                                                                                           |
| `src/lib/office/`     | The live Attention-room adapter: source-owned People/Work projection, asset decoding and the Svelte canvas boundary.                                                                 |
| `src/routes/`         | Company door plus Attention, Work, People and Authority.                                                                                                                             |

The room vendors Pixel Agents 1.4.1's office engine and artwork at pinned commit
`3537e140c2094761beae748592aeb92ece8edfdd`. Its renderer, sprites, furniture, seat assignment,
pathfinding and character state machine remain upstream code; Restless supplies only the browser asset
adapter, the company-shaped layout and a read-only projection from People/Work. See
`src/lib/vendor/pixel-agents/NOTICE.md` and `static/vendor/pixel-agents/LICENSE`.

## Data boundaries

The primary cockpit reads `/api/companies/:company/attention` and
`/api/companies/:company/cockpit`; owner messages, attachments, review decisions and bounded
authority actions use their company-scoped endpoints. Components still receive mapped view
models rather than database rows.

Live state is never inferred from configuration. Runtime availability, source health,
credentials and effect receipts come from probes or source-owned projections. Fixture state is
not presented as a live company capability.
