import assert from 'node:assert/strict';
import test from 'node:test';
import {
	clientFolders,
	crumbs,
	decodePin,
	explore,
	filterPin,
	folderChain,
	folderOf,
	folderPin,
	humanName,
	liveCollections,
	parentFolder,
	type LiveItem
} from './library-explorer.ts';

test('a file sits in its directory; the outputs top level and pathless items are the top', () => {
	assert.equal(folderOf('/company/outputs/index.html'), '');
	assert.equal(
		folderOf('/company/outputs/sites/aleon-electrical/v3/index.html'),
		'/company/outputs/sites/aleon-electrical/v3'
	);
	assert.equal(folderOf('/company/downloads/brief.pdf'), '/company/downloads');
	assert.equal(folderOf('http://127.0.0.1:8650/'), '');
	assert.equal(folderOf(''), '');
	assert.deepEqual(folderChain('/company/outputs/sites/aleon'), [
		'/company/outputs/sites',
		'/company/outputs/sites/aleon'
	]);
	assert.deepEqual(folderChain('/company/downloads'), ['/company/downloads']);
	assert.equal(parentFolder('/company/outputs/sites/aleon'), '/company/outputs/sites');
	assert.equal(parentFolder('/company/outputs/sites'), '');
});

test('slugs read as names; names that are already words keep their words', () => {
	assert.equal(humanName('aleon-electrical', 'folder'), 'Aleon Electrical');
	assert.equal(humanName('outreach_test-send', 'folder'), 'Outreach Test Send');
	assert.equal(humanName('house-of-the-rising-sun', 'folder'), 'House of the Rising Sun');
	assert.equal(humanName('v3', 'folder'), 'Version 3');
	assert.equal(humanName('mobile-390.png'), 'Mobile 390');
	assert.equal(humanName('concept-a1000-upgrade-report.md'), 'Concept a1000 upgrade report');
	assert.equal(humanName('README.md'), 'README');
	assert.equal(
		humanName('Taylor & Ward concept website, version 1.html'),
		'Taylor & Ward concept website, version 1'
	);
	assert.equal(humanName('Q4%20plan.pdf'), 'Q4 plan');
});

test('the breadcrumb names each folder, by the site it holds when its own name is a machine’s', () => {
	const titles = new Map([
		['/company/outputs/sites/aleon/v3', 'Aleon Electrical concept, version 3'],
		[
			'/company/outputs/bakehouse-concept-website-version-3-quiet-luxury-27',
			'Bakehouse concept website, version 3'
		]
	]);
	assert.deepEqual(
		crumbs('/company/outputs/sites/aleon/v3', titles).map((crumb) => crumb.title),
		['All files', 'Sites', 'Aleon', 'Version 3']
	);
	assert.deepEqual(
		crumbs('/company/outputs/bakehouse-concept-website-version-3-quiet-luxury-27', titles).map(
			(crumb) => crumb.title
		),
		['All files', 'Bakehouse concept website, version 3']
	);
	assert.equal(crumbs('/company/downloads', titles)[1].title, 'Downloads');
});

test('a folder lists the folders inside it, newest first, then its own files', () => {
	const items = [
		{ id: 'a', dir: '', updatedAt: '2026-11-01T00:00:00Z' },
		{ id: 'b', dir: '/company/outputs/sites/aleon/v1', updatedAt: '2026-10-01T00:00:00Z' },
		{ id: 'c', dir: '/company/outputs/sites/aleon/v2', updatedAt: '2026-11-02T00:00:00Z' },
		{ id: 'd', dir: '/company/outputs/sites/bakehouse', updatedAt: '2026-10-20T00:00:00Z' },
		{ id: 'e', dir: '/company/outputs/sites', updatedAt: '2026-10-21T00:00:00Z' }
	];
	const top = explore(items, '', new Map());
	assert.deepEqual(
		top.folders.map((folder) => folder.title),
		['Sites']
	);
	assert.deepEqual(
		top.items.map((item) => item.id),
		['a']
	);
	const sites = explore(items, '/company/outputs/sites', new Map());
	assert.deepEqual(
		sites.folders.map((folder) => folder.title),
		['Aleon', 'Bakehouse']
	);
	assert.equal(sites.folders[0].updatedAt, '2026-11-02T00:00:00.000Z');
	assert.deepEqual(
		sites.items.map((item) => item.id),
		['e']
	);
	/* A folder holding only hidden notes is simply not among the items, so it is not shown. */
	assert.deepEqual(explore([], '', new Map()).folders, []);
});

test('folders and saved filters pin under keys the server accepts, and read back', () => {
	const folder = folderPin('/company/outputs/sites/Aleon Electrical/v3');
	assert.ok(folder && /^folder:[A-Za-z0-9_-]+$/.test(folder));
	assert.deepEqual(decodePin(folder!), {
		kind: 'folder',
		folder: '/company/outputs/sites/Aleon Electrical/v3'
	});
	const saved = filterPin('Site shots this week', '?type=image&since=7');
	assert.ok(saved && /^filter:[A-Za-z0-9_-]+$/.test(saved));
	assert.deepEqual(decodePin(saved!), {
		kind: 'filter',
		name: 'Site shots this week',
		search: 'type=image&since=7'
	});
	assert.deepEqual(decodePin('doc:123'), { kind: 'item', key: 'doc:123' });
	assert.equal(folderPin(`/company/outputs/${'x'.repeat(300)}`), null);
	assert.equal(decodePin('folder:@@'), null);
});

const item = (over: Partial<LiveItem>): LiveItem => ({
	key: 'site:x',
	kind: 'site',
	dir: '',
	updatedAt: '2026-11-02T00:00:00Z',
	workId: null,
	review: false,
	inReview: false,
	live: false,
	...over
});

test('a client folder is the deepest folder two pieces of Work wrote to, never a kind folder', () => {
	const items = [
		item({ dir: '/company/outputs/sites/aleon-electrical/v1', workId: 'w1' }),
		item({ dir: '/company/outputs/sites/aleon-electrical/v2', workId: 'w2' }),
		item({ dir: '/company/outputs/sites/bakehouse', workId: 'w3' }),
		item({ dir: '/company/outputs/images', workId: 'w4' }),
		item({ dir: '/company/outputs/images', workId: 'w5' })
	];
	assert.deepEqual([...clientFolders(items)], ['/company/outputs/sites/aleon-electrical']);
});

test('Live now mirrors open goals and busy clients, says why, and drops a closed goal', () => {
	const now = Date.parse('2026-11-03T00:00:00Z');
	const works = [
		{ id: 'w1', goal_id: 'g1' },
		{ id: 'w2', goal_id: 'g1' },
		{ id: 'w3', goal_id: 'g2' },
		{ id: 'w4', goal_id: 'g3' }
	];
	const items = [
		item({
			key: 'site:1',
			dir: '/company/outputs/sites/aleon-electrical/v1',
			workId: 'w1',
			updatedAt: '2026-11-01T00:00:00Z'
		}),
		item({
			key: 'site:2',
			dir: '/company/outputs/sites/aleon-electrical/v2',
			workId: 'w2',
			updatedAt: '2026-11-02T12:00:00Z',
			review: true
		}),
		item({
			key: 'doc:3',
			dir: '',
			workId: 'w3',
			updatedAt: '2026-11-02T00:00:00Z',
			kind: 'site',
			live: true
		}),
		item({ key: 'site:4', dir: '', workId: 'w4', updatedAt: '2026-11-02T00:00:00Z' }),
		item({ key: 'doc:5', dir: '', updatedAt: '2026-10-01T00:00:00Z', collection: 'Old pitch' })
	];
	const goals = [
		{ id: 'g1', title: 'Win Aleon', closed_at: null },
		{ id: 'g2', title: 'Outreach test send', closed_at: null },
		{ id: 'g3', title: 'Last quarter', closed_at: '2026-10-30T00:00:00Z' }
	];
	const live = liveCollections({ goals, works, items, titles: new Map(), since: 0, now });
	assert.deepEqual(
		live.map((row) => [row.title, row.reason]),
		[
			['Aleon Electrical', 'needs you'],
			['Win Aleon', 'needs you'],
			['Outreach test send', 'live site']
		]
	);
	/* New since the last visit, when nothing waits on the owner. */
	const later = liveCollections({
		goals,
		works,
		items: items.map((row) => ({ ...row, review: false })),
		titles: new Map(),
		since: Date.parse('2026-11-01T06:00:00Z'),
		now
	});
	assert.equal(later.find((row) => row.title === 'Win Aleon')?.reason, '1 new');
	/* At most six. */
	const many = Array.from({ length: 9 }, (_, index) => ({
		id: `m${index}`,
		title: `Goal ${index}`,
		closed_at: null
	}));
	const busy = liveCollections({
		goals: many,
		works: many.map((goal) => ({ id: `w-${goal.id}`, goal_id: goal.id })),
		items: many.map((goal) => item({ key: `site:${goal.id}`, workId: `w-${goal.id}` })),
		titles: new Map(),
		since: 0,
		now
	});
	assert.equal(busy.length, 6);
});
