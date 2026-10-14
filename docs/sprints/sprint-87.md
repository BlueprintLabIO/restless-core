# Sprint 87 — An app store that is never empty

**Status:** in progress
**Programme:** capabilities; cold start
**Depends on:** Sprint 83 (Apps I and II).

## Outcome

On 14 October:
- Exec suggested two skills, Frontend Design and Taste Skill, as "candidates" the owner had to find
  and accept in Apps.
- A chat reply could not grant them.
- Apps had little to browse.

A new company starts with nothing in Apps, and Exec can only suggest what it already knows.

When this sprint is done, Apps lists skills and MCP servers drawn from well-known free registries. It is
filled on a schedule and searchable. Each item shows what it is, where it comes from, what it can touch
and whether it runs scripts. Installing is one deliberate click. Exec recommends from the same catalogue,
with a link that opens the item.

## Sources

Only sources with a public, unauthenticated or free API that allows listing. Each is checked for terms
and rate limits before it is enabled, and that check is recorded in the source's entry.

- **The official MCP Registry** (`registry.modelcontextprotocol.io`, `GET /v0/servers`, paginated). It
  gives server name, description, repository, packages and remotes.
- **GitHub skill repositories** that follow the `SKILL.md` convention, starting with
  `anthropics/skills`. Read through the GitHub contents API, unauthenticated with backoff, or with the
  plane's token when one is set.
- **Further directories** (for example agenticskills.io) only if they publish an API or a feed and
  their terms allow reuse. Otherwise they are linked, not scraped.

## Changes

**S87-01. Catalogue store.**
- A plane-level `catalogue_items` table, keyed by `(source, source_id)`. Each item has:
  - kind (skill or mcp) and name;
  - summary and description;
  - homepage, repository and licence;
  - install spec (an MCP package or remote, or a skill archive path);
  - permissions (network, files, scripts);
  - the time it was fetched and the source's revision.
- No company data goes into it. Every company on the plane reads it.

**S87-02. Fetcher.**
- A plane job polls each enabled source daily, with ETag and If-Modified-Since where offered.
- It respects rate limits and stores items, marking those that vanish as withdrawn.
- A source that fails keeps its last good items and records the error.

**S87-03. Apps › Browse.**
- Search and kind filters.
- Each row shows: name, one-line summary, source badge, "Runs scripts" when it does, and Install.
- The detail sheet shows:
  - the description;
  - the repository link and licence;
  - what it can access;
  - the exact files or package that will be installed.

**S87-04. Installing.**
- Installing a skill copies the fetched archive into the company's skills as a candidate, already
  accepted by this click.
- A skill with scripts asks for a second confirmation that names the scripts.
- An MCP server is added through the existing connection flow, prefilled from the install spec.
- Nothing installs from a chat reply.

**S87-05. Exec recommends from the catalogue.**
- `restless apps search <words>` returns catalogue items.
- Exec's suggestion is a link that opens the item in Apps (`/{company}/apps?item=…`). The owner
  installs it there.

## Acceptance

- With network access, a fetch of the MCP Registry stores at least 100 servers. A fetch of
  `anthropics/skills` stores its skills. A second fetch with no changes writes nothing.
- Apps › Browse on a new company lists catalogue items, and search finds "frontend design".
- Installing a skill with scripts needs the second confirmation. Installing one without scripts makes
  it usable by the next turn.
- Exec's `restless apps search` returns the same items, and its link opens the item.
- Unit tests cover parsing of each source, using saved fixtures; no network is needed in CI.
