import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';

const address = '127.0.0.1';
const port = Number(process.env.RESTLESS_READER_FIXTURE_PORT ?? 7787);
const upstream = new URL(process.env.RESTLESS_READER_UPSTREAM ?? 'http://127.0.0.1:7788');
const company = 'restless_cloud_quality_enforcer_test';
const attentionPath = `/api/companies/${company}/attention`;
const execConversationPath = `/api/companies/${company}/actors/exec/conversation`;
const fixture = await readFile(new URL('../fixtures/sprint28-attention.json', import.meta.url));
const conversationFixture = await readFile(
	new URL('../fixtures/sprint28-conversation.json', import.meta.url)
);

const server = createServer(async (request, response) => {
	try {
		const url = new URL(
			request.url ?? '/',
			`http://${request.headers.host ?? `${address}:${port}`}`
		);
		if (request.method === 'GET' && url.pathname === attentionPath) {
			response.writeHead(200, {
				'content-type': 'application/json',
				'cache-control': 'no-store'
			});
			response.end(fixture);
			return;
		}
		if (request.method === 'GET' && url.pathname === execConversationPath) {
			response.writeHead(200, {
				'content-type': 'application/json',
				'cache-control': 'no-store'
			});
			response.end(conversationFixture);
			return;
		}

		const target = new URL(`${url.pathname}${url.search}`, upstream);
		const body =
			request.method === 'GET' || request.method === 'HEAD'
				? undefined
				: Buffer.concat(await Array.fromAsync(request));
		const upstreamResponse = await fetch(target, {
			method: request.method,
			headers: Object.fromEntries(
				Object.entries(request.headers).filter(
					([name]) => !['host', 'content-length'].includes(name)
				)
			),
			body,
			redirect: 'manual'
		});
		response.writeHead(upstreamResponse.status, Object.fromEntries(upstreamResponse.headers));
		response.end(Buffer.from(await upstreamResponse.arrayBuffer()));
	} catch (error) {
		response.writeHead(502, { 'content-type': 'application/json' });
		response.end(JSON.stringify({ error: error instanceof Error ? error.message : String(error) }));
	}
});

server.listen(port, address, () => {
	process.stdout.write(`Sprint 28 reader fixture listening on http://${address}:${port}\n`);
});
