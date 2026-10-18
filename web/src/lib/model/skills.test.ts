import assert from 'node:assert/strict';
import test from 'node:test';
import * as skills from './skills.ts';
import * as menu from '../primitives/composer-menu.ts';
import type { ComposerOption } from './skills';

const { EXEC_COMMANDS, parseComposerCommand, skillInUse, skillLabel } = skills;
const { composerTrigger, filterComposerOptions } = menu;

test('/goal and /loop become Restless primitives while prose stays prose', () => {
	assert.deepEqual(parseComposerCommand('/goal Ship the pricing page'), {
		kind: 'goal-set',
		objective: 'Ship the pricing page'
	});
	assert.deepEqual(parseComposerCommand('/goal'), { kind: 'goal-show' });
	assert.deepEqual(parseComposerCommand('/goal clear'), { kind: 'goal-clear', target: '' });
	assert.deepEqual(parseComposerCommand('/goal clear 2'), { kind: 'goal-clear', target: '2' });
	assert.deepEqual(parseComposerCommand('/loop stop all'), { kind: 'loop-clear', target: 'all' });
	assert.deepEqual(parseComposerCommand('/loop 30m review inbound leads'), {
		kind: 'loop-set',
		every: '30m',
		prompt: 'review inbound leads'
	});
	assert.equal(parseComposerCommand('/loop soon please')?.kind, 'invalid');
	assert.equal(parseComposerCommand('Compare /goal with OKRs'), null);
});

test('/ opens only at the start of a message and $ at any word boundary', () => {
	assert.deepEqual(composerTrigger('/go', 3), { trigger: '/', query: 'go', start: 0, end: 3 });
	assert.equal(composerTrigger('see src/lib', 11), null);
	assert.deepEqual(composerTrigger('Redesign it $front', 18), {
		trigger: '$',
		query: 'front',
		start: 12,
		end: 18
	});
});

test('# and @ open at a word boundary and offer only their kind', () => {
	assert.deepEqual(composerTrigger('see #oat', 8), {
		trigger: '#',
		query: 'oat',
		start: 4,
		end: 8
	});
	assert.deepEqual(composerTrigger('ask @kat', 8), {
		trigger: '@',
		query: 'kat',
		start: 4,
		end: 8
	});
	assert.equal(composerTrigger('issue#4', 7), null, 'a hash inside a word is prose');
	assert.equal(composerTrigger('mail a@b', 8), null, 'an email address is prose');
	const options: ComposerOption[] = [
		...EXEC_COMMANDS,
		{ kind: 'reference', name: 'oatlands', label: 'Oatlands evidence', description: 'Work' },
		{ kind: 'person', name: 'katniss', label: 'Katniss', description: 'Lead' }
	];
	assert.deepEqual(
		filterComposerOptions(options, '#', '').map((option: ComposerOption) => option.name),
		['oatlands']
	);
	assert.deepEqual(
		filterComposerOptions(options, '@', 'k').map((option: ComposerOption) => option.name),
		['katniss']
	);
	assert.equal(
		filterComposerOptions(options, '/', '').some(
			(option: ComposerOption) => option.kind === 'reference'
		),
		false
	);
});

test('$ offers only skills and hides ones already selected', () => {
	const options: ComposerOption[] = [
		...EXEC_COMMANDS,
		{ kind: 'skill', name: 'gauntlet', label: 'Gauntlet', description: 'Blind review' },
		{ kind: 'skill', name: 'grill-me', label: 'Grill me', description: 'Interview' }
	];
	assert.deepEqual(
		filterComposerOptions(options, '$', 'g').map((option: ComposerOption) => option.name),
		['gauntlet', 'grill-me']
	);
	assert.equal(filterComposerOptions(options, '/', 'g')[0].name, 'goal');
	assert.deepEqual(
		filterComposerOptions(options, '$', '', ['gauntlet']).map(
			(option: ComposerOption) => option.name
		),
		['grill-me']
	);
});

test('/goal clear and /loop stop act only on what was named, never on all by default', () => {
	const rows = ['Ship pricing', 'Hire a designer', 'Review leads'];
	const title = (row: string) => row;
	assert.equal(skills.pickTargets(rows, '', title, true), null, 'several rows: ask which');
	assert.deepEqual(skills.pickTargets(['Only one'], '', title, true), ['Only one']);
	assert.deepEqual(skills.pickTargets(rows, '2', title, true), ['Hire a designer']);
	assert.deepEqual(skills.pickTargets(rows, 'pricing', title, true), ['Ship pricing']);
	assert.equal(skills.pickTargets(rows, 'e', title, true), null, 'ambiguous words ask');
	assert.deepEqual(skills.pickTargets(rows, 'all', title, true), rows);
	assert.equal(skills.pickTargets(rows, 'all', title, false), null, 'goals close one at a time');
	assert.equal(skills.pickTargets(rows, '9', title, true), null);
});

test('a skill applied in a tool call reads as that skill', () => {
	assert.equal(skillInUse('restless skill use frontend-design --work abc'), 'frontend-design');
	assert.equal(skillInUse('restless skill list'), null);
	assert.equal(skillLabel('frontend-design'), 'Frontend design');
});

test('an uploaded SKILL.md keeps its own name, or is named from its file and first heading', () => {
	const named = '---\nname: resale-deals\ndescription: "Find deals"\n---\n\n# Resale deals\n';
	assert.equal(skills.skillMdFromUpload('anything.md', named), named);
	const plain = '# Resale deals\n\nOnly flag a deal above A$100 profit.\n';
	const made = skills.skillMdFromUpload('resale-deals-SKILL.md', plain);
	assert.match(made, /^---\nname: resale-deals\ndescription: "Resale deals"\n---\n/);
	assert.ok(made.includes('A$100 profit'));
});
