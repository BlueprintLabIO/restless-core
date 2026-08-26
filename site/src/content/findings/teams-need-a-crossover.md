---
title: "Teams need a crossover, not a slogan"
deck: "The same 240-account workload rewarded two workers when demand arrived at once and rejected them when demand was paced."
publishedAt: 2026-08-26
order: 1
readTime: "6 min"
run: "EXP-05"
finding: "Team size should follow valuable work waiting at the same time. Total task count is a poor substitute."
status: "Accepted"
---

We started with an easy claim to believe: sales work should parallelise. Each account can be owned by
one person. Accounts can close locally. The result does not need one writer to combine every draft.

That description was correct and incomplete.

## One population, two demand shapes

The experiment used the same 240-account population in two conditions. In the first, all accounts
were ready at once. In the second, six batches arrived every 45 seconds.

With the full backlog available, two workers beat one in two separate runs. Throughput reached 1.98
times the baseline in one run and 2.83 times in the next. The 90th percentile completion time fell to
53 percent and then 37 percent of the single-worker result. Charged cost per accepted account also
fell.

The paced version did not cross. Two workers improved throughput by 24 to 32 percent, yet the 90th
percentile completion time did not improve. The lead kept a single worker in both natural elastic
runs.

The task count never changed. The arrival shape did.

## Idle capacity has a cost

An added worker pays a fixed price before useful overlap begins. It must receive context, inspect the
source and establish its local operating state. A non-producing lead must still sample and judge the
result. Those costs remain even when the queue cannot keep both workers busy.

Parallelism wins only after the available work can repay those fixed costs. A long list says little
about that condition. The useful question is whether valuable independent work is waiting while the
current worker is occupied.

That distinction also explains why adding four workers did not become the new default. Four-way sales
production finished much faster than the accepted two-worker arm, but its weakest outputs fell beyond
the frozen quality tolerance. Mechanical capacity was real. Accepted capacity stopped at two.

## Local closure comes first

The result depends on each account closing without a central model rewrite. A worker produced an exact
account outcome. Deterministic checks validated the full population. The lead reviewed samples and
exceptions instead of rewriting 240 units into one memo.

When a batch needs one model to reread and combine every contribution, added workers create a new
bottleneck at the end. The assembler absorbs the time saved upstream and introduces another place for
facts to drift.

Sales avoided that trap because the account was the product unit. Monitoring later showed the same
shape: 40 entities divided between two workers reached 1.92 times the throughput, cut the tail latency
almost in half and reduced charged cost per alert. Each entity ended as its own alert. No model fan-in
was required.

## The operating rule

Start with one end-to-end worker for a coherent outcome. Add another worker when all four conditions
are visible:

1. units can close independently;
2. useful units are waiting at the same time;
3. earlier completion changes response value, coverage or opportunity;
4. lead review and provider capacity can sustain the overlap.

Then measure the next worker at the margin. Stop when tail quality falls, response time stays flat or
lead review erases the gain.

This is deliberately a judgment call. Encoding 240 accounts or a department name into a staffing
rule would preserve the surface detail and lose the cause.

## Boundary of the finding

The work used fictional accounts in isolated test companies, one model family and one provider route.
It supports a demand-sensitive staffing prior. It says nothing about universal sales productivity,
real prospect response or the best team size under another model.

The next honest step is a real inbound queue with externally observed arrivals. The crossover should
move with service urgency, provider headroom and outcome quality. If it does not, the current account
fixture taught us less than it appeared to.
