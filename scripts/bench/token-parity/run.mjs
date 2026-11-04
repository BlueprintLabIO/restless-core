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
 * Tasks: (a) send one templated email; (b) a small Work item, a one-page document from a brief with
 * its review; (c) an owner question about company state. */
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
	}
];

/* The baseline: one agent, one session, the same scripted moves a direct Claude Code run takes. Its
 * fixed prefix is a Claude Code-sized system prompt plus tool schemas, cached for 5 minutes as
 * Claude Code does, with the conversation's tail cached too. */
const BASELINE_SYSTEM_TOKENS = Number(args['baseline-system-tokens'] ?? 16_000);
const BASELINE_PROJECT_TOKENS = Number(args['baseline-project-tokens'] ?? 1_500);
const filler = (tokens, seed) => `${seed} `.repeat(Math.ceil((tokens * 4) / (seed.length + 1))).slice(0, tokens * 4);
const BASELINE_TASKS = {
	a_email: [
		{ tool: 'cat > /tmp/email.json <<J … J && send-email --request-file /tmp/email.json', result: '{"provider_id":"bench","status":"accepted"}' },
		{ text: 'Sent the welcome email to new.customer@bench.example.' }
	],
	b_doc_work: [
		{ tool: 'cat > outputs/delivery-policy.md <<D … D', result: '' },
		{ tool: 'cat outputs/delivery-policy.md', result: '# Delivery policy\n\nWe deliver within 10 km of the bakery. Orders placed by 8 pm arrive the next morning. Delivery is $5, free over $40.' },
		{ text: 'Wrote and reviewed the one-page delivery policy at outputs/delivery-policy.md.' }
	],
	/* A direct harness has no team: the one session writes and checks it, as in b. */
	d_team_work: [
		{ tool: 'cat > outputs/delivery-policy.md <<D … D', result: '' },
		{ tool: 'cat outputs/delivery-policy.md', result: '# Delivery policy\n\nWe deliver within 10 km of the bakery. Orders placed by 8 pm arrive the next morning. Delivery is $5, free over $40.' },
		{ text: 'Wrote and checked the one-page delivery policy at outputs/delivery-policy.md.' }
	],
	c_state_qa: [
		{ tool: 'cat state/work.md state/people.md', result: '- Delivery policy (docs lead, in progress)\n- docs-writer: writer, Docs team lead' },
		{ text: 'One piece of work is open: the delivery policy, with the docs lead.' }
	]
};
async function baseline(task) {
	const system = [
		{ type: 'text', text: filler(BASELINE_SYSTEM_TOKENS, 'harness') },
		{
			type: 'text',
			text: filler(BASELINE_PROJECT_TOKENS, 'project'),
			cache_control: { type: 'ephemeral', ...BASELINE_TTL }
		}
	];
	const messages = [{ role: 'user', content: [{ type: 'text', text: `BASELINE ${task.message}` }] }];
	await stats(true);
	for (const step of BASELINE_TASKS[task.id]) {
		const last = messages.at(-1);
		const content = last.content.map((block, index) =>
			index === last.content.length - 1
				? { ...block, cache_control: { type: 'ephemeral', ...BASELINE_TTL } }
				: block
		);
		const response = await fetch(`${mock}/v1/messages`, {
			method: 'POST',
			headers: { 'content-type': 'application/json', 'x-api-key': 'bench-baseline' },
			body: JSON.stringify({
				model: 'claude-sonnet-5-5',
				max_tokens: 4096,
				system,
				tools: [{ name: 'bash', description: 'Run a command', input_schema: { type: 'object' } }],
				messages: [...messages.slice(0, -1), { ...last, content }]
			})
		});
		assert(response.ok, `baseline call ${response.status}`);
		await response.json();
		if (step.tool) {
			const id = `toolu_${randomUUID().slice(0, 8)}`;
			messages.push({ role: 'assistant', content: [{ type: 'tool_use', id, name: 'bash', input: { command: step.tool } }] });
			messages.push({ role: 'user', content: [{ type: 'tool_result', tool_use_id: id, content: step.result }] });
		}
	}
	return stats();
}

/* Cost-weighted input, in uncached-token equivalents at Anthropic's prices: a cache read is 0.1x,
 * a cache write 1.25x (5-minute) to 2x (1-hour); 1.25x is used, which flatters neither side. */
const weighted = (row) =>
	row.input_tokens + 0.1 * row.cache_read_input_tokens + 1.25 * row.cache_creation_input_tokens;
const total = (row) => row.input_tokens + row.cache_read_input_tokens + row.cache_creation_input_tokens;

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

const results = [];
for (const task of TASKS) {
	await stats(true);
	const started = new Date();
	const logged = await logLength();
	await tellExec(task.message);
	const run = await settle();
	const restless = await stats();
	const sent = await payloads(logged);
	const actors = await byActor(started).catch(() => null);
	const direct = await baseline(task);
	results.push({
		task: task.id,
		restless: {
			...restless,
			...run,
			payloads: sent?.by_actor ?? null,
			actors: actors?.by ?? null,
			billing: actors?.billing ?? null
		},
		baseline: direct
	});
	console.error(`${task.id}: restless ${restless.calls} calls, baseline ${direct.calls} calls`);
}

const ratio = (a, b) => (b ? Math.round((a / b) * 100) / 100 : null);
const report = {
	at: new Date().toISOString(),
	company,
	route,
	quiet_ms: quietMs,
	baseline_fixed_tokens: BASELINE_SYSTEM_TOKENS + BASELINE_PROJECT_TOKENS,
	tasks: results.map(({ task, restless, baseline }) => ({
		task,
		restless,
		baseline,
		ratio: {
			calls: ratio(restless.calls, baseline.calls),
			input_total: ratio(total(restless), total(baseline)),
			input_weighted: ratio(weighted(restless), weighted(baseline)),
			output: ratio(restless.output_tokens, baseline.output_tokens)
		}
	}))
};
const table = [
	'| Task | Restless calls | Restless input (uncached/read/write) | Baseline calls | Baseline input (uncached/read/write) | Calls ratio | Input ratio | Weighted ratio |',
	'| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |',
	...report.tasks.map(
		({ task, restless: r, baseline: b, ratio: x }) =>
			`| ${task} | ${r.calls} | ${total(r)} (${r.input_tokens}/${r.cache_read_input_tokens}/${r.cache_creation_input_tokens}) | ${b.calls} | ${total(b)} (${b.input_tokens}/${b.cache_read_input_tokens}/${b.cache_creation_input_tokens}) | ${x.calls} | ${x.input_total} | ${x.input_weighted} |`
	)
].join('\n');
report.table = table;
if (args.out) await writeFile(args.out, JSON.stringify(report, null, 2));
console.log(table);
