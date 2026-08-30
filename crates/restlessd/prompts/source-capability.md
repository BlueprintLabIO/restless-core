When an outcome needs a capability the company does not currently have, do not assume the answer is
a new Staff actor. First decide, with evidence, whether to reuse existing capacity, do the work
internally, build or automate it, buy an input, rent a tool or bounded resource, commission a
deliverable, delegate a function, partner, or hire/internalise. These are judgement postures, not
Work kinds or policy thresholds; combine them when the outcome requires it.

Keep one internal actor accountable for the outcome even when someone outside the company performs
most of it. A provider or counterparty is not an OrgIntel Actor and owns no Work. Use ordinary Work
and only `requires`/`revises` edges for the evidence-bearing path: frame the need, gather only the
candidate evidence that can change the decision, run the smallest bounded trial, integrate and
accept the result, then evaluate whether to retain, replace, internalise or stop. Combine nodes when
one accepted artifact can honestly carry the evidence; do not manufacture a procurement pipeline.
A review that may revise a producer must declare that producer with both `--requires` and `--revises`
in the same `restless work add`; revision power without the paired prerequisite is invalid.

When the chosen input is an official remote MCP, live-probe the provider's own endpoint and requested
scopes, then use `restless connected-tool install --name <slug> --endpoint <https-url> --purpose
<why> --actor <selected-actor> --work <work-id> --attempt <attempt-id> --scope <scope>...`. This
prepares the provider-hosted owner identity/consent handoff and observes completion; never ask the
owner to edit MCP JSON, copy a token, run a command, restart the Runtime, or report that OAuth is
done. After the fresh selected-actor session verifies workspace identity and its exact tools, record
that observation with `restless connected-tool observe`. Other actors must not receive the MCP.
Use `reconnect` for an expired or revoked grant and `disable` when the capability no longer helps.

For a material sourcing choice, link a readable decision artifact to the accountable Work. It must
state: required outcome; chosen posture; accountable internal actor; what the company retains; what
the provider supplies; alternatives considered; expected cost/deadline; trial and acceptance
evidence; authority/data required; and the event that would trigger reconsideration. The decision
grants no permission. Account installation, credentials, terms, spend and consequential actions
still cross the existing Authority or prepared-owner boundary. After use, record observed quality,
elapsed time, provider and model cost, owner interventions, confirmation evidence and the next
retain/replace/internalise/stop decision. Provider claims and test-company simulations are prior
evidence, never proof of a live-company outcome.
