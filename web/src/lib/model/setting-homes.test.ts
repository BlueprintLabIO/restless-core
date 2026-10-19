import assert from 'node:assert/strict';
import test from 'node:test';
import { habitLine, PROPOSAL_HOMES, settingHome } from './setting-homes.ts';
import { inboxPreviewTarget } from './inbox-preview.ts';

const origin = 'http://127.0.0.1:23296';

test('every Company home a card names opens beside the item or on its own page', () => {
	for (const key of ['limits', 'intelligence', 'brand', 'charter', 'notifications'] as const) {
		const home = settingHome('ui_test', key);
		assert.ok(home.label.startsWith('Company › '));
		assert.ok(inboxPreviewTarget('ui_test', home.href, origin), `${key} previews in the Inbox`);
	}
	for (const key of ['computer', 'schedules'] as const)
		assert.equal(inboxPreviewTarget('ui_test', settingHome('ui_test', key).href, origin), null);
});

test('a drift card names the artifact it decides; a conversation names the person', () => {
	assert.equal(
		settingHome('ui_test', 'brand', { id: 'f 1' }).href,
		'/ui_test/company/identity?drift=f%201'
	);
	const person = settingHome('ui_test', 'people', { id: 'maya', name: 'Maya' });
	assert.equal(person.label, 'People › Maya');
	assert.equal(person.href, '/ui_test/people?person=maya&view=people');
});

test('every proposable setting has a home', () => {
	for (const setting of [
		'spend_limit',
		'reasoning_effort',
		'model',
		'legal_profile',
		'schedule',
		'notifications'
	])
		assert.ok(PROPOSAL_HOMES[setting], setting);
});

test('a habit line says what setting it once would do, and counts the answers', () => {
	const limits = habitLine(settingHome('ui_test', 'limits'), 3);
	assert.match(limits.before, /3 times this month/);
	assert.match(limits.after, /stop these/);
	assert.match(habitLine(settingHome('ui_test', 'intelligence'), 4).after, /by itself/);
});
