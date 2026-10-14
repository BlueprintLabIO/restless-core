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
// A scenario is a list of rules; the first whose `match` (a case-insensitive regular expression)
// matches the latest person's text answers. A rule has `steps`, run one per model call: a step is
// `{ "tool": { "name": "...", "input": {...} } }`, which asks for a tool call, or `{ "text": "..." }`,
// which ends the turn. The step taken is the number of tool results since that person's text, so
// a rule with a tool step then a text step calls the tool, gets its result, then replies. A step may
// also carry `delay_ms` (between streamed pieces), `error` (an HTTP status to answer with) and
// `message` (that error's text). With no matching rule the mock answers "Mock reply: <their text>".
// Every request is appended to --log as one JSON line, for checks to read.
import { createServer } from 'node:http';
import { readFileSync, appendFileSync } from 'node:fs';
import { randomUUID } from 'node:crypto';

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

function chooseStep(messages) {
	const { text, toolResults } = turnState(messages);
	for (const rule of scenario()) {
		if (!new RegExp(rule.match ?? '.*', 'i').test(text)) continue;
		const steps = rule.steps ?? [{ text: rule.reply ?? '' }];
		return { step: steps[Math.min(toolResults, steps.length - 1)], text };
	}
	return { step: { text: `Mock reply: ${text.slice(0, 400)}` }, text };
}

const words = (text) => text.match(/\S+\s*/g) ?? [''];
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const usage = (text) => ({ input_tokens: 100, output_tokens: Math.max(1, Math.ceil(text.length / 4)) });

async function stream(response, model, step) {
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
		message: { id, type: 'message', role: 'assistant', model, content: [], stop_reason: null, usage: { input_tokens: 100, output_tokens: 0 } }
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
		send('message_delta', { type: 'message_delta', delta: { stop_reason: 'tool_use', stop_sequence: null }, usage: { output_tokens: 20 } });
	} else {
		const text = step.text ?? '';
		send('content_block_start', { type: 'content_block_start', index: 0, content_block: { type: 'text', text: '' } });
		for (const piece of words(text)) {
			send('content_block_delta', { type: 'content_block_delta', index: 0, delta: { type: 'text_delta', text: piece } });
			await pause(delay);
		}
		send('content_block_stop', { type: 'content_block_stop', index: 0 });
		send('message_delta', { type: 'message_delta', delta: { stop_reason: 'end_turn', stop_sequence: null }, usage: { output_tokens: usage(text).output_tokens } });
	}
	send('message_stop', { type: 'message_stop' });
	response.end();
}

function whole(response, model, step) {
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
		usage: usage(step.text ?? '')
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
	if (log) appendFileSync(log, JSON.stringify({ at: new Date().toISOString(), method: request.method, path: url.pathname, body }) + '\n');

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
	if (request.method === 'POST' && url.pathname.endsWith('/messages')) {
		const { step } = chooseStep(body.messages ?? []);
		if (step.error)
			return json(response, Number(step.error), {
				type: 'error',
				error: { type: 'api_error', message: step.message ?? 'Mock provider error' }
			});
		const model = body.model ?? MODELS[0];
		return body.stream ? stream(response, model, step) : whole(response, model, step);
	}
	json(response, 404, { type: 'error', error: { type: 'not_found_error', message: `mock: no ${request.method} ${url.pathname}` } });
});

server.listen(port, '127.0.0.1', () => console.log(`mock-model listening on http://127.0.0.1:${port}/v1`));
