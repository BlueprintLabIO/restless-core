# v03 — model-diverse launch contract

## Change under test

Keep the ACP/Pi harness and prompt identical while changing only the pinned live-free OpenRouter model.
This tests whether the first-party controls are model-neutral and exposes provider/model variance before
coordination experiments begin.

## Evidence

| Model | Result | Tool calls | Usage | Cost |
| --- | --- | ---: | --- | ---: |
| `poolside/laguna-xs-2.1:free` | exact marker, no-edit claim correct | 1 | 278 in / 75 out / 256 cache read | $0 |
| `cohere/north-mini-code:free` | exact marker, no-edit claim correct | 1 | 279 in / 469 out | $0 |
| `google/gemma-4-26b-a4b-it:free` | upstream shared-pool 429 before first token | 0 | 0 | $0 |

Every launch independently proved prompt and completion price `0` from the live catalogue. Laguna and
North produced the same read -> result behavior through ACP. Gemma's exact provider error was preserved;
it is infrastructure unavailability, not evidence about task quality.

## Score

Harness/model-portability score: **90/100**.

| Criterion | Points |
| --- | ---: |
| Same exact contract across three model IDs | 20/20 |
| Two independent models execute the same tool correctly | 20/20 |
| Live zero-price gate for every launch | 20/20 |
| Provider failure remains explicit and zero-usage | 20/20 |
| Error semantics across ACP | 10/20 — v1 lacks an error stop reason; metadata must disambiguate `refusal` |

## Decision

Retain model-neutral launch. Add an explicit runtime `outcome` (`completed`, `cancelled`, `error`,
`max_tokens`) in Restless metadata. A retryable provider failure must not fail or revise Work. Do not
silently route to a paid model.
