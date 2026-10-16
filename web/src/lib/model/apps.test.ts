import assert from 'node:assert/strict';
import test from 'node:test';
import * as apps from './apps.ts';
import type { ToolConnection } from './connections.ts';
import type { SkillRow } from './skills.ts';

const { CATALOGUE, buildApps, classifyLink, connectTag } = apps;
import { registryKey, catalogueConnectionName } from './app-catalogue.ts';

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

test('an app turned off keeps its sign-in and reads as off, and removed know-how is kept to restore', () => {
	const ended = {
		connection: 'fixture',
		grantee: '*',
		tools: [],
		granted_by: 'owner',
		granted_at: ''
	};
	const { mine, removed } = buildApps(
		[
			connection({ name: 'off', grants: [], ended_grant: ended }),
			connection({ name: 'new', grants: [] })
		],
		{
			skills: [skill({ name: 'old-way', disposition: 'retired', source: 'company' })],
			assignments: [],
			usable: [],
			scan: { state: 'observed' }
		}
	);
	const state = (name: string) => mine.find((app) => app.key === `c-${name}`);
	assert.equal(state('off')?.state, 'paused');
	assert.equal(state('off')?.attention, 'Off');
	assert.equal(state('new')?.state, 'needs_you');
	assert.deepEqual(
		removed.map((app) => app.key),
		['s-old-way']
	);
	assert.ok(!mine.some((app) => app.key === 's-old-way'));
});

test('the public directory adds services and know-how the company lacks, real names first', () => {
	const registry = [
		{
			key: 'mcp:io.zzz/a',
			kind: 'mcp' as const,
			name: '$AI tokens',
			summary: '',
			source: 'MCP Registry',
			homepage: null,
			endpoint: 'https://ai.example/mcp',
			files: [],
			runs_scripts: false
		},
		{
			key: 'mcp:io.cal/b',
			kind: 'mcp' as const,
			name: 'Calendar Hub',
			summary: 'Events',
			source: 'MCP Registry',
			homepage: null,
			endpoint: 'https://cal.example/mcp',
			files: [],
			runs_scripts: false
		},
		{
			key: 'mcp:dup',
			kind: 'mcp' as const,
			name: 'Slack again',
			summary: '',
			source: 'MCP Registry',
			homepage: null,
			endpoint: 'https://mcp.slack.com/mcp',
			files: [],
			runs_scripts: false
		},
		{
			key: 'skill:anthropics/pdf',
			kind: 'skill' as const,
			name: 'pdf',
			summary: 'PDF tools',
			source: 'Anthropic skills',
			homepage: null,
			endpoint: null,
			files: ['SKILL.md', 'scripts/fill.py'],
			runs_scripts: true
		}
	];
	const { browse } = buildApps([], null, [], registry);
	const more = browse.filter((app) => app.category === 'More').map((app) => app.name);
	assert.deepEqual(more, ['$AI tokens', 'Calendar Hub']);
	assert.ok(
		!browse.some((app) => app.name === 'Slack again'),
		'a registry copy of a catalogued address is left out'
	);
	const pdf = browse.find((app) => app.key === 'x-pdf');
	assert.equal(pdf?.registrySkill?.runsScripts, true);
	assert.equal(connectTag(pdf!), 'Has scripts');
	assert.equal(browse.find((app) => app.name === 'Calendar Hub')?.key, registryKey('mcp:io.cal/b'));
});

test('same-title and non-Latin registry entries keep distinct identities and legacy links cannot choose the wrong publisher', async () => {
	const base = {
		kind: 'mcp' as const,
		summary: 'Company service',
		source: 'MCP Registry',
		homepage: null,
		files: [],
		runs_scripts: false
	};
	const registry = [
		{ ...base, key: 'mcp:com.one/mcp', name: 'mcp', endpoint: 'https://one.example/mcp' },
		{ ...base, key: 'mcp:com.two/mcp', name: 'mcp', endpoint: 'https://two.example/mcp' },
		{ ...base, key: 'mcp:cn.example/tool', name: '工具', endpoint: 'https://cn.example/mcp' },
		{ ...base, key: 'mcp:kr.example/tool', name: '도구', endpoint: 'https://kr.example/mcp' }
	];
	const projection = buildApps([], null, [], registry);
	const found = projection.browse.filter((app) => app.catalogue?.registryId);
	assert.equal(found.length, 4);
	assert.equal(new Set(found.map((app) => app.key)).size, 4);
	for (const item of registry) {
		const resolved = apps.resolveApp(projection, registryKey(item.key));
		assert.equal(resolved.app?.catalogue?.endpoint, item.endpoint);
		const entry = resolved.app!.catalogue!;
		const name = await catalogueConnectionName(entry);
		assert.match(name, /^[a-z][a-z0-9-]{0,39}$/);
		assert.equal(await catalogueConnectionName(entry), name);
	}
	assert.equal(apps.resolveApp(projection, 'r-mcp').ambiguous, true);
	assert.equal(apps.resolveApp(projection, 'r-mcp').app, undefined);
	assert.equal(apps.resolveApp(projection, 'r-service').ambiguous, true);
	const reordered = buildApps([], null, [], registry.toReversed());
	assert.equal(
		apps.resolveApp(reordered, registryKey(registry[0].key)).app?.catalogue?.endpoint,
		registry[0].endpoint
	);
	const renamed = buildApps([], null, [], [{ ...registry[0], name: 'A better display title' }]);
	assert.equal(
		apps.resolveApp(renamed, registryKey(registry[0].key)).app?.catalogue?.endpoint,
		registry[0].endpoint
	);
});
