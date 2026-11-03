import assert from 'node:assert/strict';
import test from 'node:test';
import { speakable, speechToText, textToSpeech } from './voice.ts';

test('speech to text is the browser unless a choice that runs here is kept', () => {
	assert.equal(speechToText(undefined), 'browser');
	assert.equal(speechToText('browser'), 'browser');
	/* A provider is listed but has no route on this version: dictation keeps working. */
	assert.equal(speechToText('openai'), 'browser');
	assert.equal(speechToText('deepgram'), 'browser');
	assert.equal(speechToText(42), 'browser');
});

test('text to speech is off until turned on', () => {
	assert.equal(textToSpeech(undefined), 'off');
	assert.equal(textToSpeech('browser'), 'browser');
	assert.equal(textToSpeech('loud'), 'off');
});

test('a message is heard as words, not Markdown', () => {
	assert.equal(
		speakable(
			'## Ready\n\n**Invoice 12** is sent to [Ana](https://x.test/a).\n\n- Paid `$40`\n\n```\nlog\n```'
		),
		'Ready\nInvoice 12 is sent to Ana.\nPaid $40'
	);
	assert.equal(speakable('   '), '');
});
