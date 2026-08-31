# After action — first reference-led clean reset

## Outcome

The dogfood produced a genuinely fresh Astro/React landing page at commit
`b8755e37221e37c62724b26e85979af17d42a62d`. Its independent critic accepted the exact rendered
candidate after desktop, mobile, reduced-motion, keyboard, overflow, console and complete-page
checks. The new direction follows the Restless product language and uses the external maturity
reference as calibration rather than identity.

## What the harness exposed

1. Model selection was not a reliable preflight. Changing a company default did not update an
   existing Exec actor, and a configured provider-specific OAuth credential remained disabled after
   the company was added. Silent fallback and provider errors consumed attempts before production.
2. Owner review was attached too early. The first producer Work required owner review, which blocked
   the critic behind a judgement that should only happen after criticism.
3. Handoff commands were easy to confuse. The lead repeatedly used `resolve-handoff` where the
   owner-authority path required `prepare-owner-brief` followed by `escalate-handoff`.
4. Benign lead feedback could reopen completed producer Work, turning acceptance into unnecessary
   additional Attempts.
5. A team could announce completion with a container-only `127.0.0.1` URL, no owner attention item
   and no candidate integration branch. The page existed, but the owner could not open it.
6. A custom live-probe gate tried to discover the ReviewTarget artifact from the coordination graph
   before the same in-flight Attempt had persisted it. The resulting timing dependency blocked an
   otherwise healthy service. The gate must probe the declared loopback URL directly; terminal
   artifact linkage and handoff inspection enforce target identity separately.
7. Retiring a defective gate did not release the reserved owner-review gate name, while terminal
   admission still required a passing gate with that exact name. The installed CLI could not replace
   it in place. Until the substrate supports corrected same-name gate generations, the safe recovery
   is a new admission Work pinned to the same accepted commit and tree, with the failed Work retained
   as provenance.

## Resulting contract change

The skill now makes clean reset a three-stage graph:

- fresh producer Work from an exact clean base, without owner review or integration mutation;
- independent critic Work, also without owner review; and
- final delivery/admission Work from the accepted commit, with the candidate integration branch,
  one live isolated ReviewTarget and an explicit owner handoff.

It also requires a real model/credential admission preflight and distinguishes the supervised
container-loopback ReviewTarget from the ticketed isolated URL the owner can open. Declaring
completion from the former is forbidden. These are harness-level invariants worth automating later;
the skill makes them enforceable immediately without adding a new workflow engine.
