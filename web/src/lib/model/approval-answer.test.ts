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

test('every other Inbox approval names its own request, never a party', () => {
	const of = (kind: string, reference: string, extra: object = {}) => ({
		source: { plane: 'authority', kind, reference },
		...extra
	});
	assert.deepEqual(approvalTarget(of('email_mandate_proposal', 'f6b1')), {
		kind: 'email_mandate',
		proposal: 'f6b1'
	});
	assert.deepEqual(approvalTarget(of('payee_proposal', '42')), { kind: 'payee', proposal: 42 });
	assert.throws(() => approvalTarget(of('payee_proposal', 'x')), /no approval target/);
	assert.deepEqual(
		approvalTarget(
			of('account_notice', 'n1', {
				accountUse: { account: { id: 'acct', label: 'GitHub', kind: 'github', provider: null } }
			}) as never
		),
		{ kind: 'account_notice', account: 'acct', notice: 'n1' }
	);
	assert.throws(() => approvalTarget(of('account_notice', 'n1')), /no approval target/);
	assert.deepEqual(approvalTarget(of('account_request', 'r1')), {
		kind: 'account_request',
		request: 'r1'
	});
	assert.deepEqual(approvalTarget(of('publication_authorization', 'p1')), {
		kind: 'publication',
		publication: 'p1'
	});
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
