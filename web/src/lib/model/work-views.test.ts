import assert from 'node:assert/strict';
import test from 'node:test';
import { emptyWorkTitle } from './work-views.ts';

test('a filter says what it found nothing of; only a goal talks about goals', () => {
	assert.equal(emptyWorkTitle('waiting'), 'Nothing is waiting on you.');
	assert.equal(emptyWorkTitle('active'), 'Nothing is in progress.');
	assert.equal(emptyWorkTitle('archived'), 'Nothing is archived.');
	assert.equal(emptyWorkTitle('other'), 'No other work.');
	assert.equal(emptyWorkTitle(''), 'No work yet');
	assert.equal(
		emptyWorkTitle('4f0c2d1e-0000-4000-8000-000000000000'),
		'No work under this goal yet'
	);
});
