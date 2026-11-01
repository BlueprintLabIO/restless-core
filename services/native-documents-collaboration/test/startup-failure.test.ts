import assert from 'node:assert/strict';
import test from 'node:test';
import { startupFailure } from '../src/main.js';

test('a startup failure names its cause without credentials or tokens', () => {
  const error = Object.assign(new Error('connect ECONNREFUSED postgres://restless:hunter2@127.0.0.1:5432/docs\nstack'), { code: 'ECONNREFUSED' });
  const text = startupFailure(error);
  assert.match(text, /^Error ECONNREFUSED: connect ECONNREFUSED postgres:\/\/\*\*\*@127\.0\.0\.1:5432\/docs$/);
  assert.doesNotMatch(text, /hunter2/);
  assert.equal(startupFailure(new Error(`bad key ${'k'.repeat(40)}`)), 'Error: bad key ***');
  assert.equal(startupFailure('nope'), 'unknown error');
});
