# Sprint 08 — The company knows who it is, can source a missing capability and can prepare one real payment safely

**Status:** In progress. T8's durable-identity contract is complete. Local implementation,
adversarial verification and the six-node `_test` sourcing run are complete for the wider slice;
the remaining checklist stays open because T0 and the real-provider exit require explicit owner
access and authority, while T9/T10 still require the unavailable visual browser review. No bank
account creation, provider credential import, funding or live payment is authorised by this
document. Ticket status lives only in the checklist below.
**Date:** 19 August 2026
**Spec refs:** `authority-plane` §1 / §2.2 / §2.2.1 / §2.6 / §6 / §7 / §8 / §9 / §12 / §13,
`cross-layer-contract` §2 / §3 / §5 / §6 / §8, `orgintel` §3.4 / §5 / §6.3,
`owner-cockpit` §5 / §6 / §7 / §10.3 / §12.3 / §12.4,
`ARCHITECTURE.md` §2.1 / §3 / §6.1 / §9.2 / §9.6 / §16.1 / §16.2 / §16.8,
Sprint 05 T6 and Sprint 07

---

## Observed product and implementation gap

Restless can spend model budget and execute generic governed external commands, but it cannot yet
truthfully answer the first questions a functioning company asks about money:

- What is the legal entity behind this operating company or trading name?
- Which details are owner-asserted, which were observed from an external registry, and when?
- What cash is available, pending or unknown at the provider?
- Which account may the company observe, prepare a payment from or actually debit?
- What is the maximum loss if the Runtime or an agent is compromised?
- Did a payment merely get submitted, enter approval, become scheduled or actually settle?

The gap is concrete in the current build:

- `CompanyConfig` contains a runtime name, mission, model configuration, model-spend fuse and
  credential references. It has no legal entity, registration, safe invoice profile or restricted
  identity references.
- Authority has real effect intents, first-party approval records, idempotency and reconciliation,
  but no direct-money envelope, per-payment limit, aggregate reservation or account access mode.
- The generic effect runner gives an actor-selected Runtime executable named secret bindings and
  treats the child exit/result as its receipt. That is the right general boundary for ordinary
  consequential tools; it is not provider confirmation and should not receive a live bank-write key.
- Sprint 05 proved an imported Infisical backend, short-lived machine authentication, exact-value
  leakage checks and graceful outage. Finance has not yet exercised that boundary, and no reason has
  been demonstrated to add a second secret backend.
- Sprint 07 can present an exact owner ask and observable resume condition, but no real payment or KYB
  handoff currently exists for it to present.
- The provider branch below exists only in founder-authored sprint prose. The live company has no
  explicit Work that frames the missing operating-money capability, researches the build/buy/
  delegate boundary, selects an external posture, runs a bounded provider probe and evaluates whether
  to retain or replace the result. The company therefore cannot yet demonstrate external capability
  acquisition as an organisational behaviour.
- Sprint 06 made teams durable, but the current People surface still groups the Exec with `world` and
  `daemon` under `Not people`, filters those rows through a Svelte-owned actor-id list, presents team
  members as chat contacts before refusing the conversation, and renders an uninformative `READY` on
  nearly every row. Live actor ids still encode `staff`, `lead`, `live` and Work/revision history even
  though those facts already belong to actor kind, team membership and Work.
- The current Work implementation has an unverified SvelteFlow/Dagre graph candidate and a separate
  board projection, but no sprint contract proves that either view remains legible on
  real `requires`/`revises` topologies, that both views derive status and progress from the same
  OrgIntel projection, or that a displayed live/current step is observed rather than inferred. The
  layout-engine experiment also has no purge decision yet.

The current Authority spec explicitly excludes full banking and unrestricted production payment
administration. This sprint does not silently erase that exclusion. Its financial slice tests one
narrower claim:

> A Restless company can know the legal identity it is operating under, observe real account state,
> prepare one bounded operating payment through a host-side provider adapter, obtain the owner's
> approval in the provider's own authenticated experience, and reconcile the provider-confirmed
> result—without exposing treasury write access, provider credentials or personal KYB evidence to
> the Company Runtime.

## Outcome

> **One selected Restless company has an owner-confirmed legal profile and a deliberately bounded
> operating-money connection. The Exec can see current provider-backed cash, prepare one payment to
> an owner-established beneficiary, bring the exact provider-native approval to the owner, observe
> the external result and continue from a confirmed receipt. The protected treasury account has no
> Restless write credential. Raw money credentials, owner identity documents and private/personal tax identifiers
> never enter the Runtime, model context, Git, OrgIntel, browser JSON, logs or receipts. The company
> itself frames that external need, researches candidate sourcing postures, records why it is renting
> rather than building the capability, runs the provider trial as ordinary Work and evaluates the
> observed result. Its People surface presents one durable Exec, accountable team contacts and their
> staff without exposing message-provenance actors or assignment-shaped identities as colleagues.
> The Work surface makes those same company outcomes legible through one calm dependency graph and
> one denser board projection, with honest progress, evidence and observed liveness leading to the
> existing Work detail rather than a second workflow or activity system.**

The first provider candidate is **Airwallex** for an Australian operating company. It currently
documents separate read/write resource scopes, separate Beneficiary and Transfer scopes, API-created
transfers routed through an account approval workflow, a sandbox, short-lived API access tokens and
transfer webhooks. These are candidate capabilities, not evidence that the owner's actual account is
eligible or configured. T0 live-probes the real surfaces before provider code becomes canon.

**Candidate evidence checked 20 August 2026:** Airwallex documents
[scoped API keys](https://www.airwallex.com/docs/developer-tools/api/api-key-scopes),
[API-initiated transfer approvals](https://www.airwallex.com/docs/payouts/transfers/manage-transfers/manage-approvals-for-transfers),
[30-minute access tokens](https://www.airwallex.com/docs/api/authentication/api_access_token) and
[signed, retried webhooks](https://www.airwallex.com/docs/developer-tools/webhooks/webhooks-overview).
Its Business Account sandbox uses `https://api.sandbox.airwallex.com`, while the separate Connected
Accounts product documents `api-demo`; the adapter must not conflate those contracts. Airwallex also
versions Business Account requests by account/date, so T0 records the exact version observed for the
selected account rather than inheriting a source-code `latest` constant.
Its Australian safeguarding explanation states that it is not an ADI and its wallet is not directly
covered by the FCS; [APRA's FCS guidance](https://www.apra.gov.au/financial-claims-scheme-banks-building-societies-and-credit-unions)
describes the separate protection for eligible deposits at Australian ADIs. These links support the
candidate choice only; current provider terms, the product disclosure statement and the live account
probe govern implementation.

The account architecture is deliberately asymmetric:

```text
existing Australian ADI treasury
  └── Restless: read-only or no connection

bounded Airwallex operating wallet
  ├── read connection: balances, financial transactions, transfer status
  ├── submit connection: transfer read/write only
  ├── beneficiary creation/change: owner in provider UI
  ├── API-created transfer: provider approval required
  └── maximum funded balance: owner-chosen accepted loss
```

Airwallex is a financial platform rather than an Australian ADI; its documented safeguarding model
is not direct Financial Claims Scheme deposit coverage. The operating wallet therefore cannot become
the company's unbounded reserve merely because the API is convenient.

## Success contract

The sprint passes only when one observed end-to-end run demonstrates all of the following:

1. **One usable legal identity.** The selected company has one owner-confirmed legal profile naming
   its legal name, trading name where relevant, entity type, jurisdiction and registration
   identifier. The product distinguishes owner assertion from live registry observation and records
   when the observation occurred. It does not generalise a multi-entity corporate graph.
2. **A safe projection, not a PII dump.** Agents can use only the fields intentionally approved for
   invoices, contracts and provider requests. Tax identifiers, residential details, beneficial-owner
   evidence, passports, licences and provider authentication material are absent from the shared
   Runtime and normal model context. KYB documents stay with the provider wherever possible.
3. **Provider state is external truth.** Balance, transfer and onboarding claims carry provider
   references and observation time. Unavailable or stale state is `unknown`/`unavailable`, never
   silently zero, absent or settled.
4. **Two materially different credentials.** Read access and payment submission use separate scoped
   provider credentials. No admin key exists. The submission credential cannot create, update or
   delete beneficiaries and cannot administer the provider account.
5. **No finance secret in the Runtime.** The long-lived provider key and webhook verification secret
   remain in Infisical. Short-lived provider access tokens exist only in host-side adapter memory.
   The generic Runtime effect-child path cannot request the finance binding.
6. **Bounded deterministic authority.** Before submission, Authority checks the exact source account,
   provider beneficiary reference, amount in minor units, currency, per-payment ceiling, aggregate
   reserved/settled amount, freeze state and idempotency key. No model decides permission. Pending and
   unknown payments continue to reserve budget.
7. **Provider-native approval.** The submitted transfer enters the provider's actual approval state.
   Restless cannot approve it with the same credential or through conversation. The owner sees the
   exact beneficiary, amount, purpose, source evidence, no-action consequence and provider-native
   action.
8. **Confirmation is not attestation.** Submission, approval, scheduling and settlement remain
   distinct. A process exit or self-reported JSON cannot become confirmed money movement. Only the
   provider's authenticated API/webhook state can produce a provider-confirmed receipt.
9. **Ambiguity does not duplicate money.** A lost response, daemon restart or duplicate webhook leaves
   the payment safely replayed or explicitly unknown. The same idempotency key cannot create another
   transfer, and unknown outcome must reconcile before a new execution.
10. **Freeze and revoke are local.** Freezing financial effects or revoking the submit key stops new
    payments while cash observation, files, Git, OrgIntel, the cockpit and unrelated work continue.
11. **The owner performs only the irreducible step.** If onboarding needs identity, legal attestation,
    MFA or a document upload, Restless brings the exact provider link/status and observes the provider
    result. It does not ask the owner to repeat company research, manually transcribe a prepared
    payment or report completion that the provider can expose.
12. **One real economic effect.** After fake-provider adversarial checks and the real Airwallex
    sandbox pass, one explicitly owner-approved, low-value live transfer between owner-controlled or
    otherwise genuinely owed accounts reaches a provider-confirmed terminal result. A sandbox-only
    run, a fabricated live-company receipt or an unfunded payment draft does not close the sprint.
13. **The owner can follow what approval caused.** Before an owner action, the surface says whether
    it completes the current Work or unlocks a named next action. After approval, the item does not
    simply disappear or return the owner to unrelated context: it becomes a compact continuation
    showing the recorded decision, what it unlocked, the responsible actor, current Work/Attempt or
    provider state, and the observed outcome or blocker. If nothing follows, it says explicitly that
    the decision completed the Work and no further action is scheduled. This is a projection of
    existing Work edges, Attempts, messages, Authority intents/receipts and provider observations—not
    a new workflow engine, inferred progress or model narration.
14. **External capability sourcing is explicit but ordinary Work.** The operating-money gap is framed
    as an outcome owned by an internal actor. Candidate research, the sourcing decision, bounded
    provider probe, acquisition/integration and evaluation are visible through normal Work, Attempts,
    decisions and artifact references. The decision says what the company retains, what the provider
    supplies, the expected evidence and when to reconsider. There is no sourcing-specific Work kind,
    status, edge, universal provider interface, marketplace or capability registry, and the external
    provider is not made an OrgIntel Actor.
15. **One durable staff identity is not one assignment.** New Staff actor ids use one stable
    `{domain}-{craft}` identity without `staff-`, team position, `live`, revision, stage, retry or
    implementation suffixes. A distinct human-readable display identity is the primary People label;
    role and team relation remain separate facts. Creation rejects a non-conforming or already-used
    domain/craft identity and makes reuse of the existing actor the obvious path. Existing historical
    ids are not silently renamed or merged.
16. **People shows the organisation rather than schema plumbing.** `world` and `daemon` remain as
    provenance-preserving actor rows classified as `system`, but People excludes actors by kind rather
    than a UI id list. The Exec has a distinct first position. Exec and accountable team leads retain
    contact affordances; members are denser inspectable rows whose detail leads with current Work and
    the route to their lead. Role subtitles and universal `READY` labels do not consume permanent row
    space. The compact focus summary shows the Work title/current focus, not a long execution
    instruction mistaken for an owner-facing outcome.
17. **Work has one truth and two useful lenses.** The Work tab presents goals and current Work through
    a readable dependency map whose `requires` and `revises` directions remain distinct, plus a
    compact Kanban projection for scanning next, in-motion, waiting and recently landed outcomes.
    Both lenses use the same repeatable-read OrgIntel Work/Attempt/edge/artifact/gate projection and
    open the same Work detail. Progress is expressed through source-owned revision, Attempt, gate and
    evidence state rather than invented percentages. Current activity appears only when backed by an
    observed session/process signal with observation time and explicit stale/unknown treatment. A
    representative topology comparison chooses one lightweight layout engine and deletes the losing
    candidate; the board does not become a second state machine or add drag-to-mutate semantics.

If the company is ineligible for the candidate provider or transfer approvals are unavailable on the
actual account, record that provider fact and update the sprint before implementing a replacement.
Do not claim the outcome with a provider-shaped mock, browser screenshot or documentation quote.

## Classification and authority boundary

This slice contains both judgement and deterministic mechanics. Keeping them separate is the core
security design.

**Model/actor judgement:** whether the bill is legitimate, whether it matches a contract, whether it
is worth paying now, what evidence matters, and what recommendation to give the owner.

**Deterministic Authority:** whether this principal may use this exact account and beneficiary,
whether amount and aggregate exposure fit the envelope, whether approval is required, whether the
intent is a duplicate, whether an unknown outcome blocks retry, and whether the provider confirmed
the result.

Possession of legal details grants no authority. Knowing the ABN, registered address or director name
does not permit an actor to attest beneficial ownership, open an account, accept provider terms, sign
a contract, add a beneficiary or move funds.

## Legal identity boundary

V0 needs one useful profile, not a company-secretary product.

The Authority Plane owns current company identity because mandate, provider onboarding and legal
representation depend on it. The smallest useful shape is expected to cover:

```text
legal_name
trading_name, if different
entity_type
jurisdiction
registration_identifier type + value
registered_office or approved business address
principal place of business, where needed
safe display/invoice fields
restricted references, not raw sensitive values
owner assertion metadata
registry observation source + observed_at
```

The exact storage shape is a current mutable profile plus governance-relevant change attribution,
not an append-only legal ledger. External registries and providers remain authoritative for their
own state.

The sprint does not store passports, driver licences, facial images, signatures or raw beneficial-
owner documents. It does not copy provider KYB payloads into OrgIntel. If a provider needs those
materials, the owner uses the provider's own protected channel through a prepared link or handoff and
Restless observes only the bounded onboarding state required to resume.

## Money-secret boundary

Sprint 05's imported Infisical deployment remains the first implementation backend. Hosting is a
replaceable deployment choice: local dogfood may use the already-proven loopback-only self-hosted
instance; a managed deployment may use Infisical Cloud or managed self-hosting. This sprint does not
add another vault or make secret hosting part of the company ontology.

Infisical's current [machine-identity documentation](https://infisical.com/docs/documentation/platform/identities/machine-identities)
retains the short-lived access-token model already proven locally in Sprint 05.

Expected finance secret layout:

```text
/companies/<company>/finance/airwallex/read/api-key
/companies/<company>/finance/airwallex/submit/api-key
/companies/<company>/finance/airwallex/webhook/signing-secret
```

Provider client/account identifiers and masked display metadata may live in Authority current state.
Raw API keys and signing secrets do not. The host-side adapter exchanges the scoped API key for the
provider's short-lived access token and retains that token only in memory.

The adapter begins as a module inside the trusted Authority service. A separate finance microservice
or HSM is not justified by the first local single-company run: provider-native scope, an approval-only
workflow and the limited operating balance bound the accepted loss. Split the process only if the
real provider requires a separate certificate/signing boundary or dogfood demonstrates that the
modular boundary cannot contain the credential.

## Provider branch, run and purge

The sprint starts with one candidate contract rather than three integrations:

1. **Airwallex — first candidate.** Live-probe sandbox access, actual account eligibility, scoped
   keys, the absence of Beneficiary write, API-created transfer approval, status lookup, webhook
   verification and revocation.
2. **Wise Business — fallback candidate only if Airwallex fails a required capability.** Verify its
   API token scope and whether API-created transfers reliably inherit team approval. Do not build it
   in parallel.
3. **Existing ADI/CDR — treasury/read candidate, not the first write path.** Use for protected cash
   observation where useful; do not wait for a general Australian payment-initiation standard.

T0 records the smallest runnable evidence for the candidate. Once one path passes, purge provider-
selection scaffolding and implement one canon. A generic provider marketplace, provider catalogue or
multi-bank abstraction is out of scope.

## External sourcing experiment

The provider branch is also Sprint 08's first explicit external-capability acquisition experiment.
The company, not only the sprint document, must perform this reasoning:

```text
missing operating-money outcome
→ frame what must remain internal and what may be external
→ research the smallest credible postures and providers
→ run one bounded live probe
→ record the sourcing decision and required authority
→ acquire and use the selected external service
→ evaluate the accepted result, cost and owner attention
→ retain, replace, internalise or stop
```

This is a judgement path. The categories guide the Exec; no price threshold, keyword rule or state
machine chooses for it. The Work graph stays explicit by giving research, decision, probe, use and
evaluation accountable owners and artifacts, while remaining flexible through the existing
`requires` and `revises` edges. An alternative becomes a Work branch only when there is a real reason
to run it; the graph does not pre-create every possible provider. Provider discovery in the browser
is ordinary Runtime work. Terms, account creation, spend and credential access cross the existing
Authority boundary when they become consequential.

The current Airwallex → Wise fallback is the real experiment. Sprint 08 does not add another external
purchase merely to populate a matrix. A `_test` scenario may prove the Work/Authority mechanics, but
only the actual provider probe and result may support the live company's sourcing decision.

## Tickets

| ✓ | Ticket | Layer | Evidence served | Depends |
|---|---|---|---|---|
| [ ] | [**S08-T0 · Live-probe the provider and freeze one runnable contract**](sprint-08/t00-provider-proof.md) | Authority + Runtime integration | Airwallex documentation looks suitable, but no actual account/sandbox run has proved the scopes and approval path Restless needs | S08-T7 |
| [ ] | [**S08-T1 · Authority-owned legal identity and safe company projection**](sprint-08/t01-legal-profile.md) | Authority + Runtime projection | Current `CompanyConfig.name` cannot supply or distinguish the legal facts required by invoices, provider KYB and external representation | — |
| [ ] | [**S08-T2 · Finance secrets terminate in a host-side adapter**](sprint-08/t02-finance-secret-boundary.md) | Authority credential plane | The generic effect child is actor-selected Runtime code and must not receive a live money credential | S08-T0 |
| [ ] | [**S08-T3 · Bounded money intent, reservation and confirmed receipt**](sprint-08/t03-money-authority.md) | Authority | Existing first-party approval and model-spend fuse do not bound an exact account, beneficiary, amount or concurrent pending exposure | S08-T0 |
| [ ] | [**S08-T4 · One Airwallex approval and reconciliation adapter**](sprint-08/t04-airwallex-adapter.md) | Authority + external provider | Restless has no provider-confirmed payment path; process success can currently be mistaken for external success | S08-T2, S08-T3 |
| [ ] | [**S08-T5 · Prepared KYB/payment last mile in the existing owner surface**](sprint-08/t05-owner-last-mile.md) | OrgIntel + Owner surface | Identity and payment participation are named handoff categories but have not been exercised against a real provider state | S08-T1, S08-T4 |
| [ ] | [**S08-T7 · Source one external capability through ordinary Work**](sprint-08/t07-external-sourcing-work.md) | OrgIntel + Runtime + Authority boundary | Provider selection lives in sprint prose rather than company Work, so Restless has not shown that an Exec can recognise, research, acquire and evaluate a missing external capability | — |
| [x] | [**S08-T8 · Give every Staff actor one durable organisational identity**](sprint-08/t08-actor-identity.md) | OrgIntel + owner projection | Live ids still encode `staff`, `lead`, `live`, stage and revision even though actor creation already claims identity is stable | — |
| [ ] | [**S08-T9 · Make People an honest contact and inspection surface**](sprint-08/t09-people-contacts.md) | Owner surface + OrgIntel projection | The current roster calls the Exec `Not people`, leaks system actors, and gives non-contacts chat affordances before refusing them | S08-T8 |
| [ ] | [**S08-T10 · Make Work legible as one graph and one board**](sprint-08/t10-work-graph-and-board.md) | Owner surface + OrgIntel projection | The graph/board candidate has no representative-topology proof, shared progress/liveness contract or layout-engine purge decision | — |
| [ ] | [**S08-T6 · Adversarial `_test`, sandbox, bounded live run and purge**](sprint-08/t06-dogfood-and-purge.md) | All touched layers | A green adapter test cannot prove that credentials stayed contained, that real money settled exactly once, or that sourcing and owner surfaces work as one company | S08-T1–T5, S08-T7–T10 |

T7 frames the external need and creates the ordinary Work that T0 executes; it does not delay T1,
T8 or T9. T0 and T1 may run in parallel once the provider-probe Work is explicit. T2 and T3 begin
only after T0 fixes the real provider contract. T4 integrates the two Authority seams. T5 uses the
resulting source states rather than fixtures. T8 settles identity before T9 presents it. T10 may run
independently but must consume the same live Work projection used by T5 and T7. T6 is the only ticket
that can close the sprint.

## Slice per layer

**Authority Plane.** Own the legal profile, finance connection metadata, scoped credential
references, exact money consequence, hard envelope, pending reservation, freeze/revoke, provider
adapter and confirmed receipt. Reuse the existing Authority database and effect idempotency concepts.
Do not route finance through the mutable Runtime executable path.

**OrgIntel.** Own the missing-outcome framing, sourcing judgement, candidate research Work, internal
accountability, invoice/contract review, recommendation, payment Work, provider evaluation and
prepared owner handoff. It references the Authority intent/receipt and company-safe profile. It does
not own balances, approve payments, store credentials, declare settlement or model a provider's
internal workflow. System message principals remain provenance actors, not people or Work owners.

**Company Runtime.** Holds ordinary invoices, contracts and evidence and may consume the safe
  read-only company profile and cash projection. That projection may include an owner-approved public
  business registration identifier such as an ABN, which is required on ordinary Australian business
  documents; it never receives provider keys, short-lived access tokens, webhook secrets,
  private/personal tax identifiers or owner KYB documents.

**Owner surface.** Reuse the Sprint 07 authored attention meaning and source-owned actions. Show
provider-backed amount, currency, beneficiary, reason, evidence, approval state and observable next
state. Open the provider-native action or prepared link. Do not build a finance dashboard before one
real payment reveals the useful continuing view. Distinguish terminal outcome acceptance from
non-terminal permission to proceed: the control names the exact consequence rather than collapsing
both into “Accept”. Once the provider or cockpit observes the owner's action, retain the same causal
thread as a decision continuation until the first successor Work/Attempt, provider transition,
confirmed receipt or truthful blocker is visible. Chat may explain that state; it is not the source
of the state. Present the Exec separately from teams, make team leads the contact points and make
member selection inspection-first. Read actor kind and team relationships from OrgIntel; never infer
personhood, hierarchy or contactability from ids or role strings.

**Work owner surface.** Preserve Work as the OrgIntel primitive. Render the current outcome path,
hard handovers and review returns in a calm graph; render the same rows as a denser board when the
owner needs scanning rather than causality. Both lenses link to one detail surface containing the
outcome contract, current revision and Attempt, evidence, gates and relationships. Do not infer a
percentage or animate activity from Work status alone. An observed live session/process may be shown
with its observation time; absence, staleness and source unavailability remain distinct.

**External systems.** The registry owns registry truth. Airwallex owns onboarding, beneficiary,
approval, transfer and settlement truth. The protected ADI owns treasury truth. Restless stores
observations and references and reconciles rather than copying those systems into a second ledger.

## Verification sequence

### 1. Provider proof before integration

Using an Airwallex sandbox and then the actual account configuration, record:

- exact key scopes and account boundary;
- confirmation that the submit key lacks Beneficiary write;
- API access-token lifetime and refresh behaviour;
- an API-created transfer entering the provider approval workflow;
- status retrieval after a lost local response;
- webhook signature verification, stable event identity and duplicate delivery;
- revocation/regeneration behaviour;
- whether live account onboarding or approval workflow requires provider enablement.

No secret or provider response containing secret material enters the report.

### 2. Deterministic fake provider in a `_test` company

Exercise the same Authority adapter contract with controlled outcomes:

- read success and unavailable/stale balance;
- below-limit submission awaiting approval;
- above-limit denial;
- two individually valid pending payments exceeding the aggregate limit;
- provider rejection and budget release;
- success followed by lost response and status reconciliation;
- duplicate webhook and duplicate idempotency key;
- forged/invalid webhook signature;
- freeze and credential revocation;
- Runtime attempt to request the finance secret or invoke the finance effect through the generic
  effect child.

Expected results are exact Authority states and provider observations, not prose from an agent.

### 3. Airwallex sandbox

Repeat the supported path against the real sandbox. Capture provider IDs and states, not screenshots
alone. A sandbox receipt is labelled sandbox evidence and never enters a live company's confirmed
money totals.

### 4. Live bounded run

Only after the owner explicitly authorises onboarding, funding and the exact transfer:

1. owner confirms the safe legal profile;
2. provider onboarding/KYB completes through its own protected channel;
3. owner adds or verifies the beneficiary in the provider UI;
4. owner funds the operating wallet with the chosen maximum exposure;
5. Exec reviews real evidence and proposes one low-value payment;
6. Authority records and reserves the exact intent;
7. provider adapter submits it into approval;
8. owner approves in the provider UI;
9. the owner surface preserves the decision continuation and shows the approval observation, the
   responsible actor and the exact successor it released;
10. webhook/status reconciliation observes the terminal provider state;
11. the receipt survives daemon and Runtime restart;
12. OrgIntel resumes the exact dependent Work and the continuation reflects that observed transition.

The live run must never use a `_test` company or simulated market/customer fact. The transfer must be
genuinely owed or between owner-controlled accounts so the test does not manufacture a false business
expense.

### 5. Leakage and compromise checks

Use sentinel values and exact-value scans to prove that long- and short-lived provider credentials do
not appear in:

- company TOML;
- Authority or OrgIntel rows and JSON;
- Runtime environment or `/company` volume;
- browser/BFF payloads;
- command argv, process listings and shell history;
- logs, errors, traces and receipts;
- Git and ignored checkout files.

Then assume the Runtime is hostile: request an unapproved beneficiary, amount and effect command.
The strongest result it can obtain is a denied or awaiting-owner intent; it cannot retrieve the key,
change the provider approval workflow or debit protected treasury.

### 6. Sourcing and organisation checks

- In a `_test` company, present the Exec with a missing productive outcome and bounded candidate
  evidence. Observe it create ordinary internally owned Work for framing, research, decision, trial
  and evaluation without inventing an external Actor or a sourcing-specific state/edge.
- In the selected live company, show that T0's provider report and the retained/rejected provider
  decision are linked to that Work. Sprint prose alone is not evidence that the company can source.
- Show the decision's retained responsibility, provider responsibility, cost/authority need,
  acceptance evidence and reconsider trigger. No exact deterministic choice is asserted for this
  judgement path.
- Probe the People API and rendered surface with `exec`, Staff, `world`, `daemon`, leads, members and
  unassigned actors. System rows remain queryable for message provenance but never appear as people.
- Create or attempt to create Staff ids containing a prefix, team position or revision suffix; reject
  them before Work assignment. Prove a revision reuses the same actor id.
- In the browser, the Exec is the first distinct contact, leads look and behave as contacts, selecting
  a member opens Work-first inspection with a promoted route to the lead, and idle rows carry no
  universal `READY` label or duplicate role subtitle.
- Render the same representative Work fixtures in map and board lenses: a linear handover, branch,
  fan-in, revision return, blocked node, disconnected Work and completed history. Confirm the same
  rows, statuses, latest Attempts, evidence/gates and links appear in both lenses while relationship
  direction remains legible in the map.
- Compare the current layout candidate with the smallest credible alternative using those fixtures
  and one live Sprint 08 graph. Record layout quality, shipped weight, interaction/accessibility and
  maintenance cost, then retain one engine and delete the other. A screenshot of one easy chain is
  not sufficient evidence.
- Seed observed, stale, unknown and unavailable liveness. Only the observed row animates or names a
  current step; stale/unknown/unavailable remain visibly distinct and never collapse into idle or
  busy. Keyboard selection and reduced-motion mode preserve the same Work detail path.

## Risks and dispositions

| Risk | Disposition | Reason |
|---|---|---|
| A compromised Runtime drains protected treasury | **Invariant** | No treasury write credential enters Restless; the finance credential terminates host-side and is scoped only to the bounded operating account. |
| A compromised host Authority process submits every permitted operating payment | **Guarded** | Provider-native approval remains mandatory in this sprint and the operating balance is capped at an owner-chosen accepted loss. |
| A model approves its own spend or expands the envelope | **Invariant** | Models may recommend; deterministic Authority and the external provider decide permission and approval. |
| Wrong beneficiary details | **Guarded** | Restless cannot create/change beneficiaries; the owner establishes them in provider UI and the intent uses the immutable provider reference. Confirmation-of-payee evidence is retained where the provider supplies it. |
| Two pending payments individually fit but jointly exceed the envelope | **Invariant** | Authority atomically reserves pending/unknown amount before provider submission. |
| Lost response causes a duplicate transfer | **Invariant** | Stable idempotency plus provider lookup blocks blind retry while outcome is unknown. |
| Provider process exit is counted as settled money | **Invariant** | Only authenticated provider state produces confirmation; local execution remains submission evidence. |
| Provider outage blocks unrelated company work | **Accepted** | The affected read/payment state becomes unavailable; files, OrgIntel and other effects continue. |
| Operating-wallet provider becomes insolvent or restricts access | **Guarded** | Keep only bounded operating funds there, preserve an ADI treasury and record the provider's actual safeguarding/availability terms. |
| Legal/KYB data leaks through the shared Runtime | **Invariant** | Raw personal evidence is never stored there; provider-native owner handoff is the supported V0 path. |
| An approval disappears before the owner can tell what it caused | **Guarded** | The owner surface retains a causal continuation backed by existing Work, Attempt and provider states until it shows the successor, terminal outcome or blocker. |
| Company profile becomes a duplicate registry or cap-table system | **Guarded** | Store the minimum current profile and evidence references; external systems own legal/registry truth. |
| Provider selection creates a permanent abstraction for every bank | **Invariant** | T0 selects one runnable path; T6 deletes unused candidate scaffolding. A second provider must earn extraction later. |
| Sourcing categories become another workflow or universal capability algebra | **Invariant** | T7 uses ordinary Work, decisions, artifacts, effects and credentials. Posture is model judgement recorded as evidence, not a Work kind, edge or Authority command. |
| An external provider becomes an internal actor and silently sheds accountability | **Invariant** | Every Work retains an accountable internal owner; provider state and deliverables remain external references. Only an actual hire/internalisation decision creates Staff. |
| Hiding system principals removes transcript provenance | **Invariant** | `world` and `daemon` rows and message foreign keys remain; only the People projection filters `kind=system`. |
| A role or team move makes an actor id lie | **Guarded** | New Staff ids encode durable domain and craft only. Existing ids remain historical truth until an explicit owner/Exec repair; no automatic rename or merge. |
| Narrowing direct chat makes Staff inaccessible | **Accepted** | Staff remain inspectable and their Work visible; the promoted route goes to the accountable lead or Exec. Revisit if real owner work repeatedly requires bypassing that accountability. |
| A legitimate small payment to an established beneficiary is wrong in business judgement | **Accepted for this sprint only** | Provider approval remains with the owner. Later standing authority must name a smaller accepted loss before this disposition changes. |
| The Work board becomes a second workflow writer | **Invariant** | Map and board are projections over the same OrgIntel rows and expose no independent status lifecycle or drag-to-mutate path. |
| Attractive activity animation invents liveness | **Invariant** | Current activity requires an observed signal and observation time; stale, unknown and unavailable are first-class presentation states. |
| A layout library survives because it was tried | **Invariant** | T10 compares representative graphs and purges to one retained engine before T6 closes. |

## Deliberately out of scope

- Unrestricted bank access or automated treasury movement.
- Customer funds, escrow, marketplace payouts or money transmission.
- Beneficiary creation/update through Restless.
- Payroll, tax lodgement/payment, dividends, borrowing, investment or foreign-exchange automation.
- Autonomous live payment approval; every live submission in this sprint needs provider-native owner
  approval.
- Credit, overdraft or automatic replenishment of the operating wallet.
- A bookkeeping general ledger, accounts-payable suite, cap table, company-secretary product or legal
  advice engine.
- Raw passport/licence/biometric storage, a custom encrypted document vault or custom KMS/HSM.
- Multi-entity corporate groups, multiple owners or role-based human approvals.
- A generic banking/provider catalogue, open-banking write abstraction or simultaneous Airwallex and
  Wise implementations.
- Moving the established imported-secret backend merely to make finance look separately hosted.
- A new finance dashboard before the run shows what continuing owner view is useful.
- A bespoke approval-tracking state machine or activity feed that duplicates Work, Attempt, Authority
  or provider state.
- A sourcing-specific Work type, edge or lifecycle; a provider/capability registry; provider rankings;
  an `execute_capability` interface; or a first-class external engagement before repeated runs need it.
- Modelling an external provider's employees or workflow as Restless actors and Work.
- Automatic fuzzy merging or renaming of historical actor ids.
- A general org-chart redesign, actor-performance score, presence system or direct chat with every
  Staff member.
- A project-management suite, generic workflow builder, user-authored board columns, drag-to-change
  Work state, editable graph topology, activity log or custom graph-layout engine.
- Free-form graph positioning, persisted viewport/layout state or mobile graph editing before a real
  owner workflow demonstrates the need.

## Deletion

The sprint should make these paths or assumptions deletable:

- any ability for a `finance.*` effect to receive a credential through the generic Runtime
  effect-child;
- use of `CompanyConfig.name` as though it were a legal company name;
- any fixture-backed cash or payment value in the live owner surface;
- self-attested/process-exit payment records being counted as provider-confirmed money;
- unused provider-candidate code, interfaces or config after T0/T6 select one canon;
- owner instructions that ask for manual payment transcription or self-reported completion when a
  provider approval link/status exists;
- `isStandingActor` and every owner-surface actor-id allowlist used to infer personhood or contact;
- the `Not people` grouping, universal idle `READY` labels and duplicate permanent role subtitles;
- new Staff ids containing `staff`, mutable team position, `live`, revision, stage or retry history;
- the assumption in Exec context that every missing capability should become a new internal Actor;
- provider-selection reasoning that exists only in sprint prose rather than company Work and
  decision evidence.
- duplicate graph/board status derivation, invented progress percentages, unobserved busy animation,
  the losing Work layout engine and any fixture-only compatibility path needed only by that engine.

The sprint does not delete the generic governed-process effect runner, existing Infisical backend or
provider-neutral receipt concepts. It narrows where a live money secret may terminate.

## Founder decisions required before implementation

1. **Which legal entity backs the selected Restless company?** Supply or confirm the minimum legal
   profile; do not put personal KYB documents in the repository or chat.
2. **Which existing account remains protected treasury?** No write connection is requested.
3. **What maximum operating-wallet balance and per-payment amount are acceptable losses?** These are
   owner authority, not sprint defaults.
4. **May T0 open/use an Airwallex sandbox and prepare live onboarding?** Account creation, provider
   terms and KYB remain owner actions.
5. **What exact low-value live transfer is legitimate for the exit run?** The sprint must not invent an
   expense merely to obtain a receipt.

Founder alignment on the sprint does not itself answer questions 1–5 or grant the resulting
authority. Each is supplied through the relevant owner/legal/Authority boundary when the ticket
reaches it.

## Salvage

Reuse the clean-slate Sprint 05 Infisical credential backend and the current generic effect
idempotency/reconciliation logic as proven components. Re-validation is mandatory:

- the finance secret uses the live imported backend and repeats the exact-value leakage/outage probe;
- money intent reuses the idempotency idea but is exercised against provider lookup and aggregate
  budget reservation;
- Sprint 07's owner brief is reused for KYB/payment handoff and must retain the source-owned provider
  action rather than becoming a second approval record.

No legacy universal `Command`, asset custody, payment domain or immutable everything-ledger is
salvaged.

## Exit evidence

Sprint 08 closes only with:

1. the provider capability report from the actual sandbox/account surface;
2. an owner-confirmed legal profile and live registry observation where available;
3. an exact-value finance-secret leakage report and controlled Infisical outage result;
4. the adversarial `_test` matrix, including unknown, duplicate, forged webhook, over-budget and
   hostile-Runtime cases;
5. Airwallex sandbox provider IDs/status evidence;
6. one provider-native owner approval and low-value live terminal transfer receipt;
7. a restart reconciliation proving the receipt and reservation survive Runtime time travel;
8. owner-visible evidence that protected treasury retained no write connection and the operating
   balance stayed within the chosen exposure;
9. a deletion report naming the unsafe/unused path removed;
10. a friction note for every manual action, unavailable provider fact or owner clarification still
    required;
11. owner-visible evidence tracing the exact approval to the provider observation, responsible actor,
    successor Work and terminal outcome or blocker, with no model-invented intermediate state;
12. a sourcing run showing the company-owned Work from missing outcome through research, provider
    decision, real probe, use and evaluation, including the rejected or unrun alternative and the
    reconsider trigger;
13. a People API/render capture proving system principals remain in provenance but not the roster,
    the Exec is distinct and first, team leads are contacts, members are inspection-first, and one
    actor revision preserves the same conforming identity;
14. a deletion line for the UI actor-id list, `Not people`, universal `READY`, misleading Work focus
    copy and the internal-Staff-only missing-capability instruction.
15. a map/board capture and headless comparison over representative and live Sprint 08 Work proving
    shared rows/status/progress/detail links, readable `requires`/`revises` direction, honest observed/
    stale/unknown liveness and deletion of the losing layout engine.

If the final evidence is only a legal-profile form, sandbox API call, approval screenshot or green
unit test, the sprint is incomplete.
