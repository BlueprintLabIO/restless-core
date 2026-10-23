import assert from 'node:assert/strict';
import test from 'node:test';
import { failureLine, replyIsError, turnFailureFix, turnFailureReason } from './turn-failure.ts';

test('a refused key names the key and where to fix it, not the transport', () => {
	const reason = turnFailureReason(
		'[transport] the agent session failed: acp session/set_config_option for exact model: Internal error: { "details": "Authentication failed. Please check your API credentials." }'
	);
	assert.match(reason, /refused its key/);
	assert.match(reason, /Company → Intelligence/);
	assert.doesNotMatch(reason, /transport|acp|Internal error/);
});

test('credit, rate limits, missing models and dropped connections each say so', () => {
	assert.match(turnFailureReason('Your credit balance is too low'), /out of credit/);
	assert.match(turnFailureReason('429 Too Many Requests'), /busy/);
	assert.match(turnFailureReason('model claude-x not found'), /not available/);
	assert.match(turnFailureReason('request timed out after 60s'), /dropped/);
});

test('an unavailable connection says where to look, without repeating that the reply failed', () => {
	const reason = turnFailureReason(
		'Exec reply failed: This intelligence connection is temporarily unavailable.'
	);
	assert.match(reason, /unavailable right now/);
	assert.doesNotMatch(reason, /reply failed/);
});

test('anything else keeps its first sentence without the runtime prefixes', () => {
	assert.equal(
		turnFailureReason(
			'[transport] the agent session failed: The workspace is read-only. More text.'
		),
		'The workspace is read-only.'
	);
	assert.equal(turnFailureReason(''), '');
});

test('the company spend ceiling reads as a sentence with where to raise it, never the raw id', () => {
	const raw =
		'[budget] company_01a1099254287140b24b3202429a3f01 has spent $100.00 of its $100.00 ceiling; the owner must raise it before charged work continues';
	const reason = turnFailureReason(raw);
	assert.equal(
		reason,
		'the company has reached its $100.00 spend limit. Raise it in Company → Limits.'
	);
	assert.doesNotMatch(reason, /company_|\[budget\]/);
	assert.deepEqual(turnFailureFix(raw), {
		label: 'Company → Limits',
		path: 'company/resources#spend'
	});
	assert.match(
		turnFailureReason(
			'[budget] acme has used 40 of its 40 native-harness turns this month; raise it'
		),
		/native-harness allowance/
	);
});

test('a fix is offered only where the cockpit has a page for it', () => {
	assert.equal(turnFailureFix('Authentication failed')?.label, 'Company → Intelligence');
	assert.equal(turnFailureFix('request timed out'), null);
	assert.equal(turnFailureFix(''), null);
});

test('a provider error body streamed as the reply is not shown as the reply', () => {
	const body =
		'401 {"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}';
	assert.equal(replyIsError(body, '[credential] provider rejected the credential: ' + body), true);
	assert.equal(replyIsError('{"type":"error","error":{}}', ''), true);
	assert.equal(replyIsError('I checked Work and', '[transport] connection reset'), false);
	assert.equal(replyIsError('', 'anything'), false);
});

test('a Work blocker that is a runtime failure reads as a sentence; anything else is unchanged', () => {
	assert.equal(
		failureLine(
			'[transport] the agent session failed: No model connection is active yet. Connect a model in Company → Intelligence (Restless loads it within seconds), or assign this agent to another.'
		),
		'Its model connection is unavailable right now. Check it in Company → Intelligence.'
	);
	assert.equal(
		failureLine(
			"[transport] the agent session failed: This agent's account connection is no longer granted to the company"
		),
		'Its model connection is unavailable right now. Check it in Company → Intelligence.'
	);
	assert.equal(
		failureLine('Waiting on the owner to approve the hero copy.'),
		'Waiting on the owner to approve the hero copy.'
	);
});

test('a stored provider error reads as the same useful sentence as a live failure', () => {
	assert.equal(
		failureLine('529 {"type":"error","error":{"message":"Overloaded"}}'),
		'The model provider is busy right now. Try again in a minute.'
	);
	assert.equal(
		failureLine('The response contained a 529 error during our test.'),
		'The response contained a 529 error during our test.'
	);
});

test("Bun's fetch error saved as Exec's reply reads as a dropped connection, never as prose", () => {
	// 23 October: a model switch restarted the gateway mid-reply and this sentence was saved as the answer.
	const bun =
		'The socket connection was closed unexpectedly. For more information, pass `verbose: true` in the second argument to fetch()';
	assert.equal(replyIsError(bun, null), true);
	assert.equal(failureLine(bun), 'The connection to the model dropped. Try again.');
	assert.equal(
		turnFailureReason(`[transport] the agent session failed: ${bun}`),
		'the connection to the model dropped. Try again.'
	);
	// A real answer about sockets, or a partial reply, is still the reply.
	const prose =
		'The socket connection was closed unexpectedly when nginx restarted, so I raised its keepalive.';
	assert.equal(replyIsError(prose, null), false);
	assert.equal(failureLine(prose), prose);
	assert.equal(replyIsError(`I drafted the plan and ${bun}`, null), false);
});
