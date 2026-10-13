import assert from 'node:assert/strict';
import test from 'node:test';
import { turnFailureReason } from './turn-failure.ts';

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

test('anything else keeps its first sentence without the runtime prefixes', () => {
	assert.equal(
		turnFailureReason(
			'[transport] the agent session failed: The workspace is read-only. More text.'
		),
		'The workspace is read-only.'
	);
	assert.equal(turnFailureReason(''), '');
});
