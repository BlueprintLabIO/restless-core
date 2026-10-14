import assert from 'node:assert/strict';
import test from 'node:test';
import { turnFailureFix, turnFailureReason } from './turn-failure.ts';

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
