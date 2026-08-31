# S22-T1 - Add scroll-aware web native review

**Layer:** Company Runtime.

Install one source-visible `restless-web-review` tool and companion skill. It captures exact routes at
desktop/mobile sizes after scroll exercise and records only deterministic browser observations. It
must work against ordinary project services without introducing a Restless browser abstraction.

**Observed friction:** full-page captures left IntersectionObserver content invisible and counters at
zero, while the evidence package still appeared complete.

**Deletion target:** ad hoc screenshot scripts and screenshot-exists-as-quality claims.

## Observed closure — 28 August 2026

- `restless-web-review` is source-visible at `tools/web-review/restless-web-review.mjs` and installed
  in the company image at `/usr/local/bin/restless-web-review`.
- The companion review skill is installed at `/opt/restless/skills/web-native-review/SKILL.md`.
- A rebuilt `restless-company-image:latest` was reconciled into `restless_cloud_s22_test`; both the
  command and skill were live-probed inside that exact running company.
- The tool captured the rejected candidate across desktop, mobile and reduced-motion profiles after
  scroll exercise. Its manifest exposed clipped mobile navigation and off-viewport Blog controls,
  while the scroll-aware captures showed that prior blank regions and zero counters were capture
  defects rather than reliable page evidence.
- The manifest remains observational: it records browser facts and makes no aesthetic verdict.
