# S23-T3 — Rebuild Blog publication layout

**Layer:** Restless Cloud.

Give every standalone article a generous desktop reading measure, wider publication grid, breakout
figures/evidence/callouts and useful related navigation while preserving mobile readability.

**Observed friction:** renaming Journal and adding routes did not fix the timid nested reading column.

**Deletion target:** the narrow article shell and long pages made from prose alone.

## Observed closure — 28 August 2026

- All five entries remain independently addressable under `/blog/<slug>/`; public navigation uses Blog,
  not Journal.
- Computed desktop article measure is 850px, with a wider publication grid for the article figure,
  evidence band and previous/index/next navigation; mobile measure is 346px without horizontal overflow.
- Each article includes an entry-specific evidence-flow figure, source-bearing evidence strip and
  complete related navigation rather than a nested narrow prose column.
