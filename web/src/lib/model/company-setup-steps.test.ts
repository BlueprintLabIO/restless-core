import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	currentSetupStep,
	setupSteps,
	setupStoppedShort,
	setupTabs
} from './company-setup-steps.ts';

test('the model step is there only while Exec has no model', () => {
	assert.deepEqual(setupSteps(false), ['name', 'idea', 'readback']);
	assert.deepEqual(setupSteps(true), ['name', 'model', 'idea', 'readback']);
});

test('the step follows saved state: name, then a model if needed, then questions, then the read-back', () => {
	assert.equal(currentSetupStep({ nameSettled: false, needsModel: true, readback: false }), 'name');
	assert.equal(currentSetupStep({ nameSettled: true, needsModel: true, readback: false }), 'model');
	assert.equal(currentSetupStep({ nameSettled: true, needsModel: false, readback: false }), 'idea');
	assert.equal(
		currentSetupStep({ nameSettled: false, needsModel: false, readback: true }),
		'readback'
	);
});

test('tabs mark earlier steps done and the current one on', () => {
	assert.deepEqual(
		setupTabs('idea', false).map((tab) => [tab.label, tab.done, tab.on]),
		[
			['Name', true, false],
			['Your idea', false, true],
			['Read back', false, false]
		]
	);
	assert.deepEqual(
		setupTabs('model', false).map((tab) => tab.label),
		['Name', 'Model', 'Your idea', 'Read back']
	);
});

test('while Exec asks, the current step counts its questions', () => {
	assert.equal(setupTabs('idea', false, 3)[1].label, 'Your idea · 3');
	assert.equal(setupTabs('idea', false, 0)[1].label, 'Your idea');
	assert.equal(setupTabs('readback', false, 4)[1].label, 'Your idea');
});

test('Carry on is offered only when a reply to the owner stops short, never for the opening', () => {
	// Exec's opening lines exactly as the server writes them: no intent, no question.
	const opening = [
		{
			from: 'agent',
			text: "Hi, I'm Exec. I'll run Reno with you. A few quick questions and we're off."
		},
		{ from: 'agent', text: 'So, tell me about this idea of yours.' }
	];
	const answer = { from: 'you', text: 'A small renovation business in Leeds.' };
	assert.equal(setupStoppedShort([]), false);
	assert.equal(setupStoppedShort(opening), false, 'a first setup opens without Carry on');
	assert.equal(setupStoppedShort([...opening, answer]), false, 'still thinking is not stopping');
	const asks = {
		from: 'agent',
		text: 'Who is it for?',
		intent: { ownerNeed: 'Who is it for?' }
	};
	assert.equal(setupStoppedShort([...opening, answer, asks]), false);
	const readsBack = {
		from: 'agent',
		text: 'Here is what I am hearing.',
		intent: { readback: [{ label: 'Purpose', text: 'Renovations in Leeds' }] }
	};
	assert.equal(setupStoppedShort([...opening, answer, readsBack]), false);
	// A real truncation: Exec answered the owner, then ended without a question or read-back.
	const truncated = { from: 'agent', text: 'Good, renovations. Let me think about who' };
	assert.equal(setupStoppedShort([...opening, answer, truncated]), true);
	assert.equal(setupStoppedShort([...opening, answer, asks, answer, truncated]), true);
});
