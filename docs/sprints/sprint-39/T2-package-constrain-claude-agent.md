# T2 — Package and Constrain Claude Agent

**Serves:** Sprint 39 — Work Through Attention With Verified Agent Harnesses  
**Layer:** Runtime Image + Runtime Bridge + Authority  
**Depends on:** T0, T1

## Outcome

Every company runtime can launch the exact admitted Claude Agent build under a Restless-owned policy envelope, with no ambient configuration or reusable provider credential entering Runtime.

## Work

- Add the pinned `claude-agent-acp` distribution and required runtime to the company image using integrity-checked, reproducible installation.
- Expose the installed adapter and bundled Claude Code/Agent build identities to the readiness probe and launch manifest.
- Give each company/harness an isolated `CLAUDE_CONFIG_DIR` and bounded session storage path.
- Construct the Claude launch from the certified profile: exact cwd, Restless system instructions, actor identity, selected model/effort, approved tools, explicit MCP servers, and session scope.
- Disable or exclude ambient user settings, plugins, hooks, MCP servers, cached approvals, subagents, and skills unless the Restless launch contract explicitly admits them.
- Preserve project-local instructions only through the existing project-instruction hierarchy and prove they cannot replace higher Restless authority.
- Implement the T0-approved scoped provider-auth route. Ensure raw provider and subscription credentials are absent from process environment, argv, files, logs, and inspection surfaces.
- Add startup/readiness checks before the first prompt and actionable failure reasons for missing binary, bad integrity, incompatible protocol, unavailable auth, or incompatible model.
- Keep the plain interactive `claude` TUI and user-installed binaries outside the supported profile.

## Acceptance

- A clean company image reports exact adapter and bundled agent versions and passes integrity verification.
- The first prompt cannot be sent until ACP initialization, authentication, and required capability checks succeed.
- Launch evidence proves the exact system/actor context, cwd, model/effort, approved tools, and MCP set.
- Host and project fixtures containing hostile ambient Claude settings cannot augment the session.
- Runtime inspection finds no Anthropic root key, reusable subscription token, inherited host config, or unapproved MCP endpoint.
- An incompatible model/harness combination fails before provider traffic.
- Removing the pinned adapter or scoped auth route produces a clear not-ready state rather than fallback to an ambient installation.

## Makes Deletable

- Local developer Claude installations as an implicit dependency.
- Raw API-key environment experiments.
- Permissive adapter launch scripts.
- `latest` version resolution at build or runtime.
