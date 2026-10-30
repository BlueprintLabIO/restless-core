# Native token cap and unknown usage

30 October 2026. A configured native-harness token cap previously counted a turn with no reported token usage as zero; a failed ledger open also returned a default zero usage to the admission check. Both defects were reproduced with focused tests before this change. The native admission check now refuses a new capped turn when its usage ledger cannot be read, and refuses a token-capped turn when this month contains a recorded native turn without a token count. A turn-only cap remains usable when token details are absent. Uncapped native turns and metered relay routes retain their existing behavior.

The focused engine suite passed four tests, including both reproduced cases, restart with unknown usage, a turn-only cap, the existing monthly cap and deduplicated native accounting. Fourteen existing model-gateway spend tests also passed. These are local unit checks, not evidence of provider usage reporting or production token savings. A failed write after a turn, accounting recovery after process restart, simultaneous native turns near a cap, and an actual provider round trip are not covered by this change; A0.4 and the complete resource gate remain open.


A later Staff-session change records each native prompt as it completes and checks the cap before a subsequent productive prompt, completion prompt or correction. An absent usage report is recorded as unknown using the prompt's stable identity. The source-owned test `scripts/verify-native-staff-cap.py` runs the actual Staff driver against a real disposable PostgreSQL database and a simulated session. On the previous code, a one-turn cap allowed two prompts; on the candidate it allowed one. The candidate also passed token-cap, missing-usage, malformed-completion, turn-only and uncapped cases. Both fixtures reported clean removal. The source-fingerprinted local Connections release smoke now runs this check before publishing. This is not an end-to-end real-provider test and does not cover concurrent sessions, Exec's own follow-up calls, or durable recovery after a failed accounting write. A0.4 remains open.


Exec now records the productive native prompt before considering a rewrite or termination prompt, checks the owner-set cap before each, and records their usage too. If the cap prevents a follow-up, an already completed owner reply remains available. A failed or timed-out postflight is recorded as unknown rather than zero. If a command-containing reply cannot be rewritten because the session stops, the tested visible reply and summary contain a plain explanation instead of the command. In a simulated-session test the previous source made two calls with a one-turn cap; the candidate made one and preserved the reply. Seven Exec tests passed, including token/turn caps, capped rewrite, failed postflight and the existing interruption and termination tests. This is not a real-provider or concurrent-session test; cancellation during an in-flight postflight and recovery after a failed accounting write also remain open. A0.4 remains open.


### Interrupted Exec completion follow-up

An owner interruption after the native completion prompt has begun now records an
unknown-usage turn before returning the interruption. A configured native token cap
then refuses the next turn instead of counting the interrupted request as free. A
pre-cancelled request records no turn. The real ledger and a pending session were
exercised in `interrupted_native_postflight_records_only_started_usage`; all eight
`exec::tests` passed on 30 October. This does not prove provider-side cancellation,
prevent concurrent sessions from racing a cap, or repair a failed ledger write.
