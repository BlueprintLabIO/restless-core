# Restless capabilities: implementation and evidence

This is the source guide behind the [README](../README.md), reviewed on 22 September
2026. It connects product language to Core implementation and recorded runs. The
[architecture](../ARCHITECTURE.md) defines ownership;
[Company Truth](company-identity/restless-company-truth.md) records the approved product
convictions. Specifications describe the wider destination as well as current mechanisms.

## What the product is trying to achieve

Restless gives a continuing company responsibility for useful outcomes. Its optimisation
target is useful economic output relative to owner attention, time, cost and bounded risk.
The human supplies direction, taste, collaboration and consequential decisions. OrgIntel
holds continuing responsibility while replaceable model sessions perform the work.

The four modes—explore, execute, repair and evolve—are a model of company behaviour,
not a compulsory sequence of workflow stages. Likewise, the coordination theory guides
staffing judgement; it is not an automatic formula for choosing team size.

Read [OrgIntel](specs/orgintel.md), [coordination theory](COORDINATION_THEORY.md),
[the owner experience](specs/owner-cockpit.md) and
[evaluation and dogfood](specs/evaluation-dogfood.md) together. They explain why fewer
interruptions, better evidence and a usable result matter more than apparent activity.

## Capability map

“Core” includes the daemon, CLI and organisational APIs. A Core capability does not
necessarily have a dedicated settings page. The README's screenshots show the current
web surfaces; this map also includes the deeper mechanisms used by agents and operators.

| Capability | Implementation and interface | Evidence or design reference |
| --- | --- | --- |
| Human and agent identities, membership and rooms | [Access](../crates/restless-orgintel/src/access.rs), [actors](../crates/restless-orgintel/src/actors.rs), [rooms](../crates/restless-orgintel/src/rooms.rs) and the People workspace | [Collaboration decision](adr/0008-activate-bounded-company-collaboration.md); [provider-neutral access](adr/0009-provider-neutral-company-access-context.md) |
| Native collaborative documents, versions and comments | [Document state](../crates/restless-orgintel/src/documents.rs), [web editor](../web/src/lib/components/DocumentEditor.svelte), [collaboration sidecar](../services/native-documents-collaboration/) | [Native Docs architecture](adr/0010-native-docs-are-a-core-subsystem.md); [current product captures](media/README.md) |
| Attention and decision continuation | [Attention projection](../crates/restlessd/src/attention.rs) and [owner handoffs](../crates/restless-orgintel/src/review.rs) | [Outcome review tests](../crates/restless-orgintel/tests/outcome_review.rs); [owner cockpit specification](specs/owner-cockpit.md) |
| Exact action, prepared state and resume condition | `request_owner_handoff` validates all three; handoffs retain the accountable work and running attempt | [Review implementation](../crates/restless-orgintel/src/review.rs) |
| Durable goals, work, attempts and dependencies | [Goals and Work](../crates/restless-orgintel/src/goals_work.rs), [attempts](../crates/restless-orgintel/src/attempts.rs), [artifacts](../crates/restless-orgintel/src/artifacts.rs) | [Exact execution tests](../crates/restless-orgintel/tests/exact_execution.rs) |
| Accountable supervision with material-event wakes | [Execution substrate](../crates/restless-orgintel/src/substrate.rs), [staff runtime](../crates/restlessd/src/staff.rs) | [Sprint 26 run](sprints/sprint-26/run-report.md); [Sprint 30](sprints/sprint-30.md) |
| One-shot and recurring weekday schedules | [Schedule storage and occurrence recovery](../crates/restless-orgintel/src/schedules.rs), [daemon delivery](../crates/restlessd/src/schedule.rs), CLI | [Schedule appliance tests](../crates/restless-orgintel/tests/schedule_appliance.rs) |
| Context checkpoints, provenance and freshness | [Actor context](../crates/restless-orgintel/src/actor_context.rs), [staff context assembly](../crates/restlessd/src/staff/context.rs) | [Context tests](../crates/restless-orgintel/tests/actor_context.rs) |
| Outcome standards | [Typed standards](../crates/restless-orgintel/src/types.rs), [context/configuration](../crates/restlessd/src/context.rs), Authority & limits and per-Work configuration | [Four-standard comparison](dogfood/outcome-standard-tier-comparison.md) |
| Native review and exact candidate custody | [Review](../crates/restless-orgintel/src/review.rs), [immutable review targets](../crates/restless-orgintel/src/substrate.rs) | [Sprint 26 exact candidate, gate and promotion evidence](sprints/sprint-26/run-report.md); [Sprint 30 reviewer access](sprints/sprint-30.md) |
| Company Identity releases and Work binding | [Identity](../crates/restless-orgintel/src/identity.rs), [constitution compiler](../crates/restless-orgintel/src/constitution.rs), `restless identity` and related CLI operations | [Company Identity programme](sprints/company-identity-programme.md); [identity tests](../crates/restless-orgintel/tests/company_identity.rs) |
| Voice, visual language and culture | [Voice](../crates/restless-orgintel/src/voice.rs), [visual](../crates/restless-orgintel/src/visual.rs), [culture](../crates/restless-orgintel/src/culture.rs) contracts and evidence | [Two-company constitution run](dogfood/company-identity/s35-run-report.md) |
| Evidence-bound artifacts, drift and owner-approved learning | `bind_constitution_artifact`, `propose_constitution_learning`, `compute_identity_drift`, `decide_identity_migration` in [constitution](../crates/restless-orgintel/src/constitution.rs) | [Constitution tests](../crates/restless-orgintel/tests/company_constitution.rs), [held-out corpus](../crates/restless-orgintel/tests/constitution_corpus.rs), [recorded run](dogfood/company-identity/s35-run-report.md) |
| Failure classification and recovery context | [Health preflight](../crates/restlessd/src/health.rs), [attempt recovery](../crates/restlessd/src/staff/recovery.rs), [staff context](../crates/restlessd/src/staff/context.rs) | [Continuous product-work experiment](../experiment/coordination/experiments/EXP-10/RESULTS.md); [outcome-standard run](dogfood/outcome-standard-tier-comparison.md) |
| Persistent computer, browser and ordinary company tools | [Runtime](../crates/restlessd/src/runtime.rs), [company image](../infra/company-image/), embedded Company Computer | [Runtime specification](specs/company-runtime.md); [product captures](media/README.md) |
| Native harnesses and per-agent model selection | [Native harness integration](../crates/restlessd/src/native_harness.rs), [model gateway](../crates/restless-model-gateway/), Company → Intelligence and People | [Provider and agent screenshots](../README.md#choose-intelligence-per-agent) |
| Authority, grants and spend admission | [Authority](../crates/restlessd/src/authority.rs), [invocation admission](../crates/restless-orgintel/src/invocations.rs), [model gateway](../crates/restless-model-gateway/) | [Authority specification](specs/authority-plane.md); [admission tests](../crates/restless-orgintel/tests/model_invocation_admission.rs) |
| Infisical credentials and governed effects | [Effect runner](../crates/restlessd/src/effect.rs), authority-owned credential adapters and Vault | [Authority credential and receipt contract](specs/authority-plane.md) |
| Separate companies and recoverable local operation | [Runtime](../crates/restlessd/src/runtime.rs), company configuration and persistent volumes | [Local appliance results](dogfood/sprint-38/RESULTS.md); [cell architecture](CELL_ARCHITECTURE.md) |

## What makes these mechanisms useful

### Supervision without routine narration

Sprint 26's deterministic fixture delivered 100 progress artifacts with zero lead model
wakes. Material terminal facts coalesced into one wake, and a genuine blocker triggered
one promptly. Sprint 30 adds the bounded route in which an exact commission and passing
mechanical evidence settle without a paid supervisory paraphrase. These are specific
execution results, not a universal cost advantage over other harnesses.

The underlying [EXP-17 comparison](../experiment/coordination/experiments/EXP-17/RESULTS.md)
found that a serial supervisory chain added cost and latency without improving quality
in its four paired tasks. Restless retains lead responsibility and removes routine speech
from the critical path. Specialists and independent reviewers remain useful when the work
benefits from them.

### Company identity that can be corrected

The constitution compiler creates a bounded brief from an immutable release. Work and
artifacts retain their bindings to that release and its evidence. Conflicting approved
facts block dependent expression; missing pillars are explicit. Voice preserves audience,
channel and author distinctions, while culture records conduct and decisions.

The Sprint 35 run exercised Restless and a separate business, Harbour Ledger. Its fact
correction identified the affected artifact; an owner could decide its migration while
staff could not. A proposed learning needs distinct before/after artifacts and attributed
evidence. Generated repetition cannot approve itself into company policy.

This is implemented organisational machinery, accessible through Core and the CLI. It is
more specific than storing a brand document in an agent's knowledge base.

### Recovery that preserves the company

Work responsibility and attempt history live outside an individual model process. Health
checks inspect disk, containers and provider behaviour. Recovery can rebuild a focused
brief from durable work and observed failure instead of assuming a clean session means a
clean slate. Schedules track occurrences and missed-work decisions.

The effect runner separately checks intent identity, refuses a reused key for a different
command, and stops a retry when an earlier execution has an unknown outcome. Restoring
runtime files does not erase the external action record.

## Product direction and current boundaries

The following belong in the architecture and roadmap, rather than being added as present-day
checkboxes in the feature matrix:

| Direction | Current evidence boundary |
| --- | --- |
| A company that explores markets and continually improves its own operation | Explore/execute/repair/evolve is the OrgIntel design. Durable responsibility, schedules, context, review and identity learning implement parts of it; success still depends on the actual business and evidence. |
| Large elastic teams for independent sales, support or recruiting units | The coordination theory distinguishes queue capacity from coauthoring one result. It is staffing guidance, not a claim of a proved automatic autoscaler. |
| Public service publication | [Sprint 36](sprints/sprint-36/RESULTS.md) proves Core contracts and local transport, with additional Cloud evidence and remaining public-product gates. The README does not advertise turnkey public hosting. |
| An installed appliance on every platform | [Sprint 38](dogfood/sprint-38/RESULTS.md) records live macOS work and outstanding elapsed-use qualification. The README retains the development-checkout setup. |
| Taking over the exact native Codex or Claude terminal session | The [Open Company Runtime programme](sprints/README.md) is separate from current native-harness integration and embedded desktop access. |
| Hosted fleet operation | Cloud has a [separate repository](https://github.com/BlueprintLabIO/restless-cloud). Self-hostable Core does not imply managed hosting is included. |

## Comparison notes and sources

The README compares the product surface and built-in mechanisms, not benchmark quality.
**✓** means a documented built-in mechanism exists. **◐** credits a related mechanism
or an integration/configuration route; it does not claim identical behaviour. **—** marks
a feature the product does not provide natively. **?** means
an equivalent was not established from the sources reviewed, not that it is impossible.
A workflow assembled by the user is distinguished from a product-owned company mechanism.

### Paperclip

[Issues and documents](https://docs.paperclip.ing/guides/day-to-day/issues/),
[artifacts](https://docs.paperclip.ing/guides/day-to-day/artifacts/),
[decisions](https://docs.paperclip.ing/guides/day-to-day/decisions/),
[membership](https://docs.paperclip.ing/guides/org/members-and-access/),
[routines](https://docs.paperclip.ing/guides/projects-workflow/routines/),
[watchdogs](https://docs.paperclip.ing/guides/projects-workflow/task-watchdogs/),
[skills](https://docs.paperclip.ing/guides/org/skills/) and
[budgets](https://docs.paperclip.ing/guides/day-to-day/costs/) establish substantial overlap.

Paperclip already supports human membership, goal-directed delegation, a ranked decision
queue, document revisions and artifact viewing. Its agents can create issues autonomously.
Its skills and task briefs are the related mechanisms credited under identity and quality;
they do not establish Restless's four-pillar release-and-drift contract. The README's
positioning is a difference in working emphasis, not an assertion that Paperclip is only
for coding or requires manual ticket dispatch.

### OpenClaw

[Multi-agent routing](https://docs.openclaw.ai/concepts/multi-agent),
[sessions](https://docs.openclaw.ai/session), [cron](https://docs.openclaw.ai/cli/cron),
[memory consolidation](https://docs.openclaw.ai/concepts/dreaming),
[control panels](https://docs.openclaw.ai/web/control-ui/panels) and the
[repository](https://github.com/openclaw/openclaw) support its assistant, persistence,
automation and execution features.

Messaging groups and agent routing are not the same company-membership model. File,
browser and desktop facilities depend on the configured runtime capabilities. Its
memory and workspace instructions are substantive related capabilities; the comparison
credits them without treating them as governed company-identity releases.

### Lindy

[Home](https://docs.lindy.ai/teammate/home), [files](https://docs.lindy.ai/teammate/files),
[routines](https://docs.lindy.ai/teammate/routines), [skills](https://docs.lindy.ai/teammate/skills),
[artifacts](https://docs.lindy.ai/teammate/artifacts),
[credentials](https://docs.lindy.ai/integrations/credentials),
[usage](https://docs.lindy.ai/account-billing/usage) and the
[workflow guide](https://www.lindy.ai/blog/human-in-the-loop-automation) document the credited features.

Team files have version history and owner/admin editing. Team skills are reusable
instructions; their documentation describes a gradual rollout. Artifacts include hosted
interactive pages. Personal routines can become scripts that finish quietly, wake Lindy
for judgement or trigger repair after failure. These deserve credit alongside Restless's
approach to reducing unnecessary model calls.

Usage is measured in pooled credits with per-member caps. That is the related mechanism
under model-spend budgets. Model choices in the hosted product do not establish native
Codex/Claude runtime support or arbitrary bring-your-own-provider configuration.

### n8n

The [repository](https://github.com/n8n-io/n8n),
[Tools Agent](https://docs.n8n.io/integrations/builtin/cluster-nodes/root-nodes/n8n-nodes-langchain.agent/tools-agent),
[human review](https://docs.n8n.io/build/integrate-ai/ai-examples/human-in-the-loop-for-tools),
[execution retries](https://docs.n8n.io/workflows/executions/all-executions/) and
[workflow sharing](https://docs.n8n.io/workflows/sharing/) establish its automation model.

n8n can delegate to agents and workflows, attach memory, choose models and pause tool
calls for human approval. Its retry history is relevant to execution provenance; that
does not itself supply immutable business-artifact acceptance. Documents, desktop tools,
company identity and outcome quality policies can be connected or composed around the
workflow. Team features depend on the edition. “Self-hostable” is not a licence-equivalence claim.

### Dify

The [repository](https://github.com/langgenius/dify),
[Human Input node](https://docs.dify.ai/en/cloud/use-dify/nodes/human-input),
[workflow triggers](https://docs.dify.ai/en/cloud/use-dify/build/workflow-chatflow),
[LLM memory and retries](https://docs.dify.ai/en/cloud/use-dify/nodes/llm),
[model providers](https://docs.dify.ai/en/cloud/use-dify/workspace/model-providers) and
[workspace members](https://docs.dify.ai/en/cloud/use-dify/workspace/team-members-management)
describe app building, knowledge, orchestration and collaboration.

Human Input can display a draft, accept direct edits, collect feedback and route a
revision. That is credited as native review feedback, while live shared rich-text
coauthoring remains a different feature. App knowledge and prompts can hold company
facts and style; workflow versions and execution records provide related lineage.
These are not evidence of a four-pillar identity release automatically bound to an
exact business artifact. Self-hosting and hosted-plan features have different conditions.

## Updating this guide

When adding a claim, link its implementation and the strongest available recorded run.
When updating a competitor cell, prefer that product's primary documentation or code and
record the relevant scope. Move a roadmap capability into the feature list when its
mechanism and usable path exist. Keep unsuccessful experiments and outstanding acceptance
gates attached to their original reports.
