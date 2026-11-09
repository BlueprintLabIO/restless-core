import { test } from 'node:test';
import assert from 'node:assert/strict';
import { isSiteTask } from './site-ask.ts';

test('a decision that names sites and addresses is not a task on a screen', () => {
	assert.equal(
		isSiteTask(
			'Approve sending the four prospect emails',
			'1. Coast Physio (coastphysio.com.au) - jo@coastphysio.com.au\n2. https://harbourphysio.com.au'
		),
		false
	);
	assert.equal(isSiteTask('Allow the test-send permission?', 'https://example.com'), false);
	assert.equal(isSiteTask('Do you approve paying the invoice?', ''), false);
});

test('a card that asks to send finished work is a decision, whatever the message mentions', () => {
	assert.equal(
		isSiteTask(
			'Send the clinics, professional services and premium home packs, plus the 11 follow-ups',
			'I checked them on the company computer. The packs are in /company/outputs/SEND-PACKS.md'
		),
		false
	);
	assert.equal(isSiteTask('Should we publish the pricing page?', 'It is open in the company browser.'), false);
});

test('a link or a domain alone is evidence, not a site task', () => {
	assert.equal(isSiteTask('Which of these looks best?', 'https://a.example.com and b.io'), false);
});

test('asking the owner to act on a site shows the computer', () => {
	assert.equal(isSiteTask('Please sign in to Namecheap so I can register the domain'), true);
	assert.equal(isSiteTask('Can you pay the $12 renewal?'), true);
	assert.equal(isSiteTask('Can you finish this?', 'It is open in the company browser.'), true);
});
