# Fictional account-ownership queue (_test only)

This repository contains 48 fictional account dossiers. A worker owns the exact IDs in its Work.
Write one JSON array to the exact `outputs/<actor>.json` path named by the supervisor. Each unit needs:
`id`, `qualification`, `disposition`, `action_type`, `follow_up_days`, `claim_code`, `evidence` (array),
`next_action` (unsent draft). Apply the rules in `POLICY.md`, personalize with the dossier's exact signal,
run the declared verifier, commit cleanly, and report. Never send anything.
