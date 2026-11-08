#!/usr/bin/env node
/* Token parity: the same three owner tasks through Restless and through a single Claude Code-style
 * agent loop, against one scripted model (tools/mock-model) that counts every call and its tokens.
 *
 *   node scripts/bench/token-parity/run.mjs --origin http://127.0.0.1:<web> --company <x>_test \
 *     --mock http://127.0.0.1:<mock port> [--out result.json] [--quiet-ms 45000]
 *
 * The stack must run on the mock (RESTLESS_ANTHROPIC_BASE_URL) with scenario restless.json (or
 * restless-before.json for the earlier prompts); bench.sh starts both. What is scripted and what is
 * measured: the scripted model takes, in each turn, the fewest tool calls its prompt tells it to.
 * Everything else is Restless itself: which actors wake and how often, the extra calls a turn makes
 * (its end-of-turn decision), and every token each request carries (prompts, tools, history, tool
 * output), with Anthropic's caching rules. The baseline replays the same tasks as one agent with a
 * Claude Code-sized fixed prompt (see README.md for how that size was chosen).
 *
 * Tasks: (a) send one templated email; (b) a one-page document from a brief with its review; (c) an
 * owner question about company state; (d) team work, a document someone else writes and checks; (e)
 * a chain, a draft and then its check. Each is also run by two baselines (a solo agent and an agent
 * with sub-agents), timed (pass the mock `--latency` for modeled model time) and checked against a
 * scripted rubric. */
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { writeFile } from 'node:fs/promises';

const args = Object.fromEntries(
	process.argv.slice(2).reduce((pairs, value, index, all) => {
		if (value.startsWith('--')) pairs.push([value.slice(2), all[index + 1]]);
		return pairs;
	}, [])
);
const origin = args.origin;
const company = args.company;
const mock = args.mock;
const quietMs = Number(args['quiet-ms'] ?? 45_000);
/* `api` (an API key on the Messages relay) or `subscription` (a Claude sign-in through OMP's host
 * gateway). A direct harness on a sign-in (Claude Code on a Pro/Max seat) writes one-hour cache
 * entries, so the subscription baseline does too. */
const route = args.route ?? 'api';
const BASELINE_TTL = route === 'subscription' ? { ttl: '1h' } : {};
const callLog = args.log;
const capMs = Number(args['cap-ms'] ?? 600_000);
assert(origin && new URL(origin).hostname === '127.0.0.1', '--origin must be the local stack');
assert(company?.endsWith('_test'), '--company must be a disposable _test company');
assert(mock, '--mock is the mock model origin');
const api = `${origin}/api/companies/${encodeURIComponent(company)}`;
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function stats(reset = false) {
	const response = await fetch(`${mock}/stats`, { method: reset ? 'DELETE' : 'GET' });
	assert(response.ok, `mock stats ${response.status}`);
	return response.json();
}

async function tellExec(text) {
	const form = new FormData();
	form.set('client_command_id', randomUUID());
	form.set('body', text);
	const response = await fetch(`${api}/actors/exec/conversation`, {
		method: 'POST',
		headers: { Origin: origin },
		body: form
	});
	assert(response.ok, `owner message ${response.status}: ${await response.text()}`);
}

/* A task is over when no model call has been made for `quietMs` (longer than the Runtime's mail
 * settle window, so a coalesced notice still counts), or at the cap. */
async function settle() {
	const started = Date.now();
	let last = (await stats()).calls;
	let quietSince = Date.now();
	while (Date.now() - started < capMs) {
		await pause(2_000);
		const now = (await stats()).calls;
		if (now !== last) {
			last = now;
			quietSince = Date.now();
		} else if (now > 0 && Date.now() - quietSince >= quietMs) {
			return { timed_out: false, seconds: Math.round((Date.now() - started) / 1000) };
		}
	}
	return { timed_out: true, seconds: Math.round((Date.now() - started) / 1000) };
}

const TASKS = [
	{ id: 'a_email', message: 'BENCH-EMAIL to new.customer@bench.example: please send them our welcome email.' },
	{
		id: 'b_doc_work',
		message:
			'BENCH-DOC: please get a one-page delivery policy written (zones, fees, cut-off times) and let me review it.'
	},
	{ id: 'c_state_qa', message: 'BENCH-QA: what is in progress right now, and who is doing it?' },
	{
		id: 'd_team_work',
		message:
			'BENCH-TEAM: please have someone write a one-page delivery policy (zones, fees, cut-off times) and bring it to me.'
	},
	/* Two linked Work items (a draft, then the Work that requires it) while Exec said it would
	 * check back ("continue"): the wakes an intermediate result and a self-scheduled check cost. */
	{
		id: 'e_work_chain',
		message:
			'BENCH-CHAIN: please have a delivery policy drafted and then checked by someone else before it comes to me.'
	}
].filter((task) => !args.tasks || args.tasks.split(',').includes(task.id));

/* The baselines: the same tasks with the same scripted moves, as a direct harness would take them.
 *
 * - **Solo**: one agent, one session (Claude Code or Codex with no sub-agents). Team work is done
 *   by that one session: it writes and checks its own document.
 * - **Sub-agents**: one agent that hands work to sub-agents the way Claude Code's Task tool does.
 *   The parent's Task call blocks until its sub-agents report; several Task calls in one message
 *   run in parallel. Each sub-agent is a fresh session with its own fixed prefix (a sub-agent's
 *   system prompt and tool schemas, plus the project context), its brief as the first message, and
 *   its final text as the parent's tool result. Tasks with no work to hand off (a, b, c) run as in
 *   solo, since a direct harness would not spawn for them.
 *
 * Both run on the company's main model: a person running Claude Code on an Opus company's login
 * uses Opus for every turn. Each fixed prefix is cached for 5 minutes (1 hour on a sign-in), and
 * the conversation tail is cached, as Claude Code does. Every request names its scripted move to
 * the mock (`x-mock-step`), so its output tokens and modeled time are those of that move. */
const BASELINE_MODEL = (args['baseline-model'] ?? 'claude-sonnet-5-5').split('/').at(-1);
const BASELINE_SYSTEM_TOKENS = Number(args['baseline-system-tokens'] ?? 16_000);
const BASELINE_PROJECT_TOKENS = Number(args['baseline-project-tokens'] ?? 1_500);
/* A Claude Code sub-agent's fixed prefix: its agent prompt and the tool schemas it is given, without
 * the main session's own system prompt. An estimate, like the main prefix (README). */
const SUBAGENT_SYSTEM_TOKENS = Number(args['subagent-system-tokens'] ?? 12_000);
const filler = (tokens, seed) => `${seed} `.repeat(Math.ceil((tokens * 4) / (seed.length + 1))).slice(0, tokens * 4);
/* The same document Restless's scripted writer produces, so both sides write the same bytes. */
const POLICY =
	'# Delivery policy\n\nWe deliver within 10 km of the bakery. Orders placed by 8 pm arrive the next morning. Delivery is $5, free over $40.';
const WRITE = { tool: `mkdir -p outputs && cat > outputs/delivery-policy.md <<'D'\n${POLICY}\nD`, result: '' };
const READ = { tool: 'cat outputs/delivery-policy.md', result: POLICY };
const BASELINE_TASKS = {
	a_email: [
		{
			tool: `cat > /tmp/email.json <<'J'\n{"from":"hello@bench.example","to":"new.customer@bench.example","subject":"Welcome to Bench Bakery","text":"Hi, thanks for signing up. Your first loaf is on us this week."}\nJ\nsend-email --request-file /tmp/email.json`,
			result: '{"provider_id":"bench","status":"accepted"}'
		},
		{ text: 'Sent the welcome email to new.customer@bench.example.' }
	],
	b_doc_work: [WRITE, READ, { text: 'Wrote and reviewed the one-page delivery policy at outputs/delivery-policy.md for you to look at.' }],
	/* A direct harness has no team: the one session writes and checks it, as in b. */
	e_work_chain: [WRITE, READ, { text: 'Drafted and checked the one-page delivery policy at outputs/delivery-policy.md.' }],
	d_team_work: [WRITE, READ, { text: 'Wrote and checked the one-page delivery policy at outputs/delivery-policy.md.' }],
	c_state_qa: [
		{ tool: 'cat state/work.md state/people.md', result: '- Delivery policy (docs lead, in progress)\n- docs-writer: writer, Docs team lead' },
		{ text: 'One piece of work is open: the delivery policy, with the docs lead.' }
	]
};
const BRIEF = 'a one-page delivery policy (zones, fees, cut-off times) at outputs/delivery-policy.md';
const SUBAGENT_TASKS = {
	d_team_work: [
		{
			agents: [
				{
					name: 'writer',
					brief: `Write ${BRIEF}, check it against the brief, and report back.`,
					steps: [WRITE, READ, { text: 'Wrote and checked the delivery policy at outputs/delivery-policy.md: zones, fees and cut-off times are covered.' }]
				}
			]
		},
		{ text: 'The delivery policy is ready at outputs/delivery-policy.md, written and checked.' }
	],
	/* Drafted, then checked by someone else: the check needs the draft, so the two sub-agents run one
	 * after the other. Nothing in a-e can run in parallel; the parallel path is there for tasks that
	 * can (several `agents` in one step). */
	e_work_chain: [
		{
			agents: [
				{
					name: 'drafter',
					brief: `Draft ${BRIEF} and report back.`,
					steps: [WRITE, { text: 'Drafted the delivery policy at outputs/delivery-policy.md.' }]
				}
			]
		},
		{
			agents: [
				{
					name: 'reviewer',
					brief: 'Check outputs/delivery-policy.md against the brief (zones, fees, cut-off times) and report what is missing.',
					steps: [READ, { text: 'Checked outputs/delivery-policy.md: zones, fees and cut-off times are all there.' }]
				}
			]
		},
		{ text: 'The delivery policy is drafted, checked by a reviewer, and ready at outputs/delivery-policy.md.' }
	]
};
const base64 = (value) => Buffer.from(JSON.stringify(value)).toString('base64');
/* One scripted move as the mock should answer it. */
function mockStep(step) {
	if (step.text) return { text: step.text };
	if (step.tool) return { tool: { name: 'bash', input: { command: step.tool } } };
	return { tool: { name: 'Task', input: { agents: step.agents.map((agent) => agent.brief) } } };
}

/* One agent session: a fixed prefix, then the scripted steps, each one model call. Returns the
 * session's final text and what it did (the trace the quality check reads). */
async function session({ name, prefixTokens, seed, brief, steps, started }) {
	const system = [
		{ type: 'text', text: filler(prefixTokens, seed) },
		{ type: 'text', text: filler(BASELINE_PROJECT_TOKENS, 'project'), cache_control: { type: 'ephemeral', ...BASELINE_TTL } }
	];
	const tools = [
		{ name: 'bash', description: 'Run a command', input_schema: { type: 'object' } },
		{ name: 'Task', description: 'Hand a brief to sub-agents and wait for their reports', input_schema: { type: 'object' } }
	];
	const messages = [{ role: 'user', content: [{ type: 'text', text: brief }] }];
	const trace = [];
	const said = [];
	for (const step of steps) {
		const last = messages.at(-1);
		const content = last.content.map((block, index) =>
			index === last.content.length - 1 ? { ...block, cache_control: { type: 'ephemeral', ...BASELINE_TTL } } : block
		);
		const response = await fetch(`${mock}/v1/messages`, {
			method: 'POST',
			headers: { 'content-type': 'application/json', 'x-api-key': 'bench-baseline', 'x-mock-step': base64(mockStep(step)) },
			body: JSON.stringify({ model: BASELINE_MODEL, max_tokens: 4096, system, tools, messages: [...messages.slice(0, -1), { ...last, content }] })
		});
		assert(response.ok, `baseline call ${response.status}`);
		await response.json();
		if (step.text) {
			said.push({ text: step.text, at_ms: Date.now() - started });
			break;
		}
		const id = `toolu_${randomUUID().slice(0, 8)}`;
		if (step.tool) {
			messages.push({ role: 'assistant', content: [{ type: 'tool_use', id, name: 'bash', input: { command: step.tool } }] });
			trace.push({ actor: name, command: step.tool, result: step.result, at_ms: Date.now() - started });
			messages.push({ role: 'user', content: [{ type: 'tool_result', tool_use_id: id, content: step.result }] });
		} else {
			/* Task: the sub-agents run side by side; the parent waits for every report. */
			const reports = await Promise.all(
				step.agents.map((agent) =>
					session({ name: agent.name, prefixTokens: SUBAGENT_SYSTEM_TOKENS, seed: 'subagent', brief: agent.brief, steps: agent.steps, started })
				)
			);
			const ids = step.agents.map(() => `toolu_${randomUUID().slice(0, 8)}`);
			messages.push({
				role: 'assistant',
				content: step.agents.map((agent, index) => ({ type: 'tool_use', id: ids[index], name: 'Task', input: { brief: agent.brief } }))
			});
			messages.push({
				role: 'user',
				content: reports.map((report, index) => ({ type: 'tool_result', tool_use_id: ids[index], content: report.said.at(-1)?.text ?? '' }))
			});
			for (const report of reports) trace.push(...report.trace);
			step.agents.forEach((agent) => trace.push({ actor: name, delegated_to: agent.name, brief: agent.brief, at_ms: Date.now() - started }));
		}
	}
	return { trace, said };
}

/* A baseline run of one task: tokens from the mock, time from this clock (with `--latency`, the
 * mock holds each answer for its modeled time, so this is modeled model time plus a few ms). The
 * owner sees nothing until the session's final text: a direct harness is busy until it is done. */
async function baseline(task, kind) {
	const steps = kind === 'subagents' ? (SUBAGENT_TASKS[task.id] ?? BASELINE_TASKS[task.id]) : BASELINE_TASKS[task.id];
	await stats(true);
	const started = Date.now();
	/* The two baselines keep apart caches, so the second does not read the prefix the first wrote. */
	const seed = kind === 'subagents' ? 'parent' : 'harness';
	const run = await session({ name: 'agent', prefixTokens: BASELINE_SYSTEM_TOKENS, seed, brief: `BASELINE ${task.message}`, steps, started });
	const finished = run.said.at(-1)?.at_ms ?? Date.now() - started;
	const owner = run.said.map(({ text, at_ms }) => ({ text, at_ms }));
	return {
		...(await stats()),
		timing: { finished_s: seconds(finished), first_reply_s: seconds(owner[0]?.at_ms ?? finished), free_s: 0 },
		quality: quality(task.id, { trace: run.trace, owner, assignments: assignmentsOf(run.trace) })
	};
}


/* Cost-weighted input, in uncached-token equivalents at Anthropic's prices: a cache read is 0.1x,
 * a cache write 1.25x (5-minute) to 2x (1-hour); 1.25x is used, which flatters neither side. */
const weighted = (row) =>
	row.input_tokens + 0.1 * row.cache_read_input_tokens + 1.25 * row.cache_creation_input_tokens;
const total = (row) => row.input_tokens + row.cache_read_input_tokens + row.cache_creation_input_tokens;

/* What each model costs, per million tokens: the models.dev catalogue Restless prices calls with,
 * read once per run. The fallback is that catalogue on 4 November 2026, so a run without network
 * still prices the models it knows. A one-hour cache write is 2x input, as Anthropic charges. On a
 * subscription route nothing is billed per token: the same weights are the share of the plan a run
 * uses, on the assumption that a plan's usage windows weigh models by their API price (providers
 * do not publish the exact weights). */
const FALLBACK_PRICES = {
	'claude-opus-5-5': { input: 4, output: 20, cache_read: 0.2, cache_write: 5 },
	'claude-sonnet-5-5': { input: 2, output: 10, cache_read: 0.1, cache_write: 2.5 },
	'claude-haiku-5-5': { input: 0.1, output: 0.5, cache_read: 0.01, cache_write: 0.125 },
	'gpt-6-sol': { input: 2, output: 10, cache_read: 0.2, cache_write: 2.5 },
	'gpt-6-terra': { input: 1, output: 5, cache_read: 0.05, cache_write: 1.25 }
};
async function priceTable() {
	const table = { ...FALLBACK_PRICES };
	try {
		const response = await fetch('https://models.dev/api.json', { signal: AbortSignal.timeout(15_000) });
		const catalog = await response.json();
		for (const provider of ['anthropic', 'openai'])
			for (const [id, model] of Object.entries(catalog[provider]?.models ?? {}))
				if (model.cost?.input != null && model.cost?.output != null) table[id] = model.cost;
		return { table, source: 'models.dev' };
	} catch {
		return { table, source: 'catalogue snapshot of 4 November 2026' };
	}
}
const PRICES = await priceTable();
/* The bare model id a request named, whichever route sent it (`native-claude-oauth/x`, `x`). */
const bareModel = (model) => String(model).split('/').at(-1);
/* One model's usage in dollars at its catalogue rates, with the weighted input beside it. */
function priced(model, usage) {
	const rate = PRICES.table[bareModel(model)];
	const oneHour = usage.cache_creation_1h_input_tokens ?? 0;
	const fiveMinute = usage.cache_creation_input_tokens - oneHour;
	const row = {
		calls: usage.calls,
		weighted_input: Math.round(weighted(usage)),
		output_tokens: usage.output_tokens,
		usd: null
	};
	if (!rate) return row;
	const write = rate.cache_write ?? 1.25 * rate.input;
	row.usd =
		(usage.input_tokens * rate.input +
			usage.cache_read_input_tokens * (rate.cache_read ?? rate.input) +
			fiveMinute * write +
			oneHour * 2 * rate.input +
			usage.output_tokens * rate.output) /
		1_000_000;
	return row;
}
/* A side's cost by model and in all; `usd` is null when a model it used has no price. */
function costOf(side) {
	const byModel = Object.fromEntries(
		Object.entries(side.usage_by_model ?? {}).map(([model, usage]) => [model, priced(model, usage)])
	);
	const rows = Object.values(byModel);
	return {
		by_model: byModel,
		weighted_input: rows.reduce((sum, row) => sum + row.weighted_input, 0),
		output_tokens: rows.reduce((sum, row) => sum + row.output_tokens, 0),
		usd: rows.every((row) => row.usd != null)
			? Math.round(rows.reduce((sum, row) => sum + row.usd, 0) * 1e6) / 1e6
			: null
	};
}

/* Which actor made a logged call, from its system prompt: Exec, a team lead, or Staff (a Work
 * Attempt or a member's conversation). */
const systemText = (body) =>
	typeof body.system === 'string'
		? body.system
		: (body.system ?? []).map((block) => block.text ?? '').join('\n');
function actorOf(body) {
	const system = systemText(body);
	if (/\bYou are (the )?Exec\b|# Exec\b|Restless Exec/i.test(system)) return 'exec';
	if (/team lead|You lead the|lead's contract|# Lead\b/i.test(system)) return 'lead';
	return 'staff';
}
/* A call that starts a turn (a wake): its newest user message is new input (text, not tool
 * results) and not the Runtime's end-of-turn decision question. */
function startsTurn(body) {
	const last = (body.messages ?? []).findLast((message) => message.role === 'user');
	if (!last) return false;
	const blocks = Array.isArray(last.content) ? last.content : [{ type: 'text', text: last.content }];
	const text = blocks
		.filter((block) => block.type === 'text')
		.map((block) => block.text)
		.join('\n');
	if (blocks.some((block) => block.type === 'tool_result') && !text.trim()) return false;
	return !/answer with JSON only|Required keys: decision, reason|^Continue\.$/i.test(text.trim());
}
/* Bytes each part of a request carried (system prompt, tool schemas, conversation), per actor:
 * where the re-sent volume is. */
async function payloads(from) {
	if (!callLog) return null;
	const { readFile } = await import('node:fs/promises');
	const lines = (await readFile(callLog, 'utf8').catch(() => '')).trim().split('\n').filter(Boolean);
	const calls = lines
		.slice(from)
		.map((line) => JSON.parse(line))
		.filter((call) => call.method === 'POST' && call.path.endsWith('/messages') && call.usage);
	const by = {};
	for (const { body, usage } of calls) {
		const row = (by[actorOf(body)] ??= {
			calls: 0,
			wakes: 0,
			input_tokens: 0,
			system: 0,
			tools: 0,
			messages: 0,
			models: {}
		});
		row.calls += 1;
		row.wakes += Number(startsTurn(body));
		row.input_tokens +=
			usage.input_tokens + usage.cache_read_input_tokens + usage.cache_creation_input_tokens;
		row.system += Buffer.byteLength(JSON.stringify(body.system ?? ''));
		row.tools += Buffer.byteLength(JSON.stringify(body.tools ?? []));
		row.messages += Buffer.byteLength(JSON.stringify(body.messages ?? []));
		row.models[body.model] = (row.models[body.model] ?? 0) + 1;
	}
	for (const row of Object.values(by)) {
		for (const part of ['system', 'tools', 'messages']) row[`${part}_per_call`] = Math.round(row[part] / row.calls);
		row.bytes_per_call = row.system_per_call + row.tools_per_call + row.messages_per_call;
		row.input_tokens_per_wake = row.wakes ? Math.round(row.input_tokens / row.wakes) : null;
		delete row.system;
		delete row.tools;
		delete row.messages;
	}
	return { lines: lines.length, by_actor: by };
}
async function logLength() {
	return (await payloads(0))?.lines ?? 0;
}

/* Restless's own record of each call since `since`, by actor: the relay knows which actor and
 * responsibility made a call, which the provider's request does not say. */
async function byActor(since) {
	const response = await fetch(`${api}/company/spend/report?days=1`, { headers: { Origin: origin } });
	if (!response.ok) return null;
	const report = await response.json();
	const by = {};
	for (const row of report.activity ?? []) {
		if (new Date(row.occurred_at) < since) continue;
		const kind = row.actor === 'exec' ? 'exec' : row.work_id ? `work:${row.actor}` : `chat:${row.actor}`;
		const actor = (by[kind] ??= { calls: 0, input_tokens: 0, cached_input_tokens: 0, models: {} });
		actor.calls += 1;
		actor.input_tokens += row.input_tokens ?? 0;
		actor.cached_input_tokens += row.cached_input_tokens ?? 0;
		actor.models[row.model] = (actor.models[row.model] ?? 0) + 1;
	}
	for (const actor of Object.values(by))
		actor.input_tokens_per_call = Math.round(actor.input_tokens / actor.calls);
	return { by, billing: [...new Set((report.activity ?? []).map((row) => row.billing))] };
}

const seconds = (ms) => (ms == null ? null : Math.round(ms / 100) / 10);

/* The quality check: a scripted rubric over what each side actually did, the same for all three.
 * It reads the commands that ran with their results (for Restless the real results from the
 * company's computer, read back from the mock's request log; for the baselines their scripted
 * results) and the messages the owner got. The model is scripted, so this checks that the work
 * reached the owner complete and by the route the owner asked for, not how well a model writes. */
const WROTE = /cat\s*>\s*\S*delivery-policy\.md/;
const READS = /\bcat\s+[^>\s]*delivery-policy\.md/;
const complete = (text) =>
	/\b\d+\s?km\b/i.test(text) && /\$\d+/.test(text) && /\b\d{1,2}\s?(am|pm)\b/i.test(text);
const ERROR = /error|denied|refused|not allowed|failed|blocked/i;
function quality(taskId, { trace, owner, assignments }) {
	const steps = [...trace].filter((step) => step.command).sort((a, b) => a.at_ms - b.at_ms);
	const wrote = steps.findIndex((step) => WROTE.test(step.command));
	const read = wrote < 0 ? undefined : steps.slice(wrote + 1).find((step) => READS.test(step.command) && /# Delivery policy/.test(step.result ?? ''));
	const delivered = (after) => owner.some((message) => /delivery policy/i.test(message.text) && message.at_ms >= (after?.at_ms ?? Infinity));
	const policy = {
		'policy has zones, fees and cut-off times': !!read && complete(read.result),
		'read back after it was written': !!read
	};
	const checks = {
		a_email: {
			'email sent to the customer and accepted': steps.some(
				(step) => /email send|send-email/.test(step.command) && step.command.includes('new.customer@bench.example') && step.result != null && !ERROR.test(step.result)
			),
			'owner told': owner.some((message) => message.text.includes('new.customer@bench.example'))
		},
		b_doc_work: { ...policy, 'handed to the owner to review': delivered(read) },
		c_state_qa: {
			'owner answered': owner.length > 0,
			'answer names the open work': owner.some((message) => /delivery policy/i.test(message.text))
		},
		d_team_work: { ...policy, 'brought to the owner after it was checked': delivered(read) },
		e_work_chain: {
			...policy,
			'checked by someone other than its drafter': !!assignments.checker && assignments.checker !== assignments.drafter,
			'brought to the owner after the check': delivered(read)
		}
	}[taskId];
	/* What the rubric read, so a failure can be traced without rerunning the bench. */
	const evidence = {
		commands: steps.map((step) => ({
			actor: step.actor,
			at_s: seconds(step.at_ms),
			command: step.command.slice(0, 160),
			result: String(step.result ?? '').slice(0, 240)
		})),
		owner: owner.map((message) => ({ at_s: seconds(message.at_ms), text: message.text.slice(0, 240) }))
	};
	return { pass: Object.values(checks).every(Boolean), checks, assignments, evidence };
}

/* Who drafted the policy and who checked it. A baseline's trace names the session of each command.
 * Restless's commands name it: the lead a commission goes to drafts in its Work, and a check added
 * as its own Work (`work add --owner … --requires`) belongs to that Work's owner; a check inside
 * the drafting Work is the drafter's. */
function assignmentsOf(trace) {
	const steps = [...trace].filter((step) => step.command).sort((a, b) => a.at_ms - b.at_ms);
	const lead = steps.map((step) => /restless commission\b.*?--lead\s+'?([\w-]+)/s.exec(step.command)?.[1]).find(Boolean);
	if (lead) {
		const check = steps
			.map((step) => /restless work add\b.*?--owner\s+'?([\w-]+).*--requires/s.exec(step.command)?.[1])
			.find(Boolean);
		return { drafter: lead, checker: check ?? lead };
	}
	const wrote = steps.findIndex((step) => WROTE.test(step.command));
	if (wrote < 0) return {};
	return {
		drafter: steps[wrote].actor,
		checker: steps.slice(wrote + 1).find((step) => READS.test(step.command))?.actor ?? null
	};
}

/* What Restless did in a task, from the mock's request log: every tool call with its actor and
 * its real result, timed by the first request that carried the result back. Calls seen before the
 * task (an Exec session's history) are left out. */
function restlessTrace(calls, from, started) {
	const before = new Set();
	const uses = new Map();
	const results = new Map();
	const resultText = (content) =>
		typeof content === 'string' ? content : (content ?? []).map((block) => block.text ?? '').join('\n');
	calls.forEach(({ at, body }, index) => {
		const actor = actorOf(body);
		for (const message of body.messages ?? []) {
			if (!Array.isArray(message.content)) continue;
			for (const block of message.content) {
				if (block.type === 'tool_use') {
					if (index < from) before.add(block.id);
					else if (!uses.has(block.id))
						uses.set(block.id, { actor, command: block.input?.command ?? JSON.stringify(block.input ?? {}) });
				}
				if (block.type === 'tool_result' && index >= from && !results.has(block.tool_use_id))
					results.set(block.tool_use_id, { result: resultText(block.content), at_ms: Date.parse(at) - started });
			}
		}
	});
	return [...uses]
		.filter(([id]) => !before.has(id) && results.has(id))
		.map(([id, use]) => ({ ...use, ...results.get(id) }));
}

/* The Exec's messages to the owner since the task began, timed by when Restless stored them. */
async function ownerMessages(started) {
	const response = await fetch(`${api}/actors/exec/conversation`, { headers: { Origin: origin } });
	if (!response.ok) return [];
	const view = await response.json();
	return (view.messages ?? [])
		.filter((message) => message.from_actor === 'exec' && Date.parse(message.created_at) >= started)
		.map((message) => ({ text: message.body ?? '', at_ms: Date.parse(message.created_at) - started }))
		.sort((a, b) => a.at_ms - b.at_ms);
}

/* Every model call in the request log, with where it sits in the log. */
async function readCalls() {
	if (!callLog) return [];
	const { readFile } = await import('node:fs/promises');
	return (await readFile(callLog, 'utf8').catch(() => ''))
		.trim()
		.split('\n')
		.filter(Boolean)
		.map((line, index) => ({ ...JSON.parse(line), line: index }))
		.filter((call) => call.method === 'POST' && call.path.endsWith('/messages') && call.usage);
}

const results = [];
for (const task of TASKS) {
	await stats(true);
	const startedAt = new Date();
	const started = startedAt.getTime();
	const logged = await logLength();
	await tellExec(task.message);
	const run = await settle();
	const restless = await stats();
	const sent = await payloads(logged);
	const actors = await byActor(startedAt).catch(() => null);
	/* Time to the result: the Exec's last message to the owner in the task (the result, or the
	 * answer). First reply: its first, which on team work comes while the team is still working;
	 * the gap between them is how long the owner had an Exec free to talk to. */
	const calls = await readCalls();
	const from = calls.findIndex((call) => call.line >= logged);
	const window = from < 0 ? [] : calls.slice(from);
	const trace = from < 0 ? [] : restlessTrace(calls, from, started);
	const owner = await ownerMessages(started);
	const lastCall = window.length ? Math.max(...window.map((call) => Date.parse(call.at) - started)) : null;
	const finished = owner.at(-1)?.at_ms ?? lastCall;
	const timing = {
		finished_s: seconds(finished),
		first_reply_s: seconds(owner[0]?.at_ms ?? null),
		free_s: owner.length ? seconds(finished - owner[0].at_ms) : 0,
		last_call_s: seconds(lastCall),
		owner_messages: owner.length
	};
	const checked = quality(task.id, { trace, owner, assignments: assignmentsOf(trace) });
	const direct = await baseline(task, 'solo');
	const delegated = await baseline(task, 'subagents');
	results.push({
		task: task.id,
		restless: {
			...restless,
			...run,
			timing,
			quality: checked,
			payloads: sent?.by_actor ?? null,
			actors: actors?.by ?? null,
			billing: actors?.billing ?? null
		},
		baseline: direct,
		subagents: delegated
	});
	console.error(
		`${task.id}: restless ${restless.calls} calls ${timing.finished_s}s ${checked.pass ? 'pass' : 'FAIL'}, ` +
			`solo ${direct.calls} calls ${direct.timing.finished_s}s, sub-agents ${delegated.calls} calls ${delegated.timing.finished_s}s`
	);
}

const ratio = (a, b) => (a != null && b ? Math.round((a / b) * 100) / 100 : null);
const ratios = (restless, restlessCost, side, sideCost) => ({
	calls: ratio(restless.calls, side.calls),
	input_total: ratio(total(restless), total(side)),
	input_weighted: ratio(weighted(restless), weighted(side)),
	output: ratio(restless.output_tokens, side.output_tokens),
	cost: ratio(restlessCost.usd ?? 0, sideCost.usd ?? 0),
	wall_clock: ratio(restless.timing.finished_s, side.timing.finished_s)
});
const report = {
	at: new Date().toISOString(),
	company,
	route,
	quiet_ms: quietMs,
	/* The mock's latency model (`--latency`), or null when answers came at once. */
	latency: args.latency && args.latency !== 'off' ? args.latency : null,
	baseline_fixed_tokens: BASELINE_SYSTEM_TOKENS + BASELINE_PROJECT_TOKENS,
	subagent_fixed_tokens: SUBAGENT_SYSTEM_TOKENS + BASELINE_PROJECT_TOKENS,
	baseline_model: BASELINE_MODEL,
	/* Dollars on an API key; on a sign-in the same figure is the plan share (see priceTable). */
	cost_unit: route === 'subscription' ? 'plan share (API-price weights)' : 'USD at API rates',
	prices_from: PRICES.source,
	tasks: results.map(({ task, restless, baseline, subagents }) => {
		const restlessCost = costOf(restless);
		const baselineCost = costOf(baseline);
		const subagentsCost = costOf(subagents);
		return {
			task,
			restless: { ...restless, cost: restlessCost },
			baseline: { ...baseline, cost: baselineCost },
			subagents: { ...subagents, cost: subagentsCost },
			/* Restless over the solo baseline (the token-parity status reads these). */
			ratio: ratios(restless, restlessCost, baseline, baselineCost),
			/* Restless over the agent-with-sub-agents baseline. */
			ratio_subagents: ratios(restless, restlessCost, subagents, subagentsCost)
		};
	})
};
const share = (cost) =>
	Object.entries(cost.by_model)
		.map(([model, row]) => `${bareModel(model)} ${row.calls}`)
		.join(', ');
const money = (value) => (value == null ? 'unpriced' : `$${value.toFixed(4)}`);
const secs = (value) => (value == null ? '-' : `${value}`);
const verdict = (checked) =>
	checked.pass
		? 'pass'
		: `FAIL: ${Object.entries(checked.checks)
				.filter(([, ok]) => !ok)
				.map(([name]) => name)
				.join('; ')}`;
const sides = (row) => [
	['Restless', row.restless],
	['Solo agent', row.baseline],
	['Agent + sub-agents', row.subagents]
];
const table = [
	'| Task | Restless calls | Restless input (uncached/read/write) | Baseline calls | Baseline input (uncached/read/write) | Calls ratio | Input ratio | Weighted ratio |',
	'| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |',
	...report.tasks.map(
		({ task, restless: r, baseline: b, ratio: x }) =>
			`| ${task} | ${r.calls} | ${total(r)} (${r.input_tokens}/${r.cache_read_input_tokens}/${r.cache_creation_input_tokens}) | ${b.calls} | ${total(b)} (${b.input_tokens}/${b.cache_read_input_tokens}/${b.cache_creation_input_tokens}) | ${x.calls} | ${x.input_total} | ${x.input_weighted} |`
	),
	'',
	`Cost by model (${report.cost_unit}; prices from ${report.prices_from}; baseline on ${BASELINE_MODEL}):`,
	'',
	'| Task | Restless calls by model | Restless weighted input / output | Restless cost | Baseline weighted input / output | Baseline cost | Cost ratio |',
	'| --- | --- | ---: | ---: | ---: | ---: | ---: |',
	...report.tasks.map(
		({ task, restless: { cost: r }, baseline: { cost: b }, ratio: x }) =>
			`| ${task} | ${share(r)} | ${r.weighted_input} / ${r.output_tokens} | ${money(r.usd)} | ${b.weighted_input} / ${b.output_tokens} | ${money(b.usd)} | ${x.cost} |`
	),
	'',
	`Tradeoffs against a solo agent and an agent with sub-agents (time in seconds; ${
		report.latency ? `modeled model latency: ${report.latency}` : 'no model latency: answers came at once'
	}):`,
	'',
	'| Task | Side | Calls | Weighted input | Cost | Time to result | First owner reply | Exec free while working | Quality |',
	'| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |',
	...report.tasks.flatMap((row) =>
		sides(row).map(
			([side, s]) =>
				`| ${row.task} | ${side} | ${s.calls} | ${Math.round(weighted(s))} | ${money(s.cost.usd)} | ${secs(s.timing.finished_s)} | ${secs(s.timing.first_reply_s)} | ${secs(s.timing.free_s)} | ${verdict(s.quality)} |`
		)
	),
	'',
	'| Task | Weighted input vs solo | vs sub-agents | Cost vs solo | vs sub-agents | Time vs solo | vs sub-agents |',
	'| --- | ---: | ---: | ---: | ---: | ---: | ---: |',
	...report.tasks.map(
		({ task, ratio: x, ratio_subagents: y }) =>
			`| ${task} | ${x.input_weighted} | ${y.input_weighted} | ${x.cost} | ${y.cost} | ${x.wall_clock} | ${y.wall_clock} |`
	)
].join('\n');
report.table = table;
if (args.out) await writeFile(args.out, JSON.stringify(report, null, 2));
console.log(table);
