# OI idea bank

Recorded 12 November 2026, after reviewing recent Anthropic and OpenAI research posts and the top Hacker
News AI stories of the past year.

## Active programme

| # | Experiment | The question | Can run without the owner | Plan |
| --- | --- | --- | --- | --- |
| 1 | Do AI reviewers actually check? | When one agent reviews another's work, how many real mistakes does it catch? | Yes | [plan](01-do-reviewers-check.md) |
| 2 | The telephone game | How much of what the owner says survives each layer of AI management? | Yes | [plan](02-telephone-game.md) |
| 3 | Good news travels up | Do AI teams soften bad news on the way to the owner? | Mostly (owner grades a sample) | [plan](03-good-news-travels-up.md) |
| 4 | Forecasts the world scores | Do AI teams predict real outcomes better than one agent? | Yes (needs owner approval to publish predictions in advance) | [plan](04-forecasts-the-world-scores.md) |
| 5 | Searching for the best AI org chart | With the model fixed, how much does the organisation's shape change results, and does the best shape get flatter as models get stronger? | Mostly | [plan](05-org-chart-search.md) |

Suggested order: 2 and 1 first (cheap, and they build the shared task bank), start 4 early because
outcomes take time to arrive, then 3, then 5 as the flagship.

## Backlog (good ideas, not scheduled)

- **One decision a day.** How many owner decisions can the company make for itself without getting
  worse? Use every past Inbox item and the owner's real answer as labelled data, and run the decision
  model in shadow for a month of live items. Caution: a replay alone reads as a backtest (see below), so
  it needs the live month too.
- **Does an AI company remember why it made a rule?** Weekly probes of house rules each company learned
  the hard way, with and without the Library and with a nightly consolidation pass. Answers the HN
  thread "Agents don't need memory, they need documentation".
- **Brooks's law for AI teams.** Same backlog, 1 to 8 parallel workers, counting collisions
  (duplicated work, overwritten files, two agents contacting one person) against throughput. Partly
  covered by experiment 5.
- **A Project Vend-style P&L.** Sydney Resale trading with real money as a running case study. Better as
  the human story around another result than as an experiment.

## Evidence behind the choices

From the labs (Sep to Nov 2026):

- Anthropic, *Project Swap*: a market of Claude agents trading books for 201 employees. The best idea
  to borrow is the breakdown of the gap from the ideal (85% came from agents misreading what people
  wanted, not from bargaining).
- Anthropic Frontier Red Team, *Patterns and problems in emerging multiagent systems*: coordinated
  swarms found more but cost more; agents in a shared-resource conflict either forced a win or called a
  ceasefire; agents matched prices without a private channel.
- Anthropic, *How AI agents shape experts' judgment calls*: experts mostly went with the agent's
  preferred option.
- Anthropic, *The components of agent memory*: layered memory and an offline "dreaming" pass.
- OpenAI, *How agents are transforming work* and GDPval: blind expert comparisons; OpenAI replaced a
  multi-agent system with one agent holding many tools.

From Hacker News (top stories Nov 2025 to Nov 2026, comments read):

- "An agent designed to run any company autonomously" (Andon Labs, 497 points, 618 comments) drew
  accusations of astroturfing. "AI runs your company" framing is a liability.
- "Improving 15 LLMs at Coding in One Afternoon. Only the Harness Changed" (832): one controlled change
  with a clean measurement. Experiment 5 is built on this shape.
- "My AI agents found an unmapped Maya site" (890): a real, checkable result. Experiment 4 is built on
  this shape.
- "We gave 5 LLMs $100K to trade stocks" (398): dismissed as "paper money" and "backtested". Replays
  need live data alongside them.
- Agent incident stories are huge ("An AI agent published a hit piece on me", 2,346) but readers blame
  the operator. Experiments 1 and 3 are about catching problems before they become incidents.
- The year's top AI item is the HN rule against generated comments (4,229). The owner writes the prose.

Reddit could not be read directly (scripts and the browser pane are blocked). Secondary sources say
r/ClaudeAI rewards first-person setups, guides and posts about verification ("Do not trust AI to test
AI"), and requires mod permission for promotion.
