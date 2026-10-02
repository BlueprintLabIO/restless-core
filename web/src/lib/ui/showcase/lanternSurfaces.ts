/* Fixtures for the product-surface views, from the same illustrative business as lanternStudio.ts:
 * Lantern Studio, making "Last Light", a co-op lighthouse game. People (NI) bring the taste; agents
 * (AI) carry the chores. Never a claim about a real run; every public page that renders it says so. */

import type { DocBlock, DocCollaborator, DocComment, DocSummary } from '../views/DocumentView.svelte';
import type { RoomMessage, RoomParticipant, RoomSummary } from '../views/RoomView.svelte';
import type { AuthorityRule } from '../views/AuthorityLimits.svelte';
import type { AttentionEntry } from '../views/AttentionInbox.svelte';
import type { PeopleTeam, PersonDetail } from '../views/PeopleView.svelte';
import type { IntelligenceAssignment, IntelligenceConnection } from '../views/AgentIntelligence.svelte';
import type { IdentityOutput, IdentityPillar } from '../views/IdentityView.svelte';

export const STUDIO_COMPANY = 'Lantern Studio';

export const STUDIO_COLLABORATORS: DocCollaborator[] = [
	{ id: 'you', name: 'You', kind: 'person', color: '#456687' },
	{ id: 'rosa', name: 'Rosa', kind: 'person', color: '#b5733a' },
	{ id: 'marlow', name: 'Marlow', kind: 'agent', color: '#6a588e' }
];

export const STUDIO_DOCUMENTS: DocSummary[] = [
	{ id: 'brief', title: 'Last Light — playtest brief', meta: 'Brief · Draft · today' },
	{ id: 'review', title: 'Playtest feedback → next build', meta: 'Review · Draft · yesterday' },
	{ id: 'audience', title: 'Audience & first five playtests', meta: 'Plan · Draft · Monday' }
];

export const STUDIO_BRIEF: DocBlock[] = [
	{ kind: 'h1', text: 'Last Light' },
	{ kind: 'p', text: 'Two keepers. One lighthouse to bring back to life.' },
	{ kind: 'h2', text: 'What players should feel' },
	{ kind: 'p', text: 'The first evening is warm and slightly lonely, never frantic.', mark: { phrase: 'warm and slightly lonely', by: 'rosa' } },
	{ kind: 'h2', text: 'Scope for Friday' },
	{ kind: 'li', text: 'Three puzzles, each needing both keepers.' },
	{ kind: 'li', text: 'The boat level waits for the next build.' },
	{ kind: 'h2', text: 'Who does what' },
	{ kind: 'li', text: 'Rosa repaints the lamp; Ines exports it at three sizes.' },
	{ kind: 'li', text: 'Jun retunes puzzle three; Ines replays it two hundred times.' }
];

export const STUDIO_BRIEF_COMMENTS: DocComment[] = [
	{ id: 'c1', author: 'Rosa', kind: 'person', time: '10:31', anchor: 'warm and slightly lonely', text: 'The lamp is the hero. Everything else gets quieter.' },
	{ id: 'c2', author: 'Marlow', kind: 'agent', time: '10:33', text: 'Briefed Ines on the export. Lamp v3 is on the board.' }
];

export const STUDIO_ROOMS: RoomSummary[] = [
	{ id: 'week', name: 'Studio · this week', kind: 'group' },
	{ id: 'art', name: 'Art', kind: 'group', unread: 2 },
	{ id: 'jun-ines', name: 'Jun / Ines', kind: 'direct' }
];

export const STUDIO_ROOM_PEOPLE: RoomParticipant[] = [
	{ name: 'You', kind: 'person' },
	{ name: 'Rosa', kind: 'person' },
	{ name: 'Exec', kind: 'agent' },
	{ name: 'Marlow', kind: 'agent' }
];

export const STUDIO_ROOM_MESSAGES: RoomMessage[] = [
	{ id: 'm1', author: 'You', kind: 'person', time: '09:10', text: 'This week: playtest-ready by Friday. Keep it small.' },
	{ id: 'm2', author: 'Rosa', kind: 'person', time: '09:11', text: 'Then the lamp has to feel warm, not neon. I’ll repaint it today.' },
	{ id: 'm3', author: 'Exec', kind: 'agent', time: '09:12', text: 'Agreed. Marlow owns the playtest. Rosa’s art and Jun’s puzzles stay off the chore list.', replyTo: 'm1' }
];

export const STUDIO_AUTHORITY = {
	standard: 'Exceptional',
	may: [
		{ title: 'Internal company work', body: 'Plan, research, edit company files, build and coordinate inside the company computer.' },
		{ title: 'Model use inside the spend ceiling', body: 'Configured models may be used while metered spend stays under the owner’s ceiling.' }
	] satisfies AuthorityRule[],
	asks: [
		{ title: 'First consequential contact', body: 'A new external party is prepared and brought to Attention before the first real effect.' },
		{ title: 'Publishing', body: 'Anything outsiders see first, like the playable demo, waits for a person.' }
	] satisfies AuthorityRule[],
	cannot: [
		{ title: 'Expand its own mandate', body: 'The company cannot grant itself new authority or approve its own expansion.' },
		{ title: 'Reach raw secrets', body: 'Vault values and owner credentials stay outside the company computer.' }
	] satisfies AuthorityRule[],
	spend: { accounted: 3.2, ceiling: 10 }
};

export const STUDIO_ATTENTION: AttentionEntry[] = [
	{ id: 'a1', title: 'Approve the playable demo for the first playtest', category: 'Decision', from: 'Marlow', fromKind: 'agent', age: '17:30', state: 'needs-you' },
	{ id: 'a2', title: 'Review the store page copy', category: 'Review', from: 'Camille', fromKind: 'agent', age: 'preparing', state: 'preparing' },
	{ id: 'a3', title: 'Pick the playtest date', category: 'Input', from: 'Kit', fromKind: 'person', age: '11:02', state: 'done' }
];

export const STUDIO_TEAMS: PeopleTeam[] = [
	{
		name: 'Studio',
		inMotion: 4,
		blocked: 1,
		members: [
			{ id: 'marlow', name: 'Marlow', role: 'Production lead', kind: 'agent', lead: true },
			{ id: 'you', name: 'You', role: 'Creative director', kind: 'person' },
			{ id: 'rosa', name: 'Rosa', role: 'Artist', kind: 'person' },
			{ id: 'jun', name: 'Jun', role: 'Puzzle designer', kind: 'person' },
			{ id: 'ines', name: 'Ines', role: 'Build and QA', kind: 'agent' }
		]
	},
	{
		name: 'Playtests',
		inMotion: 2,
		blocked: 0,
		members: [
			{ id: 'kit', name: 'Kit', role: 'Community and playtests', kind: 'person', lead: true },
			{ id: 'camille', name: 'Camille', role: 'Operations', kind: 'agent' }
		]
	}
];

export const STUDIO_PERSON: PersonDetail = {
	id: 'ines',
	name: 'Ines',
	role: 'Build and QA · Studio',
	kind: 'agent',
	standard: 'Exceptional',
	state: 'in motion',
	accountable: 'Marlow',
	work: [
		{ title: 'Export the lamp at three sizes', revision: 3, status: 'completed' },
		{ title: 'Replay puzzle three after the ladder change', revision: 1, status: 'active' },
		{ title: 'Package the demo for three devices', revision: 1, status: 'proposed' }
	]
};

export const STUDIO_CONNECTIONS: IntelligenceConnection[] = [
	{ name: 'ChatGPT / Codex', state: 'signed-in' },
	{ name: 'Claude Code', state: 'signed-in' },
	{ name: 'OpenRouter', state: 'api-key' }
];

export const STUDIO_INTELLIGENCE: IntelligenceAssignment[] = [
	{ id: 'marlow', name: 'Marlow', role: 'lead', connection: null, model: 'company default' },
	{ id: 'ines', name: 'Ines', role: 'build and QA', connection: 'Claude Code', model: 'claude' },
	{ id: 'theo', name: 'Theo', role: 'research', connection: 'OpenRouter', model: 'gemini' },
	{ id: 'camille', name: 'Camille', role: 'operations', connection: null, model: 'company default' }
];

export const STUDIO_IDENTITY: IdentityPillar[] = [
	{ key: 'truth', label: 'Truth', entries: ['Last Light is a two-player co-op game.', 'The demo is free to play.'], correction: { from: 'Launch price $12', to: 'Launch price $9' } },
	{ key: 'voice', label: 'Voice', entries: ['Warm and plain. Never hype.', 'Talk about the players, not the studio.'] },
	{ key: 'visual', label: 'Visual language', entries: ['Warm light, rough seas.', 'Hopeful, never horror.'] },
	{ key: 'culture', label: 'Culture', entries: ['Playtest before we polish.', 'Disagree in the brief, not in Discord.'] }
];

export const STUDIO_IDENTITY_OUTPUTS: IdentityOutput[] = [
	{ title: 'Store page', state: 'outdated' },
	{ title: 'Press kit', state: 'outdated' },
	{ title: 'Playtest invitation', state: 'current' }
];
