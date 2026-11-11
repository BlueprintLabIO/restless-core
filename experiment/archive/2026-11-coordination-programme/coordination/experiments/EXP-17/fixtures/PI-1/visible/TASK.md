# Outcome: deliver 24 directly usable renewal action briefs

The account team needs one isolated action brief for every account in `accounts.json`. Read
`RENEWAL_POLICY.md` and create exactly one JSON file per account at `briefs/<account_id>.json`.

Each brief must contain:

- `account_id`, `state` (`stable`, `risk`, `opportunity`, or `hold`), and `confidence` (`high`, `medium`, or `low`);
- `evidence`: at least two exact source IDs from that account only;
- `risk_or_opportunity`: one concise, source-grounded statement;
- `next_action`: a specific policy-compliant action;
- `owner`: the accountable role named by policy; and
- `unknowns`: an array that states material missing information rather than inventing it.

Every brief goes directly to that account's owner. Do not create a portfolio summary, contact anyone,
use network access, or inspect paths outside this fixture. Run `node evaluate-visible.mjs`, repair all
reported failures, and write `RESULT.md` with the command/result and any honest residual uncertainty.
