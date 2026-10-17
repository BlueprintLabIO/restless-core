/* Local HTTP/1 regression: live updates must not consume Chromium's six fetch slots. */
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { request } from 'node:http';
import { mkdir, writeFile } from 'node:fs/promises';
const origin = process.env.RESTLESS_SMOKE_ORIGIN,
	company = process.env.RESTLESS_SMOKE_COMPANY;
assert(origin && new URL(origin).hostname === '127.0.0.1' && company?.endsWith('_test'));
const out = process.env.RESTLESS_VERIFY_OUTPUT ?? '/tmp/restless-ta94';
await mkdir(out, { recursive: true });
const browser = await chromium.launch();
const context = await browser.newContext();
const pages = [];
const sockets = [];
const sse = [];
try {
	for (let i = 0; i < 8; i++) {
		const page = await context.newPage();
		pages.push(page);
		page.on('websocket', (socket) => sockets.push(socket.url()));
		page.on('request', (request) => {
			if (/\/changes(?:\?|$)|\/activity(?:\?|$)|\/events\/live(?:\?|$)/.test(request.url()))
				sse.push(request.url());
		});
		await page.goto(`${origin}/${company}/company/accounting`);
		await page
			.getByRole('heading', { name: 'Accounting', exact: true })
			.waitFor({ timeout: 20000 });
	}
	const page = pages.at(-1);
	const start = Date.now();
	await page.locator('a.related').click();
	await page.getByRole('heading', { name: 'Tokens', exact: true }).waitFor({ timeout: 10000 });
	const navigationMs = Date.now() - start;
	assert(sockets.filter((url) => url.endsWith('/changes')).length >= 8);
	assert.equal(sse.length, 0);
	const activity = await page.evaluate(
		async (path) =>
			new Promise((resolve, reject) => {
				const socket = new WebSocket(new URL(path, location.origin).href.replace(/^http:/, 'ws:'));
				socket.binaryType = 'arraybuffer';
				const timer = setTimeout(() => {
					socket.close();
					reject(new Error('No activity snapshot'));
				}, 5000);
				socket.onmessage = (event) => {
					const text = new TextDecoder().decode(event.data);
					clearTimeout(timer);
					socket.close();
					resolve(text);
				};
				socket.onerror = () => {
					clearTimeout(timer);
					reject(new Error('Activity socket failed'));
				};
			}),
		`/api/companies/${company}/actors/exec/activity`
	);
	assert.match(activity, /event: activity/);
	const direct = await (
		await fetch(`${origin}/api/companies/${company}/direct-conversations`)
	).json();
	const room = direct.find((row) => row.person_actor_id === 'exec');
	assert(room, 'Seed the direct Room');
	const roomFrame = await page.evaluate(
		async (path) =>
			new Promise((resolve, reject) => {
				const socket = new WebSocket(new URL(path, location.origin).href.replace(/^http:/, 'ws:'));
				socket.binaryType = 'arraybuffer';
				const timer = setTimeout(() => {
					socket.close();
					reject(new Error('No replayed Room event'));
				}, 5000);
				let received = '';
				socket.onmessage = (event) => {
					received += new TextDecoder().decode(event.data);
					if (!received.includes('event: room-event')) return;
					clearTimeout(timer);
					socket.close();
					resolve(received);
				};
				socket.onerror = () => {
					clearTimeout(timer);
					reject(new Error('Room socket failed'));
				};
			}),
		`/api/companies/${company}/rooms/${room.room_id}/events/live?after_event_id=0`
	);
	assert.match(roomFrame, /event: room-event/);

	const activityResponse = await fetch(`${origin}/api/companies/${company}/actors/exec/activity`, {
		signal: AbortSignal.timeout(5000)
	});
	assert.equal(activityResponse.status, 200);
	assert.match(activityResponse.headers.get('content-type'), /text\/event-stream/);
	await activityResponse.body.cancel();
	const refused = await new Promise((resolve, reject) => {
		const req = request(
			`${origin}/api/companies/${company}/changes`,
			{
				headers: {
					Origin: 'http://untrusted.example',
					Upgrade: 'websocket',
					Connection: 'Upgrade',
					'Sec-WebSocket-Key': 'dGhlIHNhbXBsZSBub25jZQ==',
					'Sec-WebSocket-Version': '13'
				}
			},
			(response) => {
				response.resume();
				resolve(response.statusCode);
			}
		);
		req.on('upgrade', (_response, socket) => {
			socket.destroy();
			resolve(101);
		});
		req.on('error', reject);
		req.end();
	});
	assert.equal(refused, 403, 'A socket cannot bypass the browser-origin boundary');
	await page.screenshot({ path: `${out}/multi-tab-tokens.png` });
	const result = {
		passed: true,
		tabs: pages.length,
		navigationMs,
		liveSockets: sockets.length,
		sseRequests: sse.length,
		legacySse: 200,
		crossOriginSocket: refused,
		roomReplay: true
	};
	await writeFile(`${out}/multi-tab-results.json`, JSON.stringify(result, null, 2));
	console.log(JSON.stringify(result));
} finally {
	await context.close();
	await browser.close();
}
