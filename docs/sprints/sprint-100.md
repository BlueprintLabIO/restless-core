# Sprint 100: Token efficiency — lean wakes, earlier compaction, smaller prompt, producing leads

Owner direction (2026-10-19): "aim for 10x" fewer tokens without destroying quality. Act on all of:
keep large tool output out of history, make lead wakes cheap, compact earlier, coarser tools. Decisions:

- **Leads produce directly.** A lead owns its outcome's production through Work it owns, and adds
  members only when a seam repays coordination. This replaces the non-producing-supervisor
  invariant; EXP-17 measured the serial Exec/lead/worker chain at 1.73x-3.30x solo cost for matched
  quality.
- **Model choice stays the owner's.** New connections default to the provider's everyday tier
  (Terra, Sonnet, Flash), not the flagship; any agent can still be given a stronger model.

## Where the tokens went (measured before any change)

`scripts/token-report <company>` summarises the owner spend report (every gateway call with actor,
session, Work, Attempt and input/cached/output split). sydney_resale, 30 days to 2026-10-19:
10,048 calls, 908M input tokens (95.5% cached), 2.7M output.

- **History carried into wakes was 85% of input.** A session's smallest request had a median of
  116k tokens; the median session made 4 calls. Exec resumed one session that grew from 180k to
  244k tokens across wakes, about 4 calls of ~200k each per wake.
- **Exec 434M, lead (team) wakes 271M, Work Attempts 204M.** Work sessions start at 14k-18k and
  grow to ~100k in long Attempts.
- **The route was Codex** (`openai-codex/gpt-6-sol`), which compacts near the model window, so the
  OMP 80k compaction setting never applied to this company.
- **The fixed Exec prompt was 80.6 KB** (~20k tokens) with an empty company; the company operating
  rules alone were 28 KB in every Exec and Staff call.

## What changed

1. **Lean wakes (Runtime).** After a turn whose session ended above 48k tokens, the session locator is
   retired, so the next wake reconstructs from durable state and the bounded snapshot, exactly as a
   context overflow already did. Exec and every Staff wake (`model_session_retired` event).
2. **Compaction and tool output (Runtime).** Codex: `model_auto_compact_token_limit=64000`,
   `tool_output_token_limit=3000`. OMP: compaction at 64k (was 80k); tool output above 12 KB spills
   to an artifact with 4 KB head and 8 KB tail inline (was 50 KB, 20 KB + 20 KB).
3. **Smaller fixed prompt (OrgIntel/Runtime).** Operating rules 28.4 KB -> 15.1 KB: tool-by-tool
   procedures moved to the built-in `company-tools` skill, read on first use; maintainer provenance
   moved to `docs/COMPANY_OPERATING_RULES_NOTES.md`. A budget test now guards the Exec fixed prompt.
4. **Lead-direct production (OrgIntel).** A lead may own production Work; the scheduler claims it;
   Exec commissions an outcome as the lead's own Work in the same turn, removing the paid lead turn
   that restated the brief. Deleted: the lead-owned-Work rejection, the scheduler's lead exclusion
   and the lead's forced-`blocked` Attempt ending.
5. **Everyday-tier default (Cockpit).** One shared `defaultChoice` replaces three copies.

Replaying the 30-day record under 1-3 alone (call counts unchanged, so 4 is not credited) gives about
3x fewer input tokens: Exec ~5.5x, lead wakes ~5x, Work ~1.2x.

## Checklist

- [x] Measure the baseline (`scripts/token-report`)
- [x] Lean wakes, compaction and tool-output limits
- [x] Operating rules split; Exec prompt budget test
- [x] Lead-direct production
- [x] Everyday-tier default
- [ ] Live before/after on the installed appliance
