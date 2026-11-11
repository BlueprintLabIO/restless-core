# EXP-05 G1/G3 product-path charter

Remain a non-producing supervisor. Commission exactly two repo-less Work nodes, then observe and
repair through Staff:

- `continuity-worker` reads `/company/experiment/EXP-05/inbox/marker-source.json`, creates
  `/company/experiment/EXP-05/progress/continuity-target.produce` immediately after orientation, writes exactly the one-key JSON object to
  `/company/experiment/EXP-05/outputs/continuity-target.json`, validates it, links it as `review_target`, and returns outcome_met. Expected
  artifact `/company/experiment/EXP-05/outputs/continuity-target.json`; attempt limit 3; atomic gate `{"name":"marker-contract","command":["python3","/company/experiment/EXP-05/tools/validate-marker.py","/company/experiment/EXP-05/inbox/marker-source.json","/company/experiment/EXP-05/outputs/continuity-target.json"]}`.
- `continuity-control` independently reconciles `/company/experiment/EXP-05/inbox/control-operations.json` into
  `/company/experiment/EXP-05/outputs/continuity-control.json`, with progress marker `/company/experiment/EXP-05/progress/continuity-control.produce`, exact ordinary operations schema,
  visible validation, and an attributable `review_target`. Expected artifact `/company/experiment/EXP-05/outputs/continuity-control.json`;
  attempt limit 2; atomic gate `{"name":"unaffected-control-contract","command":["python3","/company/experiment/EXP-05/tools/validate-visible.py","operations","/company/experiment/EXP-05/inbox/control-operations.json","/company/experiment/EXP-05/outputs/continuity-control.json","--progress","/company/experiment/EXP-05/progress/continuity-control.produce"]}`.

Do no production. If the target process is interrupted by changed Work feedback, inspect its exact
recovery evidence and resume the same Work because the source changed; do not recreate it or touch
the unaffected control. When the exact validator-complete event arrives, review both artifacts
read-only and send one material G1/G3 judgement to Exec. Do not poll or narrate status.
