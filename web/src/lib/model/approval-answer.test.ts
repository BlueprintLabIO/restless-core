import assert from 'node:assert/strict';
import test from 'node:test';
import { approvalDecision, approvalTarget } from './approval-answer.ts';

const source = (fields: { call_key?: string; party?: string }) => ({
	source: { plane: 'authority', kind: 'approval_required', reference: '7', ...fields }
});

test('a held call is answered by its key, a first contact by its party', () => {
	assert.deepEqual(approvalTarget(source({ call_key: 'k1', party: 'x@y.z' })), {
		kind: 'tool_call',
		call_key: 'k1'
	});
	assert.deepEqual(approvalTarget(source({ party: 'lead@acme.com' })), {
		kind: 'party',
		party: 'lead@acme.com'
	});
	assert.throws(() => approvalTarget(source({})), /no approval target/);
});

test('a grant carries no note; a decline carries only the words given', () => {
	assert.deepEqual(approvalDecision('grant'), { action: 'grant' });
	assert.deepEqual(approvalDecision('grant', { always: true, note: 'ignored' }), {
		action: 'grant',
		always: true
	});
	assert.deepEqual(approvalDecision('decline', { note: '   ' }), { action: 'decline' });
	assert.deepEqual(approvalDecision('decline', { note: ' Not yet ', changesRequested: true }), {
		action: 'decline',
		note: 'Not yet',
		changes_requested: true
	});
});
