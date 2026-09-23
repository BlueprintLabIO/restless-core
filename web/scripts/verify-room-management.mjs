// Browser-only state. Every write is intercepted; source company is read-only.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';

const source = process.env.RESTLESS_TEST_SOURCE_COMPANY;
assert(source, 'Set RESTLESS_TEST_SOURCE_COMPANY for read-only shell data');
const base = process.env.RESTLESS_TEST_BASE_URL ?? 'http://127.0.0.1:8788';
const company = 'room_management_test';
const roomId = '11111111-1111-4111-8111-111111111111';
const now = new Date().toISOString();

const cockpitResponse = await fetch(`${base}/api/companies/${encodeURIComponent(source)}/cockpit`);
assert.equal(cockpitResponse.ok, true, 'source cockpit fixture is available');
const cockpit = await cockpitResponse.json();
const selectable = cockpit.people.filter(
	(person) => person.kind !== 'owner' && person.kind !== 'system'
);
assert(selectable.length >= 3, 'source company exposes three selectable people');
const initialMembers = selectable.slice(0, 2);
const addedMember = selectable[2];

const browser = await chromium.launch({ headless: true });
let closing = false;
try {
	const context = await browser.newContext({ serviceWorkers: 'block' });
	const page = await context.newPage({ viewport: { width: 1440, height: 900 } });
	const errors = [];
	page.on('pageerror', (error) => errors.push(error.message));
	let rooms = [];
	let members = [];
	let lastCommandId = '';
	let failCreate = true;

	await page.route('**/api/**', async (route) => {
		try {
			const request = route.request();
			const url = new URL(request.url());
			const prefix = `/api/companies/${company}`;
			if (!url.pathname.startsWith(prefix)) return route.continue();
			const path = url.pathname.slice(prefix.length);

			if (path === '/rooms') {
				if (request.method() === 'POST') {
					const input = request.postDataJSON();
					assert.equal(input.kind, 'group', 'two people create a group conversation');
					assert.deepEqual(
						new Set(input.participant_actor_ids),
						new Set(initialMembers.map((person) => person.actor_id))
					);
					if (failCreate) {
						failCreate = false;
						lastCommandId = input.command_id;
						return route.fulfill({
							status: 503,
							json: { message: 'Temporary connection failure' }
						});
					}
					assert.equal(input.command_id, lastCommandId, 'retry retains command identity');
					rooms = [
						{
							id: roomId,
							kind: 'group',
							title: input.title,
							created_by: 'owner',
							canonical_key: null,
							created_at: now,
							archived_at: null
						}
					];
					members = ['owner', ...input.participant_actor_ids].map((actor_id) => ({
						room_id: roomId,
						actor_id,
						role: actor_id === 'owner' ? 'owner' : 'member',
						joined_at: now,
						left_at: null
					}));
					return route.fulfill({ json: rooms[0] });
				}
				return route.fulfill({
					json: {
						rooms,
						has_more: false,
						next_before_created_at: null,
						next_before_room_id: null
					}
				});
			}

			if (path === `/rooms/${roomId}/participants`) {
				if (request.method() === 'POST') {
					const actorId = request.postDataJSON().actor_id;
					const participant = {
						room_id: roomId,
						actor_id: actorId,
						role: 'member',
						joined_at: now,
						left_at: null
					};
					members.push(participant);
					return route.fulfill({ json: participant });
				}
				assert.equal(request.method(), 'GET');
				return route.fulfill({ json: { participants: members } });
			}
			if (path.startsWith(`/rooms/${roomId}/participants/`)) {
				assert.equal(request.method(), 'DELETE');
				const actorId = decodeURIComponent(path.split('/').at(-1));
				const participant = members.find((member) => member.actor_id === actorId);
				members = members.filter((member) => member.actor_id !== actorId);
				return route.fulfill({ json: { ...participant, left_at: now } });
			}
			if (path === `/rooms/${roomId}/read-cursor`) {
				assert.equal(request.method(), 'GET');
				return route.fulfill({ json: { actor_id: 'owner', cursor: null } });
			}
			if (path === `/rooms/${roomId}/messages`) {
				assert.equal(request.method(), 'GET');
				return route.fulfill({
					json: {
						messages: [],
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
				return route.fulfill({
					body: ': no fixture activity\n\n',
					contentType: 'text/event-stream'
				});
			}

			if (request.method() !== 'GET') {
				throw new Error(`Unexpected write ${request.method()} ${path}`);
			}
			const response = await route.fetch({ url: request.url().replace(company, source) });
			const body = (await response.text()).replaceAll(source, company);
			await route.fulfill({ response, body });
		} catch (error) {
			if (!closing) throw error;
		}
	});

	await page.goto(`${base}/${company}/people`);
	await page.getByRole('heading', { name: 'Conversations', exact: true }).waitFor();
	await page.getByRole('button', { name: 'New conversation', exact: true }).click();
	const createDialog = page.locator('dialog[open]');
	await createDialog
		.getByLabel('Conversation name (optional)', { exact: true })
		.fill('Shared planning');
	for (const person of initialMembers) {
		await createDialog.getByRole('checkbox', { name: person.display, exact: true }).check();
	}
	await createDialog.getByRole('button', { name: 'Start conversation', exact: true }).click();
	await createDialog
		.getByRole('alert')
		.filter({ hasText: 'Temporary connection failure' })
		.waitFor();
	await createDialog.getByRole('button', { name: 'Start conversation', exact: true }).click();
	await page.waitForURL((url) => url.searchParams.get('room') === roomId);

	await page.getByRole('button', { name: 'Participants', exact: true }).click();
	await page.getByLabel('Add a member', { exact: true }).selectOption(addedMember.actor_id);
	const removeAdded = page.getByRole('button', {
		name: `Remove ${addedMember.display}`,
		exact: true
	});
	await removeAdded.waitFor();
	await removeAdded.click();
	await removeAdded.waitFor({ state: 'detached' });
	await page.keyboard.press('Escape');
	assert.equal(await page.locator('dialog[open]').count(), 0);

	await page.setViewportSize({ width: 390, height: 844 });
	await page.getByRole('button', { name: 'Participants', exact: true }).click();
	const box = await page.locator('dialog[open]').boundingBox();
	assert(box && box.x >= 0 && box.x + box.width <= 390, 'dialog fits mobile viewport');
	await page.keyboard.press('Escape');
	assert.deepEqual(errors, []);
	console.log(
		'PASS group create/retry, participant add/remove, keyboard dismiss, mobile bounds; no live company writes'
	);
} finally {
	closing = true;
	for (const context of browser.contexts()) {
		for (const page of context.pages()) await page.unrouteAll({ behavior: 'ignoreErrors' });
	}
	await browser.close();
}
