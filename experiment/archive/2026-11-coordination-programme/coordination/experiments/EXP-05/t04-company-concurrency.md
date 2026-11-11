# EXP-05 concurrent-company result

**Completed:** 26 August 2026
**Disposition:** product work exact; branch stopped infrastructure-invalid at blind evaluation

## Main answer

Restless did run four independently accountable departments at once: sales, customer operations,
monitoring and invoice reconciliation each had one non-producing lead and one Staff producer. Their
90 units closed exactly with valid attribution and no observed cross-department leakage. Exec had
returned from the initial dispatch before the fourth request arrived.

The cell does **not** count as a complete organisational comparison. The support blind evaluator
omitted a required `decision` field twice, including the one permitted malformed retry. The branch
stopped without replaying production or inferring the missing decision. Isolated-baseline comparisons
and a company-level semantic verdict are therefore not claimed.

## Observed product evidence

| Measure | Observation |
| --- | ---: |
| Exact locally closed units | 90/90: sales 40, support 8, monitoring 10, operations 32 |
| Peak concurrent Staff Attempts | 4 |
| Peak concurrent Staff model calls | 3 |
| Fourth-request dispatch latency | 68.679s |
| Exec CLI unavailable for fourth request | 69.432s |
| Exec returned before fourth request | true |
| Lead native-review durations | 35.2–74.3s |
| Cross-department leakage | none observed by exact projection |
| Lead-to-lead message count | unknown; bounded projection did not expose it |
| Charged spend | $1.3719 |

Sales, monitoring and operations each received valid blind `accept` judgements, with worst-unit scores
9, 10 and 6. The support content itself was grounded and safe, but its packet was schema-invalid
because `decision` was missing. Product evidence is preserved separately from measurement validity.

Evidence: [`run result`](results/company-q1x4-r1-20260826-glm53-r1/run-result.json) and
[`blind packets`](results/company-q1x4-r1-20260826-glm53-r1/blind-evaluations.json).

## Product consequence

The wide, shallow company shape remains viable, but the 69-second synchronous fourth dispatch is an
observed responsiveness bottleneck. The next implementation slice should make accepted dispatch
durable and return portfolio availability before the new lead's model orientation completes. The
experiment evaluator should use grammar- or schema-constrained output; that is test infrastructure,
not a production coordination primitive.
