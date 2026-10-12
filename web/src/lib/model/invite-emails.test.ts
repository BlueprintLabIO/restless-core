import assert from 'node:assert/strict';
import test from 'node:test';

import { looksLikeEmail, parseInviteEmails } from './invite-emails.ts';

test('several pasted addresses become one invitation each', () => {
	assert.deepEqual(
		parseInviteEmails('ana@example.com, ben@example.com; cy@example.org\nana@EXAMPLE.com  dee@example.co.uk'),
		{
			emails: ['ana@example.com', 'ben@example.com', 'cy@example.org', 'dee@example.co.uk'],
			invalid: []
		}
	);
});

test('a mail client copy keeps only the address, and what is not an address is named', () => {
	assert.deepEqual(parseInviteEmails('Bianca Rossi <bianca@example.com>, sam, x@y'), {
		emails: ['bianca@example.com'],
		invalid: ['sam', 'x@y']
	});
});

test('a name is not an email, so Share keeps looking it up as a name', () => {
	assert.equal(looksLikeEmail('Bianca'), false);
	assert.equal(looksLikeEmail('bianca@'), false);
	assert.equal(looksLikeEmail(' bianca@example.com '), true);
});
