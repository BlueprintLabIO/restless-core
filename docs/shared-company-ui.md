# Shared Companies and account shell

Core's Companies route and Cloud's account adapter use `CompanyPortfolio` and
`AccountShell` from the existing `@restless/ui` artifact. The Companies component
owns the shell, header, rows, loading state and empty state. Other account routes
can render `AccountShell` directly. Load `@restless/ui/style.css` once in the host;
fonts and the document reset remain the host's responsibility.

`CompanyPortfolioEntry` contains observed company facts and an optional entry
capability. Core supplies a native link; Cloud supplies a native POST action and
its existing organization field. Creation, retry, account navigation and other
controls are host-owned snippets. The components do not fetch, invent a company
identity, create browser state or reach into either application's stores.
Missing focus, next work or attention facts remain unavailable. Only an observed
zero attention count is rendered as “Nothing now”. A company without an entry
capability has no entry control.

Core keeps its existing company queries, appliance recovery, creation dialog and
configuration-repair routes. Cloud keeps Fleet membership, durable start state,
authentication and its released Core account-plane entry flow. The shared UI
artifact is pinned by source revision and checksum independently of the Runtime
image. This extracts the useful canonical UI intent from the older whole-Core
archive proposal without adopting its obsolete embedded Runtime transport.

Validation: `cd web && npm run check && npm run smoke:ui && npm run build`.
The artifact smoke renders the package in a clean Svelte consumer and checks
local links, hosted POST identity, unknown facts, observed zero attention and
unavailable company entry. Core's import-boundary and type-ramp checks apply.
