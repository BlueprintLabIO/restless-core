import assert from 'node:assert/strict';
import test from 'node:test';
import {
	libraryFileById,
	libraryFiles,
	replacedFile,
	type LibraryArtifact
} from './library-files.ts';
import { retiredWorkIds } from './library-files.ts';

const record = (over: Partial<LibraryArtifact>): LibraryArtifact => ({
	id: 'a',
	kind: 'output',
	uri: '/company/outputs/q4/deck/index.html',
	label: '',
	created_by: 'staff-1',
	work_id: 'w1',
	state: 'available',
	created_at: '2026-10-10T10:00:00Z',
	...over
});

/* Seen live: Staff record one deck twice, a bare `output` and the skill's named `deck`. */
test('two records of one file are one named deck that either id opens', () => {
	const files = libraryFiles(
		[
			record({ id: 'bare', label: 'output', created_at: '2026-10-10T10:01:00Z' }),
			record({ id: 'named', kind: 'deck', label: 'Q4 board deck' })
		],
		[{ id: 'w1', title: 'Prepare the Q4 board deck' }]
	);
	assert.equal(files.length, 1);
	assert.equal(files[0].kind, 'deck');
	assert.equal(files[0].label, 'Q4 board deck');
	assert.equal(libraryFileById(files, 'bare'), files[0]);
	assert.equal(libraryFileById(files, 'named'), files[0]);
});

test('an unlabelled output is named after its file, or its folder for a page', () => {
	const [page, pdf] = libraryFiles([
		record({ id: 'p', label: 'output', created_at: '2026-10-10T10:02:00Z' }),
		record({ id: 'f', uri: '/company/outputs/run/plan.pdf', label: '' })
	]);
	assert.equal(page.label, 'q4');
	assert.equal(pdf.label, 'plan.pdf');
});

/* Seen in production: a local site whose page title was its dev-server address. */
test('a site titled with its own address is named after its Work', () => {
	const files = libraryFiles(
		[
			record({ id: 's1', kind: 'site', uri: 'http://127.0.0.1:8650/', label: '127.0.0.1:8650' }),
			record({
				id: 's2',
				kind: 'site',
				uri: 'http://localhost:5173/pricing',
				label: 'http://localhost:5173/pricing',
				work_id: 'w2'
			})
		],
		[
			{ id: 'w1', title: 'Launch page' },
			{ id: 'w2', title: 'Pricing page' }
		]
	);
	assert.deepEqual(files.map((file) => file.label).toSorted(), ['Launch page', 'Pricing page']);
	assert.ok(files.every((file) => !/127\.0\.0\.1|localhost/.test(file.label)));
});

test('a real title that merely mentions a port is kept', () => {
	const [file] = libraryFiles([record({ label: 'Q4 review: port 8080 migration' })]);
	assert.equal(file.label, 'Q4 review: port 8080 migration');
});

/* Seen in production: Exec linked a site's first version after a redo replaced it. */
test('a link to a replaced file finds the newer file from the same Work', () => {
	const old = record({
		id: 'v1',
		kind: 'site',
		uri: '/company/outputs/taylor/v1/index.html',
		label: 'Taylor & Ward site',
		state: 'superseded',
		created_at: '2026-10-12T10:00:00Z'
	});
	const newer = record({
		id: 'v2',
		kind: 'site',
		uri: '/company/outputs/taylor/v2/index.html',
		label: 'Taylor & Ward site, visual',
		created_at: '2026-10-13T10:00:00Z'
	});
	const other = record({
		id: 'x',
		work_id: 'w9',
		uri: '/company/outputs/other/index.html',
		label: 'Other'
	});
	const works = [{ id: 'w1', title: 'Taylor & Ward redo' }];
	const files = libraryFiles([old, newer, other], works);
	assert.equal(libraryFileById(files, 'v1'), null, 'the old version is not listed');
	const found = replacedFile([old, newer, other], files, 'v1', works);
	assert.equal(found?.gone, 'Taylor & Ward site');
	assert.equal(found?.workTitle, 'Taylor & Ward redo');
	assert.equal(found?.replacement?.id, 'v2');
	assert.equal(replacedFile([old, newer], files, 'missing'), null);
});

test('a replaced file with nothing newer says so', () => {
	const old = record({ id: 'v1', label: 'Draft', state: 'superseded' });
	const found = replacedFile([old], libraryFiles([old]), 'v1');
	assert.equal(found?.replacement, null);
	assert.equal(found?.gone, 'Draft');
});

test("stopped Work is retired; a reviewer's revises edge retires nothing", () => {
	const retired = retiredWorkIds([
		{ id: 'site', status: 'completed' },
		{ id: 'review', status: 'completed' },
		{ id: 'abandoned', status: 'abandoned' }
	]);
	assert.deepEqual([...retired], ['abandoned']);
});

test('an output a newer one superseded leaves the Library', () => {
	const files = libraryFiles(
		[
			record({
				id: 'old',
				uri: '/company/outputs/site-v1/index.html',
				superseded_at: '2026-10-12T00:00:00Z'
			}),
			record({ id: 'new', uri: '/company/outputs/site-v2/index.html' })
		],
		[{ id: 'w1', title: 'Site' }]
	);
	assert.deepEqual(
		files.map((file) => file.id),
		['new']
	);
});
