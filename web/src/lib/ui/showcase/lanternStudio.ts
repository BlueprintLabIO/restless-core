/* Fixture data for the views in src/lib/ui. The example business is the same one the README
 * walks through: a small co-op lighthouse game. It is illustrative, never a claim about a real run,
 * and every public page that renders it must say so. */

import type { BoardColumn } from '../views/WorkBoard.svelte';

export const STUDIO = {
	name: 'Lantern Studio',
	brief: 'Make a small co-op lighthouse game, find an audience and prepare the first playtest.'
} as const;

export const STUDIO_BOARD: BoardColumn[] = [
	{
		key: 'proposed',
		label: 'Next',
		items: [
			{
				id: 'w-invite',
				title: 'Draft the playtest invitation',
				signal: 'Not started · 0/1 gates',
				status: 'proposed',
				ownerName: 'Marlow',
				revision: 1,
				href: '#'
			}
		]
	},
	{
		key: 'active',
		label: 'In motion',
		items: [
			{
				id: 'w-build',
				title: 'Revise the prototype after feedback',
				signal: 'Running · 2/4 gates · 1 output',
				status: 'active',
				ownerName: 'Ines',
				revision: 2,
				href: '#'
			},
			{
				id: 'w-audience',
				title: 'Map where new players hesitate',
				signal: 'Running · 1 output',
				status: 'active',
				ownerName: 'Theo',
				revision: 1,
				href: '#'
			}
		]
	},
	{
		key: 'blocked',
		label: 'Waiting',
		items: [
			{
				id: 'w-press',
				title: 'Publish the playable demo page',
				signal: 'Waiting for your approval · 3 outputs',
				status: 'blocked',
				ownerName: 'Marlow',
				revision: 1,
				href: '#'
			}
		]
	},
	{
		key: 'completed',
		label: 'Recently landed',
		items: [
			{
				id: 'w-brief',
				title: 'Shape the creative brief together',
				signal: 'Accepted · 2/2 gates · 1 output',
				status: 'completed',
				ownerName: 'Ines',
				revision: 1,
				href: '#'
			},
			{
				id: 'w-proto',
				title: 'Build the two-player prototype',
				signal: 'Accepted · 4/4 gates · 3 outputs',
				status: 'completed',
				ownerName: 'Ines',
				revision: 3,
				href: '#'
			}
		]
	}
];

export const STUDIO_FOLIO = {
	title: 'Approve the playable demo for the first playtest',
	whatHappened:
		'The two-player prototype is built, checked on three devices and packaged as a demo page.',
	whyItMatters:
		'Publishing is the first time outsiders see the game, so it is yours to approve.',
	uncertainty: 'Touch controls were only checked on one phone.',
	recommendation: 'Publish to the private playtest list first, then widen after one session.',
	evidence: [
		{ label: 'Build', result: 'Passed' },
		{ label: 'Two-player session', result: 'Played end to end' },
		{ label: 'Three-device check', result: '2 of 3 fully checked' }
	]
} as const;
