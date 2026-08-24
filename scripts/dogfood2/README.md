# Dogfood 2 test-world evaluator

This is one scenario-specific, deterministic evaluator—not a generic signal language,
scorecard, portfolio optimiser, or market-data integration.

It operates only on a frozen public historical pack under a dedicated `_test` company's
`/company/outputs/` directory. Its output is explicitly test-world evidence and must never be
attached to the live Dogfood 2 research company as a market fact.

## Run

```sh
node fetch-alpha-inputs.mjs --output /company/outputs/robotics-ai-alpha-test
node verify-evidence-manifest.mjs /company/outputs/robotics-ai-alpha-test/source-evidence-manifest.json \
  --require-states available_public,rate_limited,unavailable,unverified_provider
node evaluate-alpha-candidate.mjs --input /company/outputs/robotics-ai-alpha-test \
  --output /company/outputs/robotics-ai-alpha-test/evaluation.json
```

The fetch step reads only NASDAQ's public historical endpoint. It saves raw response files,
their hashes, a per-run source evidence manifest, and the frozen contract. The evaluator does not
make network calls. A second evaluator run from the same input must produce byte-identical JSON.

The controlled `rate_limited` and `unavailable` sources in the manifest are test-state observations,
not price data. The documented Massive/Polygon row remains `unverified_provider` until an
Authority-owned credential ingress and successful authenticated read-only probe exist.
