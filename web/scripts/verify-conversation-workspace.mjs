// Browser-only fixture. Every write is intercepted; the source company is read-only.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';

const source = process.env.RESTLESS_TEST_SOURCE_COMPANY;
assert(source, 'Set RESTLESS_TEST_SOURCE_COMPANY for read-only shell data');
const base = process.env.RESTLESS_TEST_BASE_URL ?? 'http://127.0.0.1:8788';
const company = 'conversation_workspace_test';
const roomId = '22222222-2222-4222-8222-222222222222';
const directRoomId = '33333333-3333-4333-8333-333333333333';
const now = new Date().toISOString();

const sourceCockpitResponse = await fetch(
	`${base}/api/companies/${encodeURIComponent(source)}/cockpit`
);
assert.equal(sourceCockpitResponse.ok, true, 'source cockpit fixture is available');
const sourceCockpit = await sourceCockpitResponse.json();
const exec = sourceCockpit.people.find((person) => person.kind === 'exec');
const addedPerson = sourceCockpit.people.find(
	(person) =>
		person.kind !== 'owner' && person.kind !== 'system' && person.actor_id !== exec?.actor_id
);
const leadIds = new Set(sourceCockpit.teams.map((team) => team.lead_actor_id));
const directPartner = sourceCockpit.people.find(
	(person) =>
		person.kind !== 'owner' &&
		person.kind !== 'system' &&
		person.actor_id !== exec?.actor_id &&
		!leadIds.has(person.actor_id)
);
assert(
	exec && addedPerson && directPartner,
	'source company exposes an Exec, another person, and a non-contact direct partner'
);

function directCanonicalKey(firstActor, secondActor) {
	const [first, second] = [firstActor, secondActor].sort();
	return `direct:${Buffer.byteLength(first)}:${first}:${Buffer.byteLength(second)}:${second}`;
}

const directRoom = {
	id: directRoomId,
	kind: 'direct',
	title: `${directPartner.display} / owner`,
	created_by: 'owner',
	canonical_key: directCanonicalKey('owner', directPartner.actor_id),
	created_at: now,
	archived_at: null
};
const searchRootMessage = {
	id: 777,
	room_id: roomId,
	from_actor: addedPerson.actor_id,
	to_actor: null,
	body: 'Quarterly planning focus',
	outcome_standard: null,
	parent_message_id: null,
	thread_root_message_id: null,
	client_command_id: null,
	created_at: now,
	revision_number: 0,
	edited_at: null,
	deleted_at: null,
	legacy_read_at: null
};
const searchThreadMessage = {
	...searchRootMessage,
	id: 778,
	from_actor: directPartner.actor_id,
	body: 'Thread target detail',
	parent_message_id: searchRootMessage.id,
	thread_root_message_id: searchRootMessage.id
};

const sourceDocumentsResponse = await fetch(
	`${base}/api/companies/${encodeURIComponent(source)}/documents?limit=30`
);
assert.equal(sourceDocumentsResponse.ok, true, 'source document fixture is available');
const sourceDocuments = await sourceDocumentsResponse.json();
const document = sourceDocuments.items?.[0]?.document;
assert(document, 'source company exposes a document');

const privateContext = 'Private history remains here; carry only this editable excerpt.';
const revisedContext = 'Shared context reviewed by the owner before opening the group.';
async function eventually(read, check, timeoutMs = 5_000) {
	const deadline = Date.now() + timeoutMs;
	let lastFailure;
	do {
		try {
			const value = await read();
			check(value);
			return value;
		} catch (error) {
			lastFailure = error;
			await new Promise((resolve) => setTimeout(resolve, 50));
		}
	} while (Date.now() < deadline);
	throw lastFailure;
}
const browser = await chromium.launch({ headless: true });
let closing = false;
let releaseDirectParticipants;

try {
	const context = await browser.newContext({ serviceWorkers: 'block' });
	const page = await context.newPage();
	const pageErrors = [];
	const unexpectedWrites = [];
	const mockedWebSockets = [];
	let directCreateAttempts = 0;
	let createAttempts = 0;
	let createCommandId = '';
	let createdRoom = null;
	let sentMessage = null;
	let sentMessageCount = 0;
	let directParticipantRequestStarted = false;
	const directParticipantsReady = new Promise((resolve) => {
		releaseDirectParticipants = resolve;
	});

	page.on('pageerror', (error) => pageErrors.push(error.message));
	await page.routeWebSocket('**', async (websocket) => {
		mockedWebSockets.push(websocket.url());
		await websocket.close({ code: 1000, reason: 'Fixture blocks live collaboration sockets' });
	});
	await page.route('**/api/**', async (route) => {
		try {
			const request = route.request();
			const url = new URL(request.url());
			const prefix = `/api/companies/${company}`;
			if (!url.pathname.startsWith(prefix)) return route.continue();
			const path = url.pathname.slice(prefix.length);

			if (path === `/actors/${encodeURIComponent(exec.actor_id)}/conversation`) {
				assert.equal(request.method(), 'GET', 'private conversation remains read-only');
				return route.fulfill({
					json: {
						actor: {
							id: exec.actor_id,
							display: exec.display,
							kind: exec.kind,
							role: exec.role
						},
						messages: [
							{
								id: 901,
								from_actor: exec.actor_id,
								to_actor: null,
								body: privateContext,
								outcome_standard: null,
								attachments: [],
								details: null,
								intent: null,
								context_path: null,
								created_at: now,
								read_at: null
							}
						]
					}
				});
			}

			if (path === '/rooms') {
				if (request.method() === 'POST') {
					const input = request.postDataJSON();
					if (input.kind === 'direct') {
						directCreateAttempts += 1;
						assert.deepEqual(input.participant_actor_ids, [directPartner.actor_id]);
						return route.fulfill({ json: directRoom });
					}
					createAttempts += 1;
					assert.equal(input.kind, 'group');
					assert.deepEqual(
						new Set(input.participant_actor_ids),
						new Set([exec.actor_id, addedPerson.actor_id]),
						'new group keeps the private participant and adds the selected person'
					);
					if (createAttempts === 1) {
						createCommandId = input.command_id;
						return route.fulfill({
							status: 503,
							json: { message: 'Temporary connection failure' }
						});
					}
					assert.equal(input.command_id, createCommandId, 'retry retains room command identity');
					createdRoom = {
						id: roomId,
						kind: 'group',
						title: input.title,
						created_by: 'owner',
						canonical_key: null,
						created_at: now,
						archived_at: null
					};
					return route.fulfill({ json: createdRoom });
				}
				return route.fulfill({
					json: {
						rooms: createdRoom ? [createdRoom, directRoom] : [directRoom],
						has_more: false,
						next_before_created_at: null,
						next_before_room_id: null
					}
				});
			}
			if (path === '/rooms/search') {
				assert.equal(request.method(), 'GET');
				return route.fulfill({
					json: {
						rooms: [],
						has_more: false,
						next_before_created_at: null,
						next_before_room_id: null
					}
				});
			}
			if (path === '/room-messages/search') {
				assert.equal(request.method(), 'GET');
				const query = url.searchParams.get('q')?.toLocaleLowerCase() ?? '';
				const messages = query.includes('quarterly')
					? [
							{
								id: searchRootMessage.id,
								room_id: roomId,
								from_actor: searchRootMessage.from_actor,
								thread_root_message_id: null,
								snippet: searchRootMessage.body
							}
						]
					: query.includes('thread')
						? [
								{
									id: searchThreadMessage.id,
									room_id: roomId,
									from_actor: searchThreadMessage.from_actor,
									thread_root_message_id: searchRootMessage.id,
									snippet: searchThreadMessage.body
								}
							]
						: [];
				return route.fulfill({
					json: { messages, has_more: false, next_before_message_id: null }
				});
			}
			if (path === `/rooms/${directRoomId}/participants`) {
				assert.equal(request.method(), 'GET');
				directParticipantRequestStarted = true;
				await directParticipantsReady;
				return route.fulfill({
					json: {
						participants: ['owner', directPartner.actor_id].map((actor_id) => ({
							room_id: directRoomId,
							actor_id,
							role: actor_id === 'owner' ? 'owner' : 'member',
							joined_at: now,
							left_at: null
						}))
					}
				});
			}
			if (path === `/rooms/${directRoomId}/read-cursor`) {
				if (request.method() === 'POST') {
					return route.fulfill({
						json: {
							room_id: directRoomId,
							actor_id: 'owner',
							last_read_message_id: request.postDataJSON().through_message_id,
							updated_at: now
						}
					});
				}
				assert.equal(request.method(), 'GET');
				return route.fulfill({ json: { actor_id: 'owner', cursor: null } });
			}
			if (path === `/rooms/${directRoomId}/messages`) {
				assert.equal(request.method(), 'GET');
				return route.fulfill({
					json: {
						messages: [
							{
								id: 801,
								room_id: directRoomId,
								from_actor: directPartner.actor_id,
								to_actor: null,
								body: 'Direct history remains private while participants load.',
								outcome_standard: null,
								parent_message_id: null,
								thread_root_message_id: null,
								client_command_id: null,
								created_at: now,
								revision_number: 0,
								edited_at: null,
								deleted_at: null,
								legacy_read_at: null
							}
						],
						mentions: [],
						has_more: false,
						next_before_message_id: null
					}
				});
			}
			if (path === `/rooms/${directRoomId}/events`) {
				return route.fulfill({
					json: {
						events: [],
						requested_after_event_id: 0,
						next_after_event_id: 0,
						snapshot_cursor: 0,
						compacted_through_event_id: 0,
						oldest_available_event_id: null,
						has_more: false,
						resync_required: false
					}
				});
			}
			if (path === `/rooms/${directRoomId}/events/live`) {
				assert.equal(request.method(), 'GET');
				return route.fulfill({ body: ': fixture activity\n\n', contentType: 'text/event-stream' });
			}

			if (path === `/rooms/${roomId}/participants`) {
				assert.equal(request.method(), 'GET');
				return route.fulfill({
					json: {
						participants: ['owner', exec.actor_id, addedPerson.actor_id].map((actor_id) => ({
							room_id: roomId,
							actor_id,
							role: actor_id === 'owner' ? 'owner' : 'member',
							joined_at: now,
							left_at: null
						}))
					}
				});
			}
			if (path === `/rooms/${roomId}/read-cursor`) {
				if (request.method() === 'POST') {
					const through = request.postDataJSON().through_message_id;
					return route.fulfill({
						json: {
							room_id: roomId,
							actor_id: 'owner',
							last_read_message_id: through,
							updated_at: now
						}
					});
				}
				assert.equal(request.method(), 'GET');
				return route.fulfill({ json: { actor_id: 'owner', cursor: null } });
			}
			if (path === `/rooms/${roomId}/messages`) {
				if (request.method() === 'POST') {
					const input = request.postDataJSON();
					sentMessageCount += 1;
					sentMessage = input;
					return route.fulfill({
						json: {
							message: {
								id: 902,
								room_id: roomId,
								from_actor: 'owner',
								to_actor: null,
								body: input.body,
								outcome_standard: null,
								parent_message_id: null,
								thread_root_message_id: null,
								client_command_id: input.command_id,
								created_at: now,
								revision_number: 0,
								edited_at: null,
								deleted_at: null,
								legacy_read_at: null
							},
							event_id: 1,
							created: true,
							mentions: [],
							resolved_mention: null
						}
					});
				}
				return route.fulfill({
					json: {
						messages: [searchRootMessage],
						mentions: [],
						has_more: false,
						next_before_message_id: null
					}
				});
			}
			if (path === `/rooms/${roomId}/threads/${searchRootMessage.id}`) {
				assert.equal(request.method(), 'GET');
				return route.fulfill({
					json: {
						messages: [searchRootMessage, searchThreadMessage],
						mentions: [],
						has_more: false,
						next_before_message_id: null
					}
				});
			}
			if (path === `/rooms/${roomId}/events`) {
				return route.fulfill({
					json: {
						events: [],
						requested_after_event_id: 0,
						next_after_event_id: 0,
						snapshot_cursor: 0,
						compacted_through_event_id: 0,
						oldest_available_event_id: null,
						has_more: false,
						resync_required: false
					}
				});
			}
			if (path === `/rooms/${roomId}/events/live`) {
				assert.equal(request.method(), 'GET');
				return route.fulfill({ body: ': fixture activity\n\n', contentType: 'text/event-stream' });
			}
			if (/^\/documents\/[^/]+\/collaboration\/token$/u.test(path)) {
				assert.equal(request.method(), 'POST');
				return route.fulfill({
					json: {
						token: 'fixture-collaboration-token',
						token_type: 'Bearer',
						access: 'write',
						expires_at: new Date(Date.now() + 60_000).toISOString()
					}
				});
			}

			if (request.method() !== 'GET') {
				unexpectedWrites.push(`${request.method()} ${path}`);
				return route.fulfill({ status: 500, json: { message: 'Unexpected fixture write' } });
			}
			const sourceUrl = request.url().replace(company, source);
			const response = await route.fetch({ url: sourceUrl });
			const body = (await response.text()).replaceAll(source, company);
			return route.fulfill({ response, body });
		} catch (error) {
			if (!closing) throw error;
		}
	});

	await page.goto(`${base}/${company}/people/rooms?person=${encodeURIComponent(exec.actor_id)}`);
	await page.getByRole('heading', { name: 'People', exact: true }).waitFor();
	assert.equal(new URL(page.url()).pathname, `/${company}/people`, 'legacy Rooms URL redirects');

	await page.getByRole('button', { name: 'New conversation', exact: true }).click();
	await page.getByRole('checkbox', { name: exec.display, exact: true }).check();
	await page.getByRole('button', { name: 'Start conversation', exact: true }).click();
	await page.waitForURL((url) => url.searchParams.get('person') === exec.actor_id);
	assert.equal(
		createAttempts,
		0,
		'opening an existing accountable contact does not fork its history'
	);
	await page.getByRole('button', { name: 'New conversation', exact: true }).click();
	await page.getByRole('checkbox', { name: directPartner.display, exact: true }).check();
	await page.getByRole('button', { name: 'Start conversation', exact: true }).click();
	await page.waitForURL((url) => url.searchParams.get('room') === directRoomId);
	assert.equal(directCreateAttempts, 1, 'one non-contact creates a direct Room');

	await eventually(
		() => directParticipantRequestStarted,
		(started) => assert.equal(started, true, 'direct participant projection starts loading')
	);
	const guardedAddPeople = page.getByRole('button', { name: 'Add people', exact: true });
	await eventually(
		async () => {
			const count = await guardedAddPeople.count();
			return { count, blocked: count === 0 || (await guardedAddPeople.isDisabled()) };
		},
		(state) => assert.equal(state.blocked, true, 'Add people is blocked until participants load')
	);
	releaseDirectParticipants();
	await eventually(
		async () => ({
			count: await guardedAddPeople.count(),
			disabled: await guardedAddPeople.isDisabled()
		}),
		(state) => {
			assert.equal(state.count, 1);
			assert.equal(
				state.disabled,
				false,
				'Add people becomes available with complete participants'
			);
		}
	);
	await guardedAddPeople.click();
	const guardedDialog = page.getByRole('dialog');
	const retainedDirectPartner = guardedDialog.getByRole('checkbox', {
		name: directPartner.display,
		exact: true
	});
	assert.equal(await retainedDirectPartner.isChecked(), true);
	assert.equal(await retainedDirectPartner.isDisabled(), true);
	await guardedDialog.getByRole('button', { name: 'Close', exact: true }).click();
	await page.goto(`${base}/${company}/people?person=${encodeURIComponent(exec.actor_id)}`);

	await page.getByText(privateContext, { exact: true }).waitFor();
	await page.getByRole('button', { name: 'Add people', exact: true }).click();
	const addDialog = page.getByRole('dialog');
	const existing = addDialog.getByRole('checkbox', { name: exec.display, exact: true });
	assert.equal(await existing.isChecked(), true, 'existing private participant remains selected');
	assert.equal(
		await existing.isDisabled(),
		true,
		'existing private participant cannot be unchecked'
	);
	await addDialog.getByRole('checkbox', { name: addedPerson.display, exact: true }).check();
	const contextDraft = addDialog.getByLabel('Context to carry forward (optional)');
	assert.equal(
		await contextDraft.inputValue(),
		privateContext,
		'latest private text is editable context'
	);
	await contextDraft.fill(revisedContext);
	await addDialog.getByRole('button', { name: 'Start group conversation', exact: true }).click();
	await addDialog.getByRole('alert').filter({ hasText: 'Temporary connection failure' }).waitFor();
	await addDialog.getByRole('button', { name: 'Start group conversation', exact: true }).click();
	await page.waitForURL((url) => url.searchParams.get('room') === roomId);
	assert.equal(createAttempts, 2);
	assert.equal(
		await page.getByText(privateContext, { exact: true }).count(),
		0,
		'private history is not copied'
	);
	await page.getByRole('button', { name: `Pin ${createdRoom.title}`, exact: true }).click();
	await page.reload();
	await page.getByRole('heading', { name: 'People', exact: true }).waitFor();
	assert.equal(
		await page.locator('aside[aria-label="Conversations"] .entry .name').first().textContent(),
		createdRoom.title,
		'pinned conversation remains first after reload'
	);

	const composer = page.getByLabel('Conversation message', { exact: true });
	await eventually(
		() => composer.inputValue(),
		(value) => assert.equal(value, revisedContext)
	);
	await page.getByLabel('Ask someone to reply', { exact: true }).selectOption(addedPerson.actor_id);
	assert.match(
		await composer.inputValue(),
		new RegExp(`@${addedPerson.actor_id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*$`)
	);

	const draftBeforeSwitch = await composer.inputValue();
	await page
		.locator('aside[aria-label="Conversations"]')
		.locator(`a[href="/${company}/people?person=${encodeURIComponent(exec.actor_id)}"]`)
		.click();
	await page.getByText(privateContext, { exact: true }).waitFor();
	await page
		.locator('aside[aria-label="Conversations"]')
		.locator(`a[href="/${company}/people?room=${roomId}"]`)
		.click();
	await eventually(
		() => page.getByLabel('Conversation message', { exact: true }).inputValue(),
		(value) => assert.equal(value, draftBeforeSwitch)
	);

	await page
		.getByLabel('Conversation message', { exact: true })
		.fill(`Please reply @${addedPerson.actor_id}`);
	await page.getByRole('button', { name: 'Send', exact: true }).click();
	await eventually(
		() => sentMessage,
		(input) => {
			assert.equal(input.body, `Please reply @${addedPerson.actor_id}`);
			assert.deepEqual(input.mentions, [{ actor_id: addedPerson.actor_id }]);
		}
	);
	const globalSearch = page.getByLabel('Search conversations and people', { exact: true });
	await globalSearch.fill('quarterly');
	await page.getByRole('link', { name: /Quarterly planning focus/u }).click();
	await page.waitForURL(
		(url) => url.searchParams.get('room') === roomId && url.searchParams.get('focus') === '777'
	);
	await page
		.locator('.room-conversation .room-message.targeted')
		.filter({ hasText: searchRootMessage.body })
		.waitFor();

	await globalSearch.fill('thread target');
	await page.getByRole('link', { name: /Thread target detail/u }).click();
	await page.waitForURL(
		(url) =>
			url.searchParams.get('room') === roomId &&
			url.searchParams.get('thread') === '777' &&
			url.searchParams.get('focus') === '778'
	);
	await page
		.locator('.thread-pane .room-message.targeted')
		.filter({ hasText: searchThreadMessage.body })
		.waitFor();
	const threadPane = page.getByRole('complementary', { name: 'Thread' });
	await page.getByRole('button', { name: 'Open document', exact: true }).click();
	await page
		.getByLabel('Open document alongside conversation', { exact: true })
		.selectOption(document.id);
	await threadPane.getByRole('button', { name: 'Include document link', exact: true }).click();
	const expectedDocumentLink = new URL(`/${company}/work/documents`, base);
	expectedDocumentLink.searchParams.set('document', document.id);
	assert.equal(
		await threadPane.getByLabel('Thread reply', { exact: true }).inputValue(),
		expectedDocumentLink.href,
		'document link is added to the unsent Thread draft'
	);
	assert.equal(sentMessageCount, 1, 'including a document link does not send a message');
	const mainPanel = page.getByRole('region', { name: 'Selected conversation' });
	const threadSend = threadPane.getByRole('button', { name: 'Send', exact: true });
	const [mainBox, threadBox, sendBox] = await Promise.all([
		mainPanel.boundingBox(),
		threadPane.boundingBox(),
		threadSend.boundingBox()
	]);
	assert(mainBox && threadBox && sendBox, 'thread controls are rendered with the document open');
	assert.ok(
		threadBox.x >= mainBox.x && threadBox.x + threadBox.width <= mainBox.x + mainBox.width + 1,
		'thread pane remains inside the narrowed conversation panel'
	);
	assert.ok(
		sendBox.x + sendBox.width <= mainBox.x + mainBox.width + 1,
		'thread Send control remains visible beside the document'
	);
	await page.goto(`${base}/${company}/work/documents?document=${encodeURIComponent(document.id)}`);
	await page.getByRole('link', { name: 'Discuss alongside', exact: true }).click();
	await page.waitForURL(
		(url) =>
			url.pathname === `/${company}/people` &&
			url.searchParams.get('person') === 'exec' &&
			url.searchParams.get('document') === document.id
	);
	await page.getByRole('complementary', { name: 'Conversation document' }).waitFor();

	await eventually(
		() => mockedWebSockets.length,
		(socketRequests) =>
			assert.ok(socketRequests >= 1, 'document collaboration socket is fixture-blocked')
	);
	assert.deepEqual(unexpectedWrites, [], 'fixture observed no unapproved company writes');
	assert.deepEqual(pageErrors, [], 'browser emitted no uncaught errors');
	console.log(
		'PASS legacy redirect, direct open, participant loading guard, group retry/context privacy, pin persistence, draft switching, reply target, document link/return path, focused search/thread layout; no live writes'
	);
} finally {
	closing = true;
	releaseDirectParticipants?.();
	for (const context of browser.contexts()) {
		for (const page of context.pages()) await page.unrouteAll({ behavior: 'ignoreErrors' });
	}
	await browser.close();
}
