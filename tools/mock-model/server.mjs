#!/usr/bin/env node
// A scripted stand-in for a model provider, so every LLM feature can be exercised with no model
// connection: streaming replies, tool calls, failures and slow answers, decided by a scenario file.
//
// It speaks the Anthropic Messages API, which the model gateway relays to when
// RESTLESS_ANTHROPIC_BASE_URL is set. Start it, point a dev stack at it, and give the company an
// Anthropic connection with any key (the key check uses the same base URL):
//
//   node tools/mock-model/server.mjs --port 18080 --scenario tools/mock-model/scenarios/default.json
//   RESTLESS_ANTHROPIC_BASE_URL=http://127.0.0.1:18080/v1 scripts/restless-dev <namespace>
//
// A scenario is a list of rules; of those whose `match` (a case-insensitive regular expression)
// matches the latest person's text, the one matching latest in it answers. A rule has `steps`, run one per model call: a step is
// `{ "tool": { "name": "...", "input": {...} } }`, which asks for a tool call, or `{ "text": "..." }`,
// which ends the turn. The step taken is the number of tool results since that person's text, so
// a rule with a tool step then a text step calls the tool, gets its result, then replies. A step may
// also carry `delay_ms` (between streamed pieces), `error` (an HTTP status to answer with) and
// `message` (that error's text). With no matching rule the mock answers "Mock reply: <their text>".
// A tool step's strings may name the rule's capture groups as `{{1}}`, `{{2}}`…, taken from its
// latest match, so a check can make Exec cite the very message that matched ("message (\\d+)…").
// Every request is appended to --log as one JSON line, for checks to read.
import { createServer } from 'node:http';
import { readFileSync, appendFileSync } from 'node:fs';
import { randomUUID, createHash } from 'node:crypto';

const args = Object.fromEntries(
	process.argv.slice(2).reduce((pairs, value, index, all) => {
		if (value.startsWith('--')) pairs.push([value.slice(2), all[index + 1]]);
		return pairs;
	}, [])
);
const port = Number(args.port ?? 18080);
const scenarioPath = args.scenario;
const log = args.log;
const MODELS = ['claude-sonnet-5-5', 'claude-opus-5-5', 'claude-haiku-4-5-20251001'];

/* Re-read on every request, so a check can change the scenario while the stack runs. */
function scenario() {
	if (!scenarioPath) return [];
	try {
		return JSON.parse(readFileSync(scenarioPath, 'utf8')).rules ?? [];
	} catch (error) {
		console.error(`mock-model: scenario unreadable: ${error.message}`);
		return [];
	}
}

const textOf = (content) =>
	typeof content === 'string'
		? content
		: (content ?? [])
				.filter((block) => block.type === 'text')
				.map((block) => block.text)
				.join('\n');

/* The latest person's words, and how many tool results came back after them. */
function turnState(messages) {
	let toolResults = 0;
	for (let index = messages.length - 1; index >= 0; index--) {
		const message = messages[index];
		if (message.role !== 'user') continue;
		const blocks = Array.isArray(message.content) ? message.content : [];
		const results = blocks.filter((block) => block.type === 'tool_result').length;
		const text = textOf(message.content).trim();
		if (results && !text) {
			toolResults += results;
			continue;
		}
		return { text, toolResults };
	}
	return { text: '', toolResults };
}

/* The runtime ends each turn by asking for a JSON decision ("The turn is ending now … answer
 * with JSON only", or for Work "Required keys: decision, reason"). Answer it the way a model
 * that follows the protocol does, so a good reply is not reported as "couldn't reply". A scenario
 * may set `completion` to choose the decision; otherwise "waiting" where it is offered (Exec),
 * else "outcome_met" (Work). */
function completionFor(text) {
	if (!/answer with JSON only|Required keys: decision, reason/i.test(text)) return null;
	const chosen = scenarioFile().completion;
	const decision =
		chosen?.decision ?? (/\bwaiting\b/.test(text) ? 'waiting' : 'outcome_met');
	return {
		text: JSON.stringify({ decision, reason: chosen?.reason ?? 'The mock turn finished.' })
	};
}

function scenarioFile() {
	if (!scenarioPath) return {};
	try {
		return JSON.parse(readFileSync(scenarioPath, 'utf8'));
	} catch {
		return {};
	}
}

/* A rule with `"scope": "all"` also reads the system prompt (ahead of the person's text), for an
 * actor whose assignment sits there rather than in its first message. */
function chooseStep(body) {
	const messages = body.messages ?? [];
	const { text, toolResults } = turnState(messages);
	const completion = completionFor(text);
	if (completion) return { step: completion, text };
	const system = typeof body.system === 'string' ? body.system : textOf(body.system);
	/* Exec's prompt carries older messages too, so the rule whose words appear latest wins: the
	 * newest message decides, not the first rule an old one happens to match. */
	let chosen = null;
	let groups = [];
	let at = -1;
	for (const rule of scenario()) {
		const pattern = new RegExp(rule.match ?? '.*', 'gi');
		const searched = rule.scope === 'all' ? `${system}\n${text}` : text;
		let match;
		let last = -1;
		let lastGroups = [];
		while ((match = pattern.exec(searched))) {
			last = match.index + (rule.scope === 'all' ? 0 : system.length + 1);
			lastGroups = [...match];
			if (match[0] === '') pattern.lastIndex++;
		}
		if (last > at) {
			at = last;
			chosen = rule;
			groups = lastGroups;
		}
	}
	if (chosen) {
		const steps = chosen.steps ?? [{ text: chosen.reply ?? '' }];
		return { step: withGroups(steps[Math.min(toolResults, steps.length - 1)], groups), text };
	}
	return { step: { text: `Mock reply: ${text.slice(0, 400)}` }, text };
}

/* `{{N}}` in a tool step's strings becomes the rule's Nth capture group. */
function withGroups(step, groups) {
	if (!step.tool || !groups.length) return step;
	const fill = (value) =>
		typeof value === 'string'
			? value.replace(/\{\{(\d+)\}\}/g, (_, n) => groups[Number(n)] ?? '')
			: Array.isArray(value)
				? value.map(fill)
				: value && typeof value === 'object'
					? Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, fill(inner)]))
					: value;
	return { ...step, tool: fill(step.tool) };
}

const words = (text) => text.match(/\S+\s*/g) ?? [''];
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const tokens = (text) => Math.ceil(String(text ?? '').length / 4);

/* Realistic accounting, so a benchmark can count what a run would cost. Tokens are estimated at
 * 4 characters each over the request as the provider sees it (tools, then system, then messages).
 * Caching follows the Anthropic rules: a block marked `cache_control` ends a cacheable prefix; a
 * later request whose prefix up to a breakpoint is byte-identical, within that breakpoint's TTL
 * (5 minutes, or 1 hour for `ttl: "1h"`), reads it from cache; the rest of the prefix up to the
 * last breakpoint is written. Totals are served at GET /stats and reset by DELETE /stats. */
const cache = new Map(); // prefix hash -> expiry (ms)
const freshStats = () => ({
	calls: 0,
	input_tokens: 0,
	cache_read_input_tokens: 0,
	cache_creation_input_tokens: 0,
	cache_creation_1h_input_tokens: 0,
	output_tokens: 0,
	request_bytes: 0,
	by_model: {},
	ttl_breakpoints: {}
});
let stats = freshStats();
/* A Claude sign-in request (OMP's gateway in Claude Code mode) opens its system prompt with an
 * `x-anthropic-billing-header:` block whose attestation differs per request. Anthropic does not let
 * it break the prompt cache (Claude Code's own caching depends on that), so it is left out here. */
const BILLING_HEADER = 'x-anthropic-billing-header:';
function prefixBlocks(body) {
	const blocks = [];
	for (const tool of body.tools ?? []) blocks.push(tool);
	const system = typeof body.system === 'string' ? [{ type: 'text', text: body.system }] : (body.system ?? []);
	for (const block of system) {
		if (typeof block.text === 'string' && block.text.startsWith(BILLING_HEADER)) continue;
		blocks.push(block);
	}
	for (const message of body.messages ?? []) {
		const content = typeof message.content === 'string' ? [{ type: 'text', text: message.content }] : (message.content ?? []);
		content.forEach((block, index) => blocks.push(index === 0 ? { role: message.role, ...block } : block));
	}
	return blocks;
}
/* As the provider does, a breakpoint also finds an earlier cached prefix that ends at any of the
 * 20 block boundaries before it, so a conversation that grows by a turn reads its old prefix. */
function account(body) {
	const now = Date.now();
	const hash = createHash('sha256');
	const boundaries = []; // { key, total } after each block
	let total = 0;
	let read = 0;
	let lastBreak = 0;
	let longBreak = 0; // the prefix written under the last one-hour breakpoint
	const ttls = [];
	/* Writes land when the request completes: a later breakpoint in the same request does not read
	 * what an earlier one is writing (the first request of a session reads nothing). */
	const writes = [];
	for (const block of prefixBlocks(body)) {
		const { cache_control: control, ...plain } = block;
		const text = JSON.stringify(plain);
		hash.update(text);
		total += tokens(text);
		boundaries.push({ key: hash.copy().digest('hex'), total });
		if (!control) continue;
		for (const boundary of boundaries.slice(-20).reverse()) {
			if ((cache.get(boundary.key) ?? 0) > now) {
				read = Math.max(read, boundary.total);
				break;
			}
		}
		const ttl = control.ttl === '1h' ? 3_600_000 : 300_000;
		ttls.push(control.ttl ?? '5m');
		if (control.ttl === '1h') longBreak = total;
		writes.push([boundaries.at(-1).key, now + ttl]);
		lastBreak = total;
	}
	for (const [key, until] of writes) cache.set(key, Math.max(cache.get(key) ?? 0, until));
	const written = Math.max(0, lastBreak - read);
	return {
		input_tokens: total - read - written,
		cache_read_input_tokens: read,
		cache_creation_input_tokens: written,
		cache_creation: {
			ephemeral_1h_input_tokens: Math.max(0, longBreak - read),
			ephemeral_5m_input_tokens: written - Math.max(0, longBreak - read)
		},
		ttls
	};
}
function record(input, outputTokens, body, bytes) {
	stats.calls += 1;
	stats.input_tokens += input.input_tokens;
	stats.cache_read_input_tokens += input.cache_read_input_tokens;
	stats.cache_creation_input_tokens += input.cache_creation_input_tokens;
	stats.cache_creation_1h_input_tokens += input.cache_creation.ephemeral_1h_input_tokens;
	stats.output_tokens += outputTokens;
	stats.request_bytes += bytes;
	const model = body.model ?? 'unknown';
	stats.by_model[model] = (stats.by_model[model] ?? 0) + 1;
	for (const ttl of input.ttls) stats.ttl_breakpoints[ttl] = (stats.ttl_breakpoints[ttl] ?? 0) + 1;
}
/* The usage a response reports: Anthropic's fields, without the mock's own bookkeeping. */
const usageOf = ({ ttls: _ttls, ...usage }) => usage;
/* A sign-in request may carry a tool under a prefixed wire name (OMP's Claude Code mode); answer
 * with the name the request declared, so the caller maps it back. */
function wireToolName(body, name) {
	const names = (body.tools ?? []).map((tool) => tool.name).filter(Boolean);
	if (names.includes(name)) return name;
	return (
		names.find((candidate) => candidate.replace(/^_+/, '').toLowerCase() === name.toLowerCase()) ??
		name
	);
}
const outputOf = (step) => (step.tool ? tokens(JSON.stringify(step.tool)) : Math.max(1, tokens(step.text ?? '')));

async function stream(response, model, step, input) {
	response.writeHead(200, {
		'content-type': 'text/event-stream',
		'cache-control': 'no-cache',
		connection: 'keep-alive'
	});
	const send = (event, data) => response.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`);
	const id = `msg_mock_${randomUUID().slice(0, 12)}`;
	const delay = step.delay_ms ?? 20;
	send('message_start', {
		type: 'message_start',
		message: { id, type: 'message', role: 'assistant', model, content: [], stop_reason: null, usage: { ...usageOf(input), output_tokens: 0 } }
	});
	if (step.tool) {
		const input = JSON.stringify(step.tool.input ?? {});
		send('content_block_start', {
			type: 'content_block_start',
			index: 0,
			content_block: { type: 'tool_use', id: `toolu_mock_${randomUUID().slice(0, 12)}`, name: step.tool.name, input: {} }
		});
		for (let at = 0; at < input.length; at += 24) {
			send('content_block_delta', { type: 'content_block_delta', index: 0, delta: { type: 'input_json_delta', partial_json: input.slice(at, at + 24) } });
			await pause(delay);
		}
		send('content_block_stop', { type: 'content_block_stop', index: 0 });
		send('message_delta', { type: 'message_delta', delta: { stop_reason: 'tool_use', stop_sequence: null }, usage: { output_tokens: outputOf(step) } });
	} else {
		const text = step.text ?? '';
		send('content_block_start', { type: 'content_block_start', index: 0, content_block: { type: 'text', text: '' } });
		for (const piece of words(text)) {
			send('content_block_delta', { type: 'content_block_delta', index: 0, delta: { type: 'text_delta', text: piece } });
			await pause(delay);
		}
		send('content_block_stop', { type: 'content_block_stop', index: 0 });
		send('message_delta', { type: 'message_delta', delta: { stop_reason: 'end_turn', stop_sequence: null }, usage: { output_tokens: outputOf(step) } });
	}
	send('message_stop', { type: 'message_stop' });
	response.end();
}

function whole(response, model, step, input) {
	const id = `msg_mock_${randomUUID().slice(0, 12)}`;
	const content = step.tool
		? [{ type: 'tool_use', id: `toolu_mock_${randomUUID().slice(0, 12)}`, name: step.tool.name, input: step.tool.input ?? {} }]
		: [{ type: 'text', text: step.text ?? '' }];
	json(response, 200, {
		id,
		type: 'message',
		role: 'assistant',
		model,
		content,
		stop_reason: step.tool ? 'tool_use' : 'end_turn',
		stop_sequence: null,
		usage: { ...usageOf(input), output_tokens: outputOf(step) }
	});
}

function json(response, status, body) {
	response.writeHead(status, { 'content-type': 'application/json' });
	response.end(JSON.stringify(body));
}

const server = createServer(async (request, response) => {
	const url = new URL(request.url, 'http://mock');
	let raw = '';
	for await (const chunk of request) raw += chunk;
	const body = raw ? JSON.parse(raw) : {};
	/* Which credential sent it, as a short hash, so a check can tell two companies' keys apart. */
	const credential = request.headers['x-api-key'] ?? request.headers.authorization ?? '';
	const key = credential
		? createHash('sha256').update(String(credential)).digest('hex').slice(0, 8)
		: null;
	if (url.pathname === '/stats') {
		if (request.method === 'DELETE') stats = freshStats();
		return json(response, 200, stats);
	}
	const isCall = request.method === 'POST' && url.pathname.endsWith('/messages');
	let step = isCall ? chooseStep(body).step : null;
	if (step?.tool) step = { ...step, tool: { ...step.tool, name: wireToolName(body, step.tool.name) } };
	const input = isCall ? account(body) : null;
	if (isCall && !step.error) record(input, outputOf(step), body, Buffer.byteLength(raw));
	if (log)
		appendFileSync(
			log,
			JSON.stringify({ at: new Date().toISOString(), method: request.method, path: url.pathname, key, usage: input, step: step && (step.tool?.name ?? 'text'), body }) + '\n'
		);

	if (request.method === 'GET' && url.pathname.endsWith('/models')) {
		return json(response, 200, {
			data: MODELS.map((id) => ({ type: 'model', id, display_name: id, created_at: '2026-01-01T00:00:00Z' })),
			has_more: false,
			first_id: MODELS[0],
			last_id: MODELS.at(-1)
		});
	}
	if (request.method === 'POST' && url.pathname.endsWith('/messages/count_tokens')) {
		return json(response, 200, { input_tokens: Math.ceil(raw.length / 4) });
	}
	if (isCall) {
		if (step.error)
			return json(response, Number(step.error), {
				type: 'error',
				error: { type: 'api_error', message: step.message ?? 'Mock provider error' }
			});
		const model = body.model ?? MODELS[0];
		return body.stream ? stream(response, model, step, input) : whole(response, model, step, input);
	}
	json(response, 404, { type: 'error', error: { type: 'not_found_error', message: `mock: no ${request.method} ${url.pathname}` } });
});

server.listen(port, '127.0.0.1', () => console.log(`mock-model listening on http://127.0.0.1:${port}/v1`));
