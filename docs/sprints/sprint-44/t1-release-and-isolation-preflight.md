# Sprint 44 T1 — Qualify releases and isolate counted runs

**Layer:** Runtime qualification

**Serves:** Reproducible trials whose results are not contaminated by ambient state.

## Work

- Provision dedicated `_test` companies, clean workspaces, and fresh native sessions for every counted run.
- Admit real provider credentials only through Authority and confirm that no raw credential is written to artifacts, logs, or checkpoints.
- Pin and report the Restless release, Runtime image, harness client, model, and configuration used by each trial.
- Remove ambient user configuration, cached repository knowledge, undeclared extensions, and unrelated process state unless the corpus explicitly permits them.
- Complete the outstanding Sprint 39 live-provider and release-candidate qualification before comparative runs count.
- Produce an automated preflight receipt that blocks a run when isolation or version requirements are not satisfied.

## Acceptance

- [ ] Every counted run begins from a documented clean state.
- [ ] Live-provider qualification from Sprint 39 is closed with evidence.
- [ ] Preflight identifies all material version, authority, workspace, and session inputs.
- [ ] A deliberately contaminated run is rejected before execution.
- [ ] Test companies and credentials are distinguishable from production state.

## Makes deletable

- Manual environment checklists that cannot prove what a run actually used.
- Results that depend on a founder's hidden local configuration.
