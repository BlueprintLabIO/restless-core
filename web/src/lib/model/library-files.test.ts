import assert from 'node:assert/strict';
import test from 'node:test';
import { libraryFileById, libraryFiles, type LibraryArtifact } from './library-files.ts';

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
