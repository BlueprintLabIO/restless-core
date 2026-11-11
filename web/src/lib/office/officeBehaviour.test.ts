import assert from 'node:assert/strict';
import test from 'node:test';
import { belongsAtDesk } from './officeBehaviour.ts';

test('anyone with Work under way belongs at their desk', () => {
	for (const presence of ['observed', 'in-motion', 'stale', 'unknown'])
		assert.equal(belongsAtDesk(presence), true, presence);
});

test('waiting, available and unreadable colleagues do not', () => {
	for (const presence of ['waiting', 'available', 'unavailable'])
		assert.equal(belongsAtDesk(presence), false, presence);
});
