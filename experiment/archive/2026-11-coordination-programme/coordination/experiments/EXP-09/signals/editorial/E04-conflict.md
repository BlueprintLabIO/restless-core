# Controlled source event E04

- source_ref: `exp09://editorial/provider-conflict-v1`
- observed_at: `2026-08-26T02:00:00Z`
- classification: `_test low-trust conflicting observation`
- intended responsibility: research publication

## Observation

One unreplicated Q2 rerun produced lower throughput than Q1. The provider trace for that rerun is
missing admission-wait fields and one worker process restarted before terminal usage was recorded.

## Evidence boundary

The observation conflicts with the current capacity claim but cannot distinguish topology from
provider/runtime failure. It is insufficient to reverse the accepted article. A bounded research note
may record the conflict and the exact clean rerun needed. Do not average it into false certainty.
