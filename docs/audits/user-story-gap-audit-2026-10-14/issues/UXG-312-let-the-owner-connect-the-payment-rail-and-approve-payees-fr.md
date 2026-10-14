# UXG-312 — Let the owner connect the payment rail and approve payees from the web

| Surface | Severity | Type | Stories |
|---|---|---|---|
| Cross-surface journeys | high | missing-function | US-190, US-205 |

## Problem

The only payment rail is Airwallex, and its money envelope (source account, currency, approved beneficiaries, per-payment and aggregate limits) can be set only through `restless finance` on the CLI. There is no owner API route for it, no Apps catalogue entry and no Company page. An Exec purchase that needs a new supplier dead-ends before the Inbox payment item can exist. Distinct from UXG-185 (freeze and allowance controls as spec drift) and UXG-127 (accounting apps), because the missing pieces here are the provider connection and adding payees.

## Evidence

- `crates/restless-engine/src/finance.rs:14-23` (envelope fields incl. beneficiary_refs)
- `crates/restless-engine/src/finance.rs:214-220` (set_envelope)
- `crates/restless/src/main.rs:2833-2844` (finance show/set-envelope CLI only)
- `crates/restless-engine/src/attention.rs:2526-2547` (payment approval happens in Airwallex once a payment already exists)
- `web/src/lib/assets/app-logos` (no Airwallex entry)

## Suggested direction

Add a Company → Money section, or an Apps entry, that connects the provider and edits the envelope and payee list. Let Exec propose a new payee as an Inbox approval.

_Audit finding only — not implemented. Status lives in the checklist in [`../README.md`](../README.md)._
