import assert from 'node:assert/strict';
import test from 'node:test';
import { linkCockpitPlaces } from './cockpit-places.ts';

const root = '/reno';

test('a Needs you line links the cockpit places it names', () => {
	assert.equal(
		linkCockpitPlaces('Needs you: set the purpose in Company › Charter.', root),
		'Needs you: set the purpose in [Company › Charter](/reno/company/charter).'
	);
	assert.equal(
		linkCockpitPlaces(
			'- **Needs you:** connect a key in Company → Intelligence, then open Apps.',
			root
		),
		'- **Needs you:** connect a key in [Company → Intelligence](/reno/company/provider), then open [Apps](/reno/apps).'
	);
});

test('Company › X links anywhere; a bare section name only on a Needs you line', () => {
	assert.equal(
		linkCockpitPlaces('Raise the cap in Company › Limits. The Library has it.', root),
		'Raise the cap in [Company › Limits](/reno/company/resources). The Library has it.'
	);
	assert.equal(
		linkCockpitPlaces('Brand voice lives in Company > Brand & voice', root),
		'Brand voice lives in [Company > Brand & voice](/reno/company/identity)'
	);
});

test('links, code and fenced blocks are left alone', () => {
	const already = 'See [Company › Charter](/elsewhere) and `Company › Limits`.';
	assert.equal(linkCockpitPlaces(already, root), already);
	const fence = '```\nNeeds you: Company › Charter\n```';
	assert.equal(linkCockpitPlaces(fence, root), fence);
});

test('nothing changes without a company or a place', () => {
	assert.equal(
		linkCockpitPlaces('Needs you: Company › Charter', ''),
		'Needs you: Company › Charter'
	);
	assert.equal(linkCockpitPlaces('Company is doing well.', root), 'Company is doing well.');
});
