import assert from 'node:assert/strict';
import { createServer } from 'vite';

const server = await createServer({
	appType: 'custom',
	server: { middlewareMode: true }
});

try {
	const { render } = await server.ssrLoadModule('svelte/server');
	const { default: Markdown } = await server.ssrLoadModule('/src/lib/primitives/Markdown.svelte');
	const { body } = render(Markdown, {
		props: {
			text: '# Hosted result\n\n[Safe](https://example.com) <script>throw new Error("xss")</script>'
		}
	});

	assert.match(body, /Hosted result/);
	assert.match(body, /href="https:\/\/example\.com"/);
	assert.doesNotMatch(body, /<script>/);
	assert.doesNotMatch(body, /throw new Error/);
	console.log('Markdown renders and sanitizes during SSR');
} finally {
	await server.close();
}
