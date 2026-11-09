import { test } from 'node:test';
import assert from 'node:assert/strict';
import { companyFileLinks, companyPathsIn } from './inbox-files.ts';

test('every way an agent names a company file is found once', () => {
	assert.deepEqual(
		companyPathsIn(
			'The packs are in /company/outputs/segment-test/SEND-PACKS.md, follow-ups in `outputs/segment-test/follow-ups.csv`.\n' +
				'See [the pack](/company/outputs/segment-test/SEND-PACKS.md), ![a](file:///company/outputs/magnet/a%20b.png) and company/downloads/brief.pdf.'
		),
		[
			'/company/outputs/segment-test/SEND-PACKS.md',
			'/company/outputs/segment-test/follow-ups.csv',
			'/company/outputs/magnet/a b.png',
			'/company/downloads/brief.pdf'
		]
	);
});

test('a folder, a web address and a climb out of the home are not files', () => {
	assert.deepEqual(
		companyPathsIn(
			'Folder /company/outputs/segment-test/ and https://example.com/company/a.png and https://x.io/outputs/b.png and /company/../etc/passwd.txt'
		),
		[]
	);
});

test('a named file opens by its recorded id when there is one, else by path', () => {
	const link = companyFileLinks('reno', [
		{
			id: 'a1',
			kind: 'file',
			uri: '/company/outputs/known.md',
			label: 'Known',
			created_by: 'exec',
			work_id: 'w1',
			state: 'available',
			created_at: '2026-11-09T00:00:00Z'
		}
	]);
	assert.equal(link('/company/outputs/known.md')?.href, '/reno/library?file=a1');
	assert.equal(
		link('/company/outputs/segment-test/SEND-PACKS.md')?.href,
		'/reno/library?path=%2Fcompany%2Foutputs%2Fsegment-test%2FSEND-PACKS.md'
	);
});
