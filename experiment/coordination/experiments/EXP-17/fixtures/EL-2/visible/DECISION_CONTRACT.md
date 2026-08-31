# Standing strategy decision contract

Maintain `DECISION_LEDGER.json` with:

- `schema: "exp17.el2.ledger.v1"`;
- `seen_signal_ids`, with each causal signal represented once;
- exactly three keyed decisions: `pricing-response`, `health-vertical`, and `partner-channel`;
- for each decision: `state` (`act`, `hold`, or `investigate`), `conclusion`, `action`, `evidence`
  (exact signal IDs), `unknowns`, and `last_changed_by`;
- a top-level `updated_for_signal` naming the newest non-duplicate signal applied.

Current evidence must displace a contradicted conclusion. A duplicate signal is a no-op: it must not
appear twice, create a second decision, or change `last_changed_by`. Unsupported certainty is a serious
defect. A later source refresh may legitimately change only the decision it addresses.
