import assert from 'node:assert/strict';
import test from 'node:test';
import * as apps from './apps.ts';
import type { ToolConnection } from './connections.ts';
import type { SkillRow } from './skills.ts';

const { CATALOGUE, buildApps, classifyLink } = apps;

function connection(overrides: Partial<ToolConnection>): ToolConnection {
	return {
		name: 'fixture',
		kind: 'remote',
		endpoint: 'https://example.test/mcp',
		args: [],
		auth_type: 'oauth',
		status: 'working',
		frozen: false,
		tools: [],
		grants: [
			{ connection: 'fixture', grantee: '*', tools: [], granted_by: 'owner', granted_at: '' }
		],
		proposed: [],
		changed: [],
		...overrides
	};
}

const skill = (overrides: Partial<SkillRow>) =>
	({
		name: 'note-etiquette',
		description: 'How to write a note.',
		source: 'candidate',
		disposition: 'candidate',
		...overrides
	}) as SkillRow;

test("a plugin's service and know-how are one app, which needs the owner when any part does", () => {
	const url = 'https://github.com/acme/notes-plugin';
	const { mine } = buildApps(
		[connection({ name: 'notes-plugin-notes', source: `plugin:${url}@abc123` })],
		{
			skills: [skill({ origin_url: `${url}#skills/note-etiquette` })],
			assignments: [],
			usable: [],
			scan: { state: 'observed' }
		}
	);
	assert.equal(mine.length, 1);
	assert.equal(mine[0].plugin, url);
	assert.equal(mine[0].connections.length, 1);
	assert.equal(mine[0].skills.length, 1);
	assert.equal(mine[0].state, 'needs_you');
});

test('an app the company has, or Exec has asked for, is not offered again in Browse', () => {
	const { browse } = buildApps(
		[connection({ name: 'linear', endpoint: 'https://mcp.linear.app/mcp' })],
		null,
		['stripe']
	);
	const keys = browse.map((app) => app.key);
	assert.ok(!keys.includes('linear'));
	assert.ok(!keys.includes('stripe'));
	assert.ok(keys.includes('slack'));
});

test('owner-facing catalogue words never name the mechanism', () => {
	for (const entry of CATALOGUE) {
		for (const words of [entry.name, entry.description, entry.how]) {
			assert.doesNotMatch(words, /\b(MCP|CLI|skill|plugin|OAuth)\b/i, `${entry.key}: ${words}`);
		}
	}
});

test('a pasted link is read as a plugin, a service address or a command', () => {
	assert.equal(classifyLink('https://github.com/acme/notes-plugin'), 'plugin');
	assert.equal(classifyLink('https://mcp.example.com/mcp'), 'remote');
	assert.equal(classifyLink('npx -y @acme/notes-mcp'), 'command');
});
