# web — the control-plane SPA

The operator's surface: four destinations across the top, the work in the middle, and the
executive docked on the right of every screen.

> **Status: wired to the daemon, partly backed.** Every surface reads from `restlessd`'s owner
> gateway on `127.0.0.1:7788` — `/v1` for the cockpit's reads and writes
> (`docs/api/openapi.yaml`, rendered at `/v1/docs`), `/api` for the attention queue, approvals
> and the browser handover. Where an endpoint does not exist yet the surface says so and names
> the gap — it does not fall back to sample data. `docs/api/MISSING.md` lists what is missing
> and which page needs it.

```
pnpm install
pnpm dev        # http://localhost:5180 — proxies /v1 and /api to restlessd on 7788
pnpm check      # svelte-check — must be clean
pnpm build      # static build via @sveltejs/adapter-static
```

**You must sign in.** Everything except `/v1/health`, `/v1/openapi.yaml` and `/v1/docs` needs
the owner credential. Generate it with `restless owner-token --rotate` — it is printed once and
only its digest is stored — and paste it into the Inbox's sign-in panel, which exchanges it for
an HttpOnly cookie at `POST /api/session`. Signing in through the Vite proxy works because the
proxy keeps the browser same-origin with the gateway; a cross-origin fetch would never send the
cookie.

## The shape

Four surfaces, and nothing else is a place you navigate to:

| Surface | What it is |
| --- | --- |
| **Inbox** | A stack you clear, one card at a time, with the rest felt underneath. First in the nav because it is the only surface that can be waiting on you. |
| **People** | A directory of who is doing what, and the page of whoever you picked. A reporting-tree arrangement is drawn but unbacked — `actors` has no parent column yet. |
| **Board** | Goals as a tree beside the columns the work flows through. Two levels, goal → commitment, because that is what OrgIntel has; the four-level version in `design.pen` is a drawing. |
| **Authority** | Every standing setting in one flat list. **Nothing here is backed yet** — the settings live in a config file with no read path. |

Profiles, files, the record, and hiring are detail views reached from a row inside one of the
four. They are not destinations.

**The executive is on every screen.** `ChatDock` is the escape hatch: anything the UI can do
you can also just ask for, which is why she is present on surfaces that have nothing to do
with chat. She states what she can see and act on rather than implying it, and every claim
that something happened carries a receipt of what actually moved.

**She collapses.** `ChatRail` is the same executive at 52px — expand control, avatar, a count
of what is waiting, and `⌘J`. Collapsing costs width, not information. The Board is where
this pays: the dock's 380px is a whole kanban column, so `Completed` is marked `secondary` in
the view model and stands down while she is expanded rather than squeezing every column below a
readable width. Collapse is remembered per surface, because what
380px buys differs by screen.

## Layout

| Path | What it is |
| --- | --- |
| `src/lib/design/` | The stylesheet, ported from `design.pen`. **Import order is the cascade** — see `index.css`. |
| `src/lib/components/` | The chrome: `AppShell`, `TopNav`, `ChatDock`, `ChatRail`, `Avatar`, `Icon`. |
| `src/lib/surfaces/` | The four destinations, one component each. |
| `src/lib/model/` | View-model types, and the per-surface dock collapse state. No framework imports beyond runes. |
| `src/lib/api/` | `client.ts` — the transport and the envelope. `map.ts` — wire shapes to view models. |
| `src/routes/` | One route per surface. `/` redirects to `/inbox`. |

## Design source

`design.pen` in this directory is the source; `src/lib/design/tokens.css` is the port. Every
token value in that file appears in the `.pen` variable table. **If you change one, change
both** — otherwise the drawing and the build disagree and neither is canon.

Light is the default and dark is a switch. The system preference deliberately does not get a
vote: the founders picked light, and `prefers-color-scheme` would silently overrule that. Dark
is reached only by `data-theme="dark"`, applied pre-paint in `app.html` so the ground never
flashes.

## Three outcomes, not two

`client.ts` returns `ok` / `stub` / `failed`, and surfaces render all three differently. That
third state is the point: a stub answers `data: null`, and a UI that collapsed it into "empty"
would tell the owner their company has no authority settings rather than that the list was never
built. `Unbacked.svelte` is what renders it.

`failed` keeps the daemon's `error.kind`. An owner who sees `authority` needs to change what
they are allowed to do; one who sees `transport` needs to start the daemon. Flattening both into
"something went wrong" throws away the only part that says what to do next.

## How the writes work

**Writes are callback props that default to null**, typed
`(...) => Promise<string | null>` — an error message, or null on success. Where a callback is
absent the affordance stays inert; nothing posts into a void, and nothing pretends to have
saved. The dock's composer is the live example: without `onSend` the send button is disabled
and says why.

That shape is not a stub. Every write in this product is a governed change that has to travel
an authority path and land on the record. A component that called `fetch()` itself would be a
second write path. The surface states the intent; the caller owns the authority.

## Wiring it later

Run `restlessd` (`scripts/restlessd.sh`), then `pnpm dev`. Vite proxies `/v1` and `/api` to
`127.0.0.1:7788`; the gateway is loopback-only and does not do CORS, deliberately.

Point the SPA at a company with `VITE_RESTLESS_COMPANY`, or `localStorage.setItem('company', …)`.
There is no company switcher yet. Authority behind the cookie is still one principal: every
`/v1` request is stamped `principal: "owner"`, and the daemon's `authorize` gate decides from
there. The credential proves you hold the owner token, not which human you are.

What is still fixture-free but unbacked: the Authority surface, the reporting tree, per-person
authority, and artifacts. All four are registered stub routes and the surfaces say so.

The attention queue is **no longer** among them — the Inbox renders the real projection from
`GET /api/companies/{company}/attention`, and the count on the nav is the same read.
