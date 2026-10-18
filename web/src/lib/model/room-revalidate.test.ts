import assert from 'node:assert/strict';
import test from 'node:test';
import { QueryClient, QueryObserver } from '@tanstack/query-core';
import { revalidateAfterEvent } from './room-revalidate.ts';

/* The server's messages; the first read is slow and starts before anyone has written. */
function room() {
	let messages: string[] = [];
	let reads = 0;
	const read = async () => {
		reads += 1;
		const seen = [...messages];
		await new Promise((resolve) => setTimeout(resolve, reads === 1 ? 50 : 5));
		return seen;
	};
	return {
		write: (text: string) => (messages = [...messages, text]),
		read,
		get reads() {
			return reads;
		}
	};
}

test('a message written while the first read is in flight is not lost', async () => {
	const server = room();
	const client = new QueryClient();
	const key = ['room-messages', 'acme', 'launch'];
	const observer = new QueryObserver(client, { queryKey: key, queryFn: server.read });
	const unsubscribe = observer.subscribe(() => {});
	await new Promise((resolve) => setTimeout(resolve, 5));
	// Two people write while the newcomer's first read is still on its way.
	server.write('Owner: kickoff at nine.');
	await revalidateAfterEvent(client, key);
	assert.deepEqual(client.getQueryData(key), ['Owner: kickoff at nine.']);
	assert.equal(server.reads, 2, 'read once more after the first read landed');
	unsubscribe();
	client.clear();
});

test('an event after the first answer revalidates once, as before', async () => {
	const server = room();
	const client = new QueryClient();
	const key = ['room-messages', 'acme', 'launch'];
	const observer = new QueryObserver(client, { queryKey: key, queryFn: server.read });
	const unsubscribe = observer.subscribe(() => {});
	await new Promise((resolve) => setTimeout(resolve, 80));
	server.write('Alice: I will bring the draft.');
	await revalidateAfterEvent(client, key);
	assert.deepEqual(client.getQueryData(key), ['Alice: I will bring the draft.']);
	assert.equal(server.reads, 2);
	unsubscribe();
	client.clear();
});
