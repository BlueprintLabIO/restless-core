# S36-T5 — Reconcile recovery and prove terminal absence

**Layer:** Authority, Runtime lifecycle and evaluation

**Observed outcome or friction:** Provider timeouts and daemon restarts can create duplicated public
endpoints or abandoned paid workloads; issuing a delete is not proof of cleanup.

**Work:** Inject response loss, duplicate callback, process crash, daemon restart, Runtime replacement,
expiry and owner revocation. Reconcile by publication identity and provider operation. Re-observe that
endpoint, workload, invitation, lease and scoped temporary artifacts are absent.

**Evidence:** No injection creates two live endpoints, loses an owed cleanup or rolls newer Authority
history back with the Runtime. Every terminal case carries a verified cleanup receipt.

**Deletion target:** Timeout-as-completion, best-effort deletion and company-wide poison markers.
