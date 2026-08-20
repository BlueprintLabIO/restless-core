# S08-T6 · Adversarial `_test`, sandbox, bounded live run and purge

**Layer:** All touched layers.
**Serves:** the complete Sprint 08 success contract.
**Depends on:** S08-T1–T5 and S08-T7–T10.
**Observed friction:** a green adapter test cannot prove that a real provider accepted the account,
that a real owner approval was required, that credentials stayed contained, that money settled
exactly once or that the Work map and board truthfully explain the company path that produced it.
**Makes deletable:** unsafe finance use of the generic effect child, sandbox/live compatibility
shortcuts, unused provider candidates and fixture-backed owner money views.

## Outcome

The deterministic fake provider proves failure behaviour, Airwallex sandbox proves the external API
contract, and one explicitly owner-approved low-value live transfer proves the business consequence.
The run report names every owner action and purges the losing/unsafe paths.

## Required run

1. Run the full `_test` matrix from the sprint spec, including concurrent reservation, lost response,
   duplicate idempotency, forged webhook, unknown status, freeze, revocation and hostile Runtime.
2. Repeat supported flows against Airwallex sandbox with provider IDs/status evidence.
3. Repeat Sprint 05-style exact secret-value scans and controlled Infisical outage.
4. Confirm the owner-approved safe legal profile and provider onboarding state.
5. Confirm the protected treasury has no Restless write credential.
6. Fund the operating wallet only to the owner-chosen maximum exposure.
7. Prepare one genuinely legitimate low-value payment to an owner-established beneficiary.
8. Observe API submission enter provider approval; obtain the owner's provider-native approval.
9. Observe and reconcile the terminal provider result.
10. Restart the daemon and replace/restore the Runtime; prove the receipt survives and no duplicate is
    attempted.
11. Resume the dependent OrgIntel Work from provider evidence.
12. Show the live company's sourcing Work from missing outcome through provider research, decision,
    real probe, use and evaluation; retain no provider Actor or sourcing-specific state/edge.
13. Render the corrected People surface with provenance-only system actors absent, the Exec distinct,
    leads contactable, members inspection-first and one revised Work retaining the same actor id.
14. Render the representative and live Sprint 08 Work through both map and board; prove shared
    source meaning, readable `requires`/`revises`, honest liveness and the same detail path.
15. Remove every unused provider candidate, unsafe finance binding, fixture/live compatibility path,
    owner-surface actor-id list, misleading roster representation and losing Work layout engine.

## Pass condition

Every Sprint 08 success criterion has a source-backed evidence line in the run report. The owner
confirms the payment they approved and the maximum exposure they accepted. Provider state confirms
the result. The exact-value scan and hostile-Runtime test confirm the credential boundary.

The ticket does not pass on a sandbox transfer, owner screenshot, self-attested JSON, manually edited
receipt or test that never touches the live adapter. It also does not pass when provider selection
exists only in the sprint document or when the People correction is evidenced only by a screenshot
without matching OrgIntel kinds, teams, Work and message provenance. The Work surface does not pass
on one easy-chain screenshot, two views with divergent status meaning or activity inferred from
`active` without an observed signal.

## Risks

- **Test spends money without a legitimate purpose — invariant:** the owner selects and approves the
  exact low-value transfer; the sprint does not invent an expense or send to an uncontrolled party.
- **Live failure contaminates a company with simulated evidence — invariant:** deterministic fixtures
  run only in `_test`; live company records contain only real provider observations.
- **Exploration survives as permanent machinery — invariant:** the run report includes an explicit
  purge list and one provider path remains.
- **Sourcing test becomes provider theatre — invariant:** the live Work references provider-native T0
  and T6 evidence; `_test` demonstrates mechanics only.
- **Roster cleanup rewrites history — invariant:** system sender and historical Staff rows remain;
  presentation and future creation semantics change without deleting provenance.
- **Graph experimentation accumulates — invariant:** representative evidence chooses one layout
  implementation and T6 verifies the losing dependency and adapter are gone.
