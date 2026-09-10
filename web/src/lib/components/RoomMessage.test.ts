import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';
import type { RoomMessage } from '../model/rooms';

const message: RoomMessage = {
	id: 75,
	room_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
	from_actor: 'alice',
	to_actor: null,
	body: 'Please review this exact message.',
	outcome_standard: null,
	parent_message_id: 10,
	thread_root_message_id: 10,
	client_command_id: 'command-75',
	created_at: '2026-09-10T00:00:00Z',
	revision_number: 0,
	edited_at: null,
	deleted_at: null,
	legacy_read_at: null
};

test('a targeted Room message renders one accessible programmatic focus anchor', async (context) => {
	const server = await createServer({
		server: { middlewareMode: true },
		appType: 'custom',
		logLevel: 'silent'
	});
	context.after(() => server.close());
	const [{ default: RoomMessage }, { render }] = await Promise.all([
		server.ssrLoadModule('/src/lib/components/RoomMessage.svelte'),
		server.ssrLoadModule('svelte/server')
	]);

	const targeted = render(RoomMessage, {
		props: { message, author: 'Alice', targeted: true, focusKey: 'mention-75' }
	}).body as string;
	const ordinary = render(RoomMessage, {
		props: { message, author: 'Alice' }
	}).body as string;

	assert.match(targeted, /class="room-message[^"].*targeted" tabindex="-1"/);
	assert.match(targeted, />Linked mention message\.<\/span>/);
	assert.doesNotMatch(ordinary, /Linked mention message\./);
});
