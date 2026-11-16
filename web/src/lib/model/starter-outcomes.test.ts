import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	STARTER_OUTCOMES,
	offeredStarters,
	starterMessage,
	starterObjective
} from './starter-outcomes.ts';

test('at most three starters are offered, each short enough for one line on a phone', () => {
	const offered = offeredStarters();
	assert.ok(offered.length >= 1 && offered.length <= 3);
	assert.equal(new Set(offered.map((starter) => starter.id)).size, offered.length);
	for (const starter of offered) {
		assert.ok(starter.title.length <= 42, starter.title);
		assert.ok(starter.doneWhen.trim() && starter.firstStep.trim(), starter.id);
	}
	const four = [1, 2, 3, 4].map((n) => ({ ...STARTER_OUTCOMES[0], id: `s${n}` }));
	assert.equal(offeredStarters(four).length, 3);
	assert.equal(offeredStarters([{ ...STARTER_OUTCOMES[0], title: ' ' }]).length, 0);
});

test('a starter becomes a goal whose first line is its title', () => {
	const [starter] = STARTER_OUTCOMES;
	const objective = starterObjective(starter);
	// The owner API takes the first line as the goal's title.
	assert.equal(objective.split('\n')[0], starter.title);
	assert.match(objective, /\nDone when: .+\.$/);
});

test('what the owner says by tapping one has no ids, commands or words they would not use', () => {
	for (const starter of STARTER_OUTCOMES) {
		const said = `${starterMessage(starter)} ${starterObjective(starter)}`;
		assert.ok(starterMessage(starter).startsWith('Let’s do it. Start with this: '));
		assert.doesNotMatch(said, /restless |--|[0-9a-f]{8}-[0-9a-f]{4}/);
		assert.doesNotMatch(said, /charter|vault|api key|compute|revision|harness/i);
	}
});
