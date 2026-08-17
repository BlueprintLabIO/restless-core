# Skill standards and registries

## Finding

The emerging ecosystem has a useful common package format, but no registry currently provides a
trustworthy answer to "which skill will make this actor good at this Work?" Restless can reuse the
format without adopting the marketplace's quality assumptions.

## First-principles model

A useful skill has four jobs:

1. route correctly from a task description;
2. provide a short, executable method for a recurring job;
3. disclose detailed references, assets, or scripts only when needed; and
4. make its assumptions and expected evidence visible enough to test.

The skill is successful only if it improves an accepted outcome at reasonable cost. Packaging,
downloads, stars, model compliance, and a clean security scan are intermediate signals.

## Current standards and discovery surfaces

### Open Agent Skills specification

The [Agent Skills specification](https://agentskills.io/specification) defines a directory containing
a required `SKILL.md` and optional `scripts/`, `references/`, and `assets/`. It standardises routing
metadata and progressive disclosure. It recommends keeping the main instructions small and loading
supporting resources only when needed.

What it legitimately gives Restless:

- a simple, filesystem-native interchange format;
- human-readable instructions that work with the persistent Company Runtime;
- progressive context loading rather than one giant actor prompt; and
- portable packages that can be versioned in Git.

What it does not give Restless:

- a behavioural quality standard;
- secure code or dependencies;
- evidence that a model follows the instructions;
- compatibility across every host; or
- authority to use tools. The specification marks `allowed-tools` experimental, and support varies.

Restless must not let a skill's `allowed-tools` metadata override Kernel authority. A skill may state
what it expects; the Kernel still decides what is allowed.

### OpenAI / Codex skills

[OpenAI's current skill documentation](https://learn.chatgpt.com/docs/build-skills) adopts the same
open format. It reinforces two sound ideas:

- only skill names and descriptions are present during initial routing; full instructions and
  resources load later; and
- descriptions need explicit triggers and boundaries because they are the routing surface.

This is evidence about how Codex hosts skills, not proof that any listed skill is effective. It is
also not a reason to make Restless dependent on an OpenAI model. The portable lesson is progressive
disclosure and clear routing metadata; the runtime implementation should remain provider-neutral.

### Anthropic's public skills repository

The [Anthropic skills repository](https://github.com/anthropics/skills) is a useful collection of
worked examples from a model vendor. It offers provenance and patterns worth inspecting. Stars and
adoption show interest, not cross-model reliability or better business outcomes.

### skills.sh

[skills.sh](https://skills.sh/) is presently the broadest visible discovery surface for Agent Skills.
Its [documentation](https://www.skills.sh/docs) is unusually clear about its limits:

- ranking is based on anonymous install telemetry;
- the platform cannot guarantee the quality or security of every skill; and
- users are expected to review skills before installation.

The [official collection](https://www.skills.sh/official) is useful for publisher provenance. It does
not prove that a skill fits Restless, works with the chosen provider, or improves an outcome.

The [security-audit view](https://www.skills.sh/audits) is useful as one input, but scanners can
disagree. At the time of this review, one Microsoft skill was simultaneously shown as safe by one
provider, zero-alert by another, and critical risk by a third. "Audited" therefore means multiple
tools produced opinions, not that the package is safe.

### MCP Registry

The [official MCP Registry](https://registry.modelcontextprotocol.io/docs) discovers MCP servers.
Those servers expose tools and data. They are not procedural skills and do not establish actor
expertise.

This distinction matters:

- a design skill may teach an actor how to perform visual review;
- a browser or screenshot MCP server may let it observe the page;
- the actor's prior accepted designs may support an expertise claim; and
- the lead decides whether that actor belongs on the team.

One registry cannot correctly substitute for the other three decisions.

## Registry comparison

| Surface | Discovers | Strong signal | Weak or absent signal | Restless use |
|---|---|---|---|---|
| Agent Skills spec | Package shape | Format compatibility | Quality, trust, fit | Adopt as file convention |
| Vendor skill repos | Skills/examples | Publisher provenance | Cross-provider performance | Inspect for candidates |
| skills.sh | Public skills | Popularity and discoverability | Outcome quality and full security | Untrusted lead list |
| MCP Registry | Tool/data servers | Protocol metadata and package location | Procedural expertise | Discover runtime capabilities |
| OrgIntel evidence | Actors and completed work | Local relevance and accepted outcomes | Broad ecosystem coverage | Select team and inform future attempts |

## What a Restless catalogue should be

For now, not a service and not a global marketplace. A checked-in or company-local index file is
enough. Each candidate entry can record:

- source repository and publisher;
- pinned commit or content digest;
- license and runtime requirements;
- instruction-only versus executable content;
- expected tools and external network use;
- which Work outcome it claims to improve;
- test-company evidence; and
- current disposition: candidate, accepted, rejected, or superseded.

The skill package remains a normal Runtime file. OrgIntel may refer to it from an actor profile or an
Attempt when a real scenario proves that useful. It should not copy the package into governed state.

## Rejected shortcuts

- **Install the leaderboard leaders.** This selects distribution success, not task fit.
- **Trust an official badge.** It answers who published the package, not whether it works.
- **Trust a scanner majority.** Static and semantic scanners cover different risks and can all miss
  harmful runtime behaviour.
- **Use every relevant skill at once.** Extra routing metadata consumes context and overlapping
  instructions conflict.
- **Turn every practice into a skill.** Stable company doctrine may belong in `AGENTS.md`; project
  facts belong in project files; one-off instructions belong in the Work brief.
- **Build a capability ontology first.** The repeated Work and outcome evidence needed to justify it
  does not yet exist.

## Sources and confidence

- [Agent Skills specification](https://agentskills.io/specification): high confidence for current
  format, no claim about effectiveness.
- [OpenAI: Build skills](https://learn.chatgpt.com/docs/build-skills): high confidence for current
  Codex behaviour, vendor-specific.
- [Anthropic skills repository](https://github.com/anthropics/skills): high provenance, examples not
  evaluation evidence.
- [skills.sh documentation](https://www.skills.sh/docs) and
  [audits](https://www.skills.sh/audits): high confidence for marketplace mechanics, low confidence
  that marketplace signals predict outcomes.
- [Official MCP Registry](https://registry.modelcontextprotocol.io/docs): high confidence for current
  server-discovery scope.

