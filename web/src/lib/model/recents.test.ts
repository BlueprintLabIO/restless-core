import assert from 'node:assert/strict';
import test from 'node:test';
import { MAX_RECENTS, recentPlace, remember, type Recent } from './recents.ts';

const url = (path: string) => new URL(path, 'https://cockpit.test');

test('only documents, sheets and conversations are places to return to', () => {
	assert.deepEqual(
		recentPlace('acme', url('/acme/library/documents?document=d-1&inspect=comments')),
		{
			kind: 'document',
			id: 'd-1',
			href: '/acme/library/documents?document=d-1'
		}
	);
	assert.deepEqual(recentPlace('acme', url('/acme/library/sheets?sheet=s-1')), {
		kind: 'sheet',
		id: 's-1',
		href: '/acme/library/sheets?sheet=s-1'
	});
	assert.equal(recentPlace('acme', url('/acme/people?room=r-1&person=bianca'))?.kind, 'room');
	assert.equal(
		recentPlace('acme', url('/acme/people?person=bianca'))?.href,
		'/acme/people?person=bianca'
	);
	for (const path of [
		'/acme',
		'/acme/library',
		'/acme/library/documents',
		'/acme/people',
		'/other/library/sheets?sheet=s-1'
	])
		assert.equal(recentPlace('acme', url(path)), null, path);
});

test('the newest visit leads and each place appears once, bounded', () => {
	const entry = (id: string): Recent => ({
		key: `document:${id}`,
		kind: 'document',
		id,
		label: id,
		href: `/acme/library/documents?document=${id}`
	});
	let list: Recent[] = [];
	for (let index = 0; index < MAX_RECENTS + 3; index += 1)
		list = remember(list, entry(`d-${index}`));
	assert.equal(list.length, MAX_RECENTS);
	assert.equal(list[0].id, `d-${MAX_RECENTS + 2}`);
	list = remember(list, { ...entry('d-5'), label: 'Renamed' });
	assert.equal(list[0].label, 'Renamed');
	assert.equal(list.filter((item) => item.id === 'd-5').length, 1);
});
