import { test } from 'node:test';
import assert from 'node:assert/strict';
import { currentSetupStep, setupSteps, setupTabs } from './company-setup-steps.ts';

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
