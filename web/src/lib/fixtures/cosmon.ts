/**
 * Cosmon — the reference company, per ARCHITECTURE.md §10.
 *
 * A small game studio building a 3D space MMORPG with creature discovery, capture,
 * training and battle mechanics. It is the reference precisely because it stresses
 * creative judgement, software delivery, art pipelines, multiplayer risk, long-running
 * work, spending, and public effects — the hard cases, not the easy ones.
 *
 * THIS IS FIXTURE DATA. Nothing here is read from or written to anything. It exists so
 * the surfaces can be looked at, walked through, and reviewed against the design
 * language before a single byte of wiring is written. Every field is shaped exactly as
 * the real view model, so replacing this module with a client is the whole migration.
 *
 * The org follows §10.1 and the work follows §10.2's first economic objective: the
 * smallest integrated browser game that proves the core product can exist.
 */

import type {
	AuthorityGrant,
	BudgetLine,
	ConnectionRow,
	DeskView,
	HqView,
	InboxUpdatesView,
	KanbanCard,
	LibraryRow,
	MissionView,
	NeedsYouItem,
	OrgNode,
	RecordDetail,
	ServesRow,
	StaffProfile,
	TapeEntry,
	TrailEntry,
	ThreadMessage,
	ThreadSummary
} from '$lib/model/view';
import type {
	OfferingLike,
	PartyLike,
	SourcingRequestLike,
	VendorWorkerLike
} from '$lib/model/market-view';

export const COMPANY_ID = 'cosmon';

const DAY = 24 * 60 * 60 * 1000;
const HOUR = 60 * 60 * 1000;
const MINUTE = 60 * 1000;

/** Fixture clock. Everything is relative so the surfaces never look stale. */
const now = Date.now();
const ago = (ms: number) => new Date(now - ms);
const ahead = (ms: number) => new Date(now + ms);

/* ============ the people ============ */

const VESPER = 'agent-vesper';
const JUNO = 'agent-juno';
const RUNE = 'agent-rune';
const MARLOW = 'agent-marlow';
const PELL = 'agent-pell';
const CASS = 'agent-cass';
const INDRA = 'agent-indra';
const BO = 'agent-bo';

const PEOPLE = [
	{ id: VESPER, name: 'Vesper', role: 'Exec · Studio Head', pig: 0 },
	{ id: JUNO, name: 'Juno Alder', role: 'Game Director', pig: 1 },
	{ id: RUNE, name: 'Rune Hallow', role: 'Technical Director', pig: 2 },
	{ id: MARLOW, name: 'Marlow Quill', role: 'Producer · OrgOps', pig: 3 },
	{ id: PELL, name: 'Pell Ashgrove', role: 'Gameplay design', pig: 4 },
	{ id: CASS, name: 'Cass Merrow', role: 'Gameplay engineer', pig: 0 },
	{ id: INDRA, name: 'Indra Vale', role: 'Technical artist', pig: 2 },
	{ id: BO, name: 'Bo Kestrel', role: 'Online feasibility', pig: 3 }
] as const;

const nameOf = (id: string) => PEOPLE.find((p) => p.id === id)?.name ?? 'unknown';
const pigOfPerson = (id: string) => PEOPLE.find((p) => p.id === id)?.pig ?? 0;

/* ============ the work board ============ */

function card(
	id: string,
	title: string,
	state: string,
	ownerId: string | null,
	costCents: number,
	stateReason: string | null = null,
	doneAt: Date | null = null
): KanbanCard {
	return {
		id,
		title,
		state,
		stateReason,
		ownerId,
		ownerName: ownerId ? nameOf(ownerId) : null,
		ownerPig: ownerId ? pigOfPerson(ownerId) : 0,
		costCents,
		doneAt
	};
}

const hq: HqView = {
	cashCents: 4_182_000,
	currency: 'USD',
	spendCents: 187_420,
	budgetCents: 400_000,
	runway: {
		months: 22,
		reason: null,
		assumption: 'Cash on hand divided by the trailing 30-day recorded spend.',
		disclaimer: 'Recorded spend only. Anything billed outside the company record is not counted.'
	},
	needsYou: 4,
	activeRuns: 3,
	columns: {
		queued: [
			card('w-battle', 'First battle: turn order, three moves, one win condition', 'queued', PELL, 0),
			card('w-audio', 'Encounter audio pass — capture chime, battle sting', 'queued', INDRA, 0),
			card('w-build', 'Browser-deployable build target, under 40 MB compressed', 'queued', CASS, 0)
		],
		inProgress: [
			card(
				'w-zone',
				'Explorable zone: the Verge, drift-lit and walkable end to end',
				'in_progress',
				INDRA,
				64_800,
				'Terrain and lighting land; traversal collision is being tuned.'
			),
			card(
				'w-capture',
				'Encounter → capture loop, tuned against three creatures',
				'in_progress',
				PELL,
				41_200,
				'Capture rates read as fair in the last playthrough; retuning the escape window.'
			),
			card(
				'w-netspike',
				'Multiplayer feasibility spike — bounded to two weeks',
				'in_progress',
				BO,
				29_900,
				'Authoritative-server prototype holds 24 clients; jitter above 120 ms is the open question.'
			)
		],
		needsReview: [
			card(
				'w-creatures',
				'Three original creatures — silhouette, ability, capture behaviour',
				'needs_review',
				JUNO,
				38_400,
				'Wants your read on whether the third one is distinct enough to keep.'
			),
			card(
				'w-assess',
				'Technical and product assessment for the next milestone',
				'needs_review',
				RUNE,
				12_100,
				'Recommends a vertical slice over a content push. Needs your call.'
			)
		],
		stuck: [
			card(
				'w-store',
				'Reserve the studio storefront name',
				'blocked',
				MARLOW,
				900,
				'Blocked: the registrar requires a verified identity document from a named director.'
			)
		],
		doneRecent: [
			card(
				'w-loop',
				'Exploration loop: movement, camera, and the drift trail',
				'done',
				CASS,
				51_600,
				null,
				ago(2 * DAY)
			),
			card('w-artbible', 'Art direction bible, first pass', 'done', INDRA, 22_300, null, ago(4 * DAY)),
			card('w-pitch', 'One-page product pitch for the first playable', 'done', JUNO, 8_700, null, ago(6 * DAY))
		]
	},
	doneOlder: 17,
	team: [
		{ id: VESPER, name: 'Vesper', role: 'Exec · Studio Head', status: 'active', pig: 0, live: true, working: true, openWork: 0, spendCents: 31_400, limitCents: 100_000 },
		{ id: JUNO, name: 'Juno Alder', role: 'Game Director', status: 'active', pig: 1, live: true, working: false, openWork: 1, spendCents: 47_100, limitCents: 60_000 },
		{ id: RUNE, name: 'Rune Hallow', role: 'Technical Director', status: 'active', pig: 2, live: true, working: false, openWork: 1, spendCents: 12_100, limitCents: 60_000 },
		{ id: MARLOW, name: 'Marlow Quill', role: 'Producer · OrgOps', status: 'active', pig: 3, live: false, working: false, openWork: 1, spendCents: 900, limitCents: 40_000 },
		{ id: PELL, name: 'Pell Ashgrove', role: 'Gameplay design', status: 'active', pig: 4, live: true, working: true, openWork: 2, spendCents: 41_200, limitCents: 50_000 },
		{ id: CASS, name: 'Cass Merrow', role: 'Gameplay engineer', status: 'active', pig: 0, live: true, working: true, openWork: 1, spendCents: 51_600, limitCents: 50_000 },
		{ id: INDRA, name: 'Indra Vale', role: 'Technical artist', status: 'active', pig: 2, live: true, working: true, openWork: 2, spendCents: 87_100, limitCents: 90_000 },
		{ id: BO, name: 'Bo Kestrel', role: 'Online feasibility', status: 'proposed', pig: 3, live: false, working: true, openWork: 1, spendCents: 29_900, limitCents: 30_000 }
	],
	drift: null
};

/* ============ what needs your word ============ */

const needsYou: NeedsYouItem[] = [
	{
		id: 'ny-creature',
		kind: 'decision',
		title: 'Keep or cut the third creature',
		detail:
			'Juno recommends cutting the Lumen Drifter and folding its capture behaviour into the Verge Moth. Two distinct creatures land better than three blurred ones.',
		createdAt: ago(3 * HOUR),
		ref: { decisionId: 'dec-creature' },
		context: {
			kind: 'decision',
			proposalFacts: [
				['recommendation', 'Cut the Lumen Drifter; keep two creatures for the first playable.'],
				['reason', 'Its silhouette reads as a variant of the Verge Moth at play distance.'],
				['work saved', 'Roughly nine days of art and tuning across Indra and Pell.'],
				['risk if kept', 'Three shallow creatures make the capture loop feel repetitive, not varied.'],
				['reversible', 'Yes — the model and ability are recorded and can be restored.']
			]
		}
	},
	{
		id: 'ny-playtest',
		kind: 'email-approval',
		title: 'Playtest invitation to the eleven people who asked',
		detail: 'Marlow drafted the first outside invitation. It goes to real inboxes, so it needs your signature.',
		createdAt: ago(9 * HOUR),
		ref: { approvalRequestId: 'apr-playtest', version: 3 },
		context: {
			kind: 'email-approval',
			draft: {
				fromName: 'Marlow Quill',
				fromEmail: 'studio@cosmon.example',
				toName: 'Playtest list',
				toEmail: 'eleven recipients',
				subject: 'Come break the first playable',
				textBody:
					'You asked to be told when there was something to play. There is — barely.\n\nOne zone, two creatures, one capture loop, one battle. It runs in a browser and it will take you about twelve minutes. We want to know where you got bored and where you got stuck, in that order.\n\nIf you are still in, reply and we will send you the link on Thursday.\n\n— Marlow, for the studio'
			}
		}
	},
	{
		id: 'ny-promote',
		kind: 'promotion-approval',
		title: 'Promote the capture-loop branch to the playable build',
		detail:
			'Cass finished the escape-window retune. The build is green and the run is recorded; promoting it is what makes it the thing playtesters see.',
		createdAt: ago(1 * DAY),
		ref: { approvalRequestId: 'apr-promote', version: 1 },
		context: { kind: 'promotion-approval', branch: 'feat/capture-escape-window', runId: 'run-8841' }
	},
	{
		id: 'ny-registrar',
		kind: 'escalation',
		title: 'The registrar wants a director’s identity document',
		detail: 'Everything up to the identity check is prepared and waiting. Only you can be the named director.',
		createdAt: ago(2 * DAY),
		ref: { escalationId: 'esc-registrar', version: 2 },
		context: {
			kind: 'escalation',
			escalationKind: 'human_identity_required',
			what: 'A government photo ID and a live selfie check, in the registrar’s own portal.',
			why: 'Company registration is a legal attestation by a named human director. It cannot be delegated.',
			expiresAt: ahead(5 * DAY),
			workTitle: 'Reserve the studio storefront name'
		}
	}
];

/* ============ what happened without you ============ */

const updates: InboxUpdatesView = {
	items: [
		{
			id: 'up-1',
			kind: 'run',
			title: 'Cass finished the escape-window retune',
			detail: 'Capture now fails honestly at low health instead of never. 41 minutes, $12.40.',
			at: ago(50 * MINUTE),
			unread: true,
			target: { kind: 'thread', thread: `agent:${CASS}` }
		},
		{
			id: 'up-2',
			kind: 'record',
			title: 'Art direction bible — version 4',
			detail: 'Indra recorded a new version: drift lighting rules and the creature silhouette grid.',
			at: ago(3 * HOUR),
			unread: true,
			target: { kind: 'record', assetId: 'asset-artbible' }
		},
		{
			id: 'up-3',
			kind: 'run',
			title: 'Bo held 24 clients on the authoritative-server prototype',
			detail: 'Jitter above 120 ms is still the open question. Full note on the thread.',
			at: ago(7 * HOUR),
			unread: true,
			target: { kind: 'thread', thread: `agent:${BO}` }
		},
		{
			id: 'up-4',
			kind: 'record',
			title: 'Technical and product assessment — version 2',
			detail: 'Rune recommends a vertical slice over a content push.',
			at: ago(1 * DAY),
			unread: false,
			target: { kind: 'record', assetId: 'asset-assess' }
		},
		{
			id: 'up-5',
			kind: 'run',
			title: 'The Verge passed its first end-to-end traversal',
			detail: 'Walkable corner to corner without falling through the world. 2h 11m, $38.90.',
			at: ago(2 * DAY),
			unread: false,
			target: { kind: 'thread', thread: `agent:${INDRA}` }
		}
	],
	total: 5,
	unreadCount: 3
};

/* ============ conversation ============ */

const threads: ThreadSummary[] = [
	{
		key: 'executive',
		kind: 'executive',
		conversationId: 'conv-exec',
		subjectId: null,
		title: 'Vesper',
		subtitle: 'Exec · Studio Head',
		pig: 0,
		live: true,
		preview: 'Two creatures, not three. I have written up why — it is waiting on your word.',
		lastAt: ago(35 * MINUTE),
		messageCount: 6
	},
	{
		key: `agent:${JUNO}`,
		kind: 'agent',
		conversationId: 'conv-juno',
		subjectId: JUNO,
		title: 'Juno Alder',
		subtitle: 'Game Director',
		pig: 1,
		live: true,
		preview: 'The Lumen Drifter reads as a Verge Moth variant at play distance. I would cut it.',
		lastAt: ago(3 * HOUR),
		messageCount: 4
	},
	{
		key: `agent:${CASS}`,
		kind: 'agent',
		conversationId: 'conv-cass',
		subjectId: CASS,
		title: 'Cass Merrow',
		subtitle: 'Gameplay engineer',
		pig: 0,
		live: true,
		preview: 'Escape window is retuned and the build is green. Ready to promote when you are.',
		lastAt: ago(50 * MINUTE),
		messageCount: 3
	},
	{
		key: `agent:${INDRA}`,
		kind: 'agent',
		conversationId: 'conv-indra',
		subjectId: INDRA,
		title: 'Indra Vale',
		subtitle: 'Technical artist',
		pig: 2,
		live: true,
		preview: 'Drift lighting is in the bible now, version 4. The Verge reads much better at dusk.',
		lastAt: ago(3 * HOUR),
		messageCount: 3
	},
	{
		key: `agent:${BO}`,
		kind: 'agent',
		conversationId: 'conv-bo',
		subjectId: BO,
		title: 'Bo Kestrel',
		subtitle: 'Online feasibility',
		pig: 3,
		live: false,
		preview: '24 clients held. Above 120 ms of jitter the capture loop stops feeling fair.',
		lastAt: ago(7 * HOUR),
		messageCount: 2
	},
	{
		key: `agent:${MARLOW}`,
		kind: 'agent',
		conversationId: 'conv-marlow',
		subjectId: MARLOW,
		title: 'Marlow Quill',
		subtitle: 'Producer · OrgOps',
		pig: 3,
		live: false,
		preview: 'The registrar will not take anything but a director in person. Escalated to you.',
		lastAt: ago(2 * DAY),
		messageCount: 3
	},
	{
		key: 'goal:goal-playable',
		kind: 'goal',
		conversationId: 'conv-playable',
		subjectId: 'goal-playable',
		title: 'First playable',
		subtitle: '5 of 8 done · 3 in flight',
		pig: 1,
		live: true,
		preview: 'Rune: the slice is the milestone. Content can wait for a foundation that holds.',
		lastAt: ago(5 * HOUR),
		messageCount: 9
	},
	{
		key: 'goal:goal-multiplayer',
		kind: 'goal',
		conversationId: 'conv-mp',
		subjectId: 'goal-multiplayer',
		title: 'Multiplayer feasibility',
		subtitle: '1 in flight · bounded to two weeks',
		pig: 3,
		live: false,
		preview: 'Bo: authoritative server is the right shape. Client prediction is the cost.',
		lastAt: ago(7 * HOUR),
		messageCount: 5
	}
];

function msg(
	id: string,
	from: ThreadMessage['from'],
	author: string,
	text: string,
	at: Date
): ThreadMessage {
	return {
		id,
		from,
		author,
		text,
		createdAt: at,
		replyToMessageId: null,
		assetId: null,
		runId: null,
		attachments: []
	};
}

const messages: Record<string, ThreadMessage[]> = {
	executive: [
		msg('m-e1', 'you', 'You', 'Where are we against the first playable?', ago(6 * HOUR)),
		msg(
			'm-e2',
			'agent',
			'Vesper',
			'Five of eight done, three in flight, one blocked on you.\n\n**Landed:** the exploration loop, the art bible, the pitch.\n**In flight:** the Verge, the capture loop, the multiplayer spike.\n**Blocked:** the storefront name — the registrar wants a director’s ID, which is yours to give.\n\nThe honest risk is not schedule, it is shape. Juno thinks we have three creatures where we need two good ones.',
			ago(6 * HOUR - 4 * MINUTE)
		),
		msg('m-e3', 'you', 'You', 'Say more about the creature problem.', ago(4 * HOUR)),
		msg(
			'm-e4',
			'agent',
			'Vesper',
			'At play distance the Lumen Drifter and the Verge Moth read as the same animal. A player learns one capture behaviour and applies it to both, so the third creature adds content without adding variety.\n\nCutting it returns about nine days across Indra and Pell — which is most of what the battle prototype needs.\n\nI have put it in front of you as a decision rather than making it: it is a taste call about the game, and that is yours.',
			ago(4 * HOUR - 3 * MINUTE)
		),
		msg('m-e5', 'you', 'You', 'What happens if I keep all three?', ago(40 * MINUTE)),
		msg(
			'm-e6',
			'agent',
			'Vesper',
			'The battle prototype slips past the playtest date, and the playtest is the point. We would be showing eleven people three creatures and no combat instead of two creatures and a fight.\n\nTwo creatures, not three. I have written up why — it is waiting on your word.',
			ago(35 * MINUTE)
		)
	],
	[`agent:${JUNO}`]: [
		msg('m-j1', 'you', 'You', 'Is the Drifter salvageable with a different silhouette?', ago(5 * HOUR)),
		msg(
			'm-j2',
			'agent',
			'Juno Alder',
			'Probably — but the redraw costs about the same as the cut saves, and we would be spending it to keep a creature nobody has asked for yet.\n\nMy read: cut it now, keep the ability. If the capture loop turns out to want a third behaviour after the playtest, we bring it back with a shape that earns its place.',
			ago(3 * HOUR)
		),
		msg('m-j3', 'you', 'You', 'Recorded. Put the recommendation in front of me formally.', ago(3 * HOUR - 20 * MINUTE)),
		msg(
			'm-j4',
			'agent',
			'Juno Alder',
			'Done — it is in your inbox with the numbers attached.',
			ago(3 * HOUR - 18 * MINUTE)
		)
	],
	[`agent:${CASS}`]: [
		msg('m-c1', 'you', 'You', 'How did the escape window land?', ago(2 * HOUR)),
		msg(
			'm-c2',
			'agent',
			'Cass Merrow',
			'Better. Capture now fails honestly at low health instead of never — the old curve made every attempt succeed if you were patient, which is not a loop, it is a wait.\n\nBuild is green. Forty-one minutes, twelve dollars forty.',
			ago(55 * MINUTE)
		),
		msg(
			'm-c3',
			'agent',
			'Cass Merrow',
			'Escape window is retuned and the build is green. Ready to promote when you are.',
			ago(50 * MINUTE)
		)
	],
	[`agent:${INDRA}`]: [
		msg('m-i1', 'you', 'You', 'Does the Verge read at dusk yet?', ago(5 * HOUR)),
		msg(
			'm-i2',
			'agent',
			'Indra Vale',
			'Much better. The drift trail carries the eye now instead of the terrain fighting it.\n\nI wrote the lighting rules into the bible so the next zone does not relitigate them.',
			ago(3 * HOUR - 5 * MINUTE)
		),
		msg(
			'm-i3',
			'agent',
			'Indra Vale',
			'Drift lighting is in the bible now, version 4. The Verge reads much better at dusk.',
			ago(3 * HOUR)
		)
	],
	[`agent:${BO}`]: [
		msg(
			'm-b1',
			'agent',
			'Bo Kestrel',
			'Authoritative server prototype holds 24 clients on one box.',
			ago(8 * HOUR)
		),
		msg(
			'm-b2',
			'agent',
			'Bo Kestrel',
			'24 clients held. Above 120 ms of jitter the capture loop stops feeling fair — the escape window lands differently on the client than on the server, and the player is right to be annoyed.\n\nThat is a client-prediction problem, not a capacity problem. Worth knowing before we promise multiplayer to anyone.',
			ago(7 * HOUR)
		)
	],
	[`agent:${MARLOW}`]: [
		msg('m-mq1', 'you', 'You', 'Where did the storefront name get to?', ago(3 * DAY)),
		msg(
			'm-mq2',
			'agent',
			'Marlow Quill',
			'Prepared up to the identity check: name reserved in the queue, forms filled, payment method attached.\n\nThe registrar will not take anything but a named director with a photo ID and a live check. I have kept the session open and escalated it to you rather than handing back a list of instructions.',
			ago(2 * DAY - 2 * HOUR)
		),
		msg(
			'm-mq3',
			'agent',
			'Marlow Quill',
			'The registrar will not take anything but a director in person. Escalated to you.',
			ago(2 * DAY)
		)
	],
	'goal:goal-playable': [
		msg(
			'm-p1',
			'agent',
			'Rune Hallow',
			'Recommendation for the next milestone: a vertical slice, not a content push.\n\nWe have one zone that works and a capture loop that is close. Adding creatures and zones on top of an unproven battle system means we find the problems late, in more places.',
			ago(6 * HOUR)
		),
		msg('m-p2', 'agent', 'Juno Alder', 'Agreed. Depth first, then breadth.', ago(5 * HOUR - 30 * MINUTE)),
		msg(
			'm-p3',
			'agent',
			'Rune Hallow',
			'The slice is the milestone. Content can wait for a foundation that holds.',
			ago(5 * HOUR)
		)
	],
	'goal:goal-multiplayer': [
		msg(
			'm-mp1',
			'agent',
			'Bo Kestrel',
			'Authoritative server is the right shape. Client prediction is the cost.',
			ago(7 * HOUR)
		)
	]
};

/* ============ the org ============ */

const org: OrgNode[] = [
	{
		id: VESPER,
		name: 'Vesper',
		role: 'Exec · Studio Head',
		status: 'active',
		pig: 0,
		draft: false,
		reports: [
			{ id: JUNO, name: 'Juno Alder', role: 'Game Director', status: 'active', pig: 1, draft: false, reports: [
				{ id: PELL, name: 'Pell Ashgrove', role: 'Gameplay design', status: 'active', pig: 4, draft: false, reports: [] }
			] },
			{ id: RUNE, name: 'Rune Hallow', role: 'Technical Director', status: 'active', pig: 2, draft: false, reports: [
				{ id: CASS, name: 'Cass Merrow', role: 'Gameplay engineer', status: 'active', pig: 0, draft: false, reports: [] },
				{ id: INDRA, name: 'Indra Vale', role: 'Technical artist', status: 'active', pig: 2, draft: false, reports: [] },
				{ id: BO, name: 'Bo Kestrel', role: 'Online feasibility', status: 'proposed', pig: 3, draft: false, reports: [] }
			] },
			{ id: MARLOW, name: 'Marlow Quill', role: 'Producer · OrgOps', status: 'active', pig: 3, draft: false, reports: [] }
		]
	}
];

/* ============ one employee, expanded ============ */

/** A plausible task trail for one employee — newest first, mixed operation kinds. */
function trailFor(id: string): TrailEntry[] {
	const kinds = [
		['work_run', 'run.started', 'ok'],
		['model_call', 'turn.completed', 'ok'],
		['command', 'work.progress.recorded', 'ok'],
		['tool_call', 'repo.read', 'ok'],
		['model_call', 'turn.completed', 'ok'],
		['command', 'asset.version.recorded', 'ok'],
		['work_run', 'run.finished', 'ok']
	] as const;
	return kinds.map((entry, index) => ({
		sequence: kinds.length - index,
		operationKind: entry[0],
		eventType: entry[1],
		status: entry[2],
		createdAt: ago((index + 1) * 37 * MINUTE)
	}));
}

function profile(
	id: string,
	over: Partial<StaffProfile> & Pick<StaffProfile, 'mandate' | 'reportsToName'>
): StaffProfile {
	const person = PEOPLE.find((p) => p.id === id)!;
	const member = hq.team.find((t) => t.id === id)!;
	return {
		id,
		name: person.name,
		role: person.role,
		kind: 'ai',
		instructions: null,
		status: member.status,
		live: member.live,
		pig: person.pig,
		spendCents: member.spendCents,
		limitCents: member.limitCents,
		currency: 'USD',
		can: [],
		needsWord: [],
		runningNow: [],
		planned: [],
		recentRuns: [],
		trackRecord: null,
		artifacts: [],
		goalsServed: [],
		stopped: false,
		operatingPolicy: { modelPolicy: 'default', memoryPolicy: 'durable', approvalEnvelope: 'standard', version: 1 },
		activeStop: null,
		trail: trailFor(id),
		trailHasMore: true,
		...over
	};
}

const staff: Record<string, StaffProfile> = {
	[VESPER]: profile(VESPER, {
		mandate:
			'Run the studio day to day. Turn the owner’s direction into work, keep the organisation coherent, and bring anything that needs judgement, identity, or money back to them with the case already made.',
		reportsToName: 'You',
		can: ['open and close work', 'assign staff', 'reorganise the team', 'read the whole record'],
		needsWord: ['send anything outside the company', 'spend above the monthly envelope', 'change the mission or standing authority'],
		runningNow: [{ id: 'run-9001', workTitle: 'Weekly direction review', status: 'running', startedAt: ago(12 * MINUTE) }],
		planned: [{ id: 'sch-1', title: 'Monday direction review', status: 'active', nextOccurrenceAt: ahead(2 * DAY), timezone: 'Australia/Sydney' }],
		recentRuns: [
			{ id: 'run-8990', workTitle: 'Milestone status roll-up', status: 'succeeded', resultSummary: 'Five of eight done; the creature question is the only real risk.', finishedAt: ago(6 * HOUR) },
			{ id: 'run-8977', workTitle: 'Escalation triage', status: 'succeeded', resultSummary: 'One registrar block; prepared to the identity check and handed up.', finishedAt: ago(2 * DAY) }
		],
		trackRecord: { runs: { total: 41, completed: 38, failed: 2, needsReview: 1 }, outputs: { accepted: 34, pendingReview: 1, reversals: 2 }, latency: { medianMs: 184_000 }, cost: { recordedCents: 31_400, metered: true }, escalationsRaised: 3 },
		goalsServed: [
			{ id: 'goal-playable', title: 'First playable', status: 'active' },
			{ id: 'goal-multiplayer', title: 'Multiplayer feasibility', status: 'active' }
		]
	}),
	[JUNO]: profile(JUNO, {
		mandate:
			'Own what the game is. Decide creature design, the shape of the capture loop, and what earns a place in the first playable — and say plainly when something does not.',
		reportsToName: 'Vesper',
		can: ['open work', 'record design decisions', 'read the whole record'],
		needsWord: ['cut or add a creature', 'change the first playable’s scope'],
		recentRuns: [
			{ id: 'run-8981', workTitle: 'Creature distinctness review', status: 'succeeded', resultSummary: 'Recommends cutting the Lumen Drifter; silhouette collision with the Verge Moth.', finishedAt: ago(3 * HOUR) }
		],
		trackRecord: { runs: { total: 24, completed: 22, failed: 1, needsReview: 1 }, outputs: { accepted: 19, pendingReview: 1, reversals: 3 }, latency: { medianMs: 412_000 }, cost: { recordedCents: 47_100, metered: true }, escalationsRaised: 0 },
		artifacts: [
			{ id: 'asset-pitch', title: 'One-page product pitch', assetType: 'document', status: 'recorded', latestVersion: 2 }
		],
		goalsServed: [{ id: 'goal-playable', title: 'First playable', status: 'active' }]
	}),
	[RUNE]: profile(RUNE, {
		mandate:
			'Own how the game is built. Keep the technical foundation honest, call the milestone shape, and refuse content that outruns the systems under it.',
		reportsToName: 'Vesper',
		can: ['open work', 'record assessments', 'run builds'],
		needsWord: ['promote a build to the playable', 'change the target platform'],
		recentRuns: [
			{ id: 'run-8975', workTitle: 'Milestone assessment', status: 'succeeded', resultSummary: 'Vertical slice over content push. Reasoning recorded.', finishedAt: ago(1 * DAY) }
		],
		trackRecord: { runs: { total: 26, completed: 25, failed: 0, needsReview: 1 }, outputs: { accepted: 22, pendingReview: 1, reversals: 1 }, latency: { medianMs: 296_000 }, cost: { recordedCents: 12_100, metered: true }, escalationsRaised: 1 },
		artifacts: [
			{ id: 'asset-assess', title: 'Technical and product assessment', assetType: 'document', status: 'recorded', latestVersion: 2 }
		],
		goalsServed: [{ id: 'goal-playable', title: 'First playable', status: 'active' }]
	}),
	[MARLOW]: profile(MARLOW, {
		mandate:
			'Keep the studio’s obligations moving: registration, scheduling, external correspondence, and the paperwork nobody else should be carrying.',
		reportsToName: 'Vesper',
		can: ['prepare filings and correspondence', 'schedule work', 'read the whole record'],
		needsWord: ['send anything outside the company', 'spend anything at all'],
		recentRuns: [
			{ id: 'run-8802', workTitle: 'Storefront name reservation', status: 'blocked', resultSummary: 'Prepared to the identity check. A named director must complete it.', finishedAt: ago(2 * DAY) }
		],
		trackRecord: { runs: { total: 15, completed: 12, failed: 1, needsReview: 0 }, outputs: { accepted: 11, pendingReview: 0, reversals: 0 }, latency: { medianMs: 74_000 }, cost: { recordedCents: 900, metered: true }, escalationsRaised: 4 },
		goalsServed: []
	}),
	[PELL]: profile(PELL, {
		mandate: 'Tune the encounter and capture loop until it is fair, legible, and worth repeating.',
		reportsToName: 'Juno Alder',
		can: ['open work', 'run tuning passes'],
		needsWord: ['change capture rates in the shipped build'],
		runningNow: [{ id: 'run-9003', workTitle: 'Escape window retune', status: 'running', startedAt: ago(28 * MINUTE) }],
		trackRecord: { runs: { total: 19, completed: 17, failed: 1, needsReview: 1 }, outputs: { accepted: 14, pendingReview: 1, reversals: 4 }, latency: { medianMs: 520_000 }, cost: { recordedCents: 41_200, metered: true }, escalationsRaised: 0 },
		goalsServed: [{ id: 'goal-playable', title: 'First playable', status: 'active' }]
	}),
	[CASS]: profile(CASS, {
		mandate: 'Build the gameplay systems: movement, camera, encounter, capture, and the build pipeline that ships them.',
		reportsToName: 'Rune Hallow',
		can: ['open work', 'run builds', 'push to feature branches'],
		needsWord: ['promote a build to the playable'],
		runningNow: [{ id: 'run-9002', workTitle: 'Browser build size pass', status: 'running', startedAt: ago(41 * MINUTE) }],
		recentRuns: [
			{ id: 'run-8841', workTitle: 'Escape window retune', status: 'succeeded', resultSummary: 'Capture fails honestly at low health. Build green.', finishedAt: ago(50 * MINUTE) }
		],
		trackRecord: { runs: { total: 33, completed: 30, failed: 2, needsReview: 1 }, outputs: { accepted: 27, pendingReview: 1, reversals: 2 }, latency: { medianMs: 631_000 }, cost: { recordedCents: 51_600, metered: true }, escalationsRaised: 1 },
		goalsServed: [{ id: 'goal-playable', title: 'First playable', status: 'active' }]
	}),
	[INDRA]: profile(INDRA, {
		mandate: 'Own the look and the art pipeline: zone lighting, creature silhouettes, and the rules that keep the next zone consistent with this one.',
		reportsToName: 'Rune Hallow',
		can: ['open work', 'record art direction', 'run asset builds'],
		needsWord: ['commission art outside the company'],
		runningNow: [{ id: 'run-9004', workTitle: 'Verge traversal collision', status: 'running', startedAt: ago(1 * HOUR) }],
		trackRecord: { runs: { total: 29, completed: 26, failed: 1, needsReview: 2 }, outputs: { accepted: 21, pendingReview: 2, reversals: 5 }, latency: { medianMs: 1_140_000 }, cost: { recordedCents: 87_100, metered: true }, escalationsRaised: 0 },
		artifacts: [
			{ id: 'asset-artbible', title: 'Art direction bible', assetType: 'document', status: 'recorded', latestVersion: 4 }
		],
		goalsServed: [{ id: 'goal-playable', title: 'First playable', status: 'active' }]
	}),
	[BO]: profile(BO, {
		mandate: 'Answer one question inside two weeks: can this game be multiplayer, and at what cost?',
		reportsToName: 'Rune Hallow',
		can: ['open work', 'run spikes'],
		needsWord: ['stand up anything publicly reachable', 'spend on hosting'],
		trackRecord: { runs: { total: 4, completed: 3, failed: 0, needsReview: 1 }, outputs: { accepted: 3, pendingReview: 1, reversals: 0 }, latency: { medianMs: 2_400_000 }, cost: { recordedCents: 0, metered: false }, escalationsRaised: 1 },
		goalsServed: [{ id: 'goal-multiplayer', title: 'Multiplayer feasibility', status: 'active' }]
	})
};

/* ============ the library ============ */

const library: LibraryRow[] = [
	{ id: 'asset-artbible', title: 'Art direction bible', assetType: 'document', status: 'recorded', byName: 'Indra Vale', byPig: 2, versionCount: 4, latestVersion: 4, updatedAt: ago(3 * HOUR) },
	{ id: 'asset-assess', title: 'Technical and product assessment', assetType: 'document', status: 'recorded', byName: 'Rune Hallow', byPig: 2, versionCount: 2, latestVersion: 2, updatedAt: ago(1 * DAY) },
	{ id: 'asset-creatures', title: 'Creature dossier — three candidates', assetType: 'document', status: 'in_review', byName: 'Juno Alder', byPig: 1, versionCount: 3, latestVersion: 3, updatedAt: ago(3 * HOUR) },
	{ id: 'asset-pitch', title: 'One-page product pitch', assetType: 'document', status: 'recorded', byName: 'Juno Alder', byPig: 1, versionCount: 2, latestVersion: 2, updatedAt: ago(6 * DAY) },
	{ id: 'asset-netspike', title: 'Multiplayer feasibility note', assetType: 'document', status: 'draft', byName: 'Bo Kestrel', byPig: 3, versionCount: 1, latestVersion: 1, updatedAt: ago(7 * HOUR) },
	{ id: 'asset-playtest', title: 'Playtest invitation draft', assetType: 'email', status: 'awaiting_approval', byName: 'Marlow Quill', byPig: 3, versionCount: 3, latestVersion: 3, updatedAt: ago(9 * HOUR) }
];

const records: Record<string, RecordDetail> = {
	'asset-artbible': {
		row: library[0],
		relatedWorkId: 'w-artbible',
		relatedWorkTitle: 'Art direction bible, first pass',
		versions: [
			{ id: 'v-ab-4', version: 4, runId: 'run-8912', producedBy: { actorId: INDRA, kind: 'ai', label: 'Indra Vale' }, contentDigest: 'sha256:b7f2c1a94e0d88b191ac3f70', mediaType: 'text/markdown', recordedAt: ago(3 * HOUR) },
			{ id: 'v-ab-3', version: 3, runId: 'run-8860', producedBy: { actorId: INDRA, kind: 'ai', label: 'Indra Vale' }, contentDigest: 'sha256:4e0d88b12d5b6e0477b9e015', mediaType: 'text/markdown', recordedAt: ago(2 * DAY) },
			{ id: 'v-ab-2', version: 2, runId: 'run-8804', producedBy: { actorId: INDRA, kind: 'ai', label: 'Indra Vale' }, contentDigest: 'sha256:91ac3f70cc10f4d2b7f2c1a9', mediaType: 'text/markdown', recordedAt: ago(4 * DAY) },
			/* Recorded outside a run — the owner wrote the opening statement themselves. There is
			 * no run to attribute it to, and inventing one would be a lie on the record. */
			{ id: 'v-ab-1', version: 1, runId: null, producedBy: null, contentDigest: 'sha256:2d5b6e0477b9e0154e0d88b1', mediaType: 'text/markdown', recordedAt: ago(6 * DAY) }
		],
		latestVersionId: 'v-ab-4',
		latestDigest: 'sha256:b7f2c1a94e0d88b191ac3f70',
		openComments: 1,
		content: {
			title: 'Art direction bible',
			body: 'THE DRIFT\n\nEverything in Cosmon is lit by drift: a slow, directional glow that comes from the field the zones sit inside, not from a sun. It has three rules.\n\n1. Drift reads as motion even when nothing moves. The gradient always runs somewhere.\n2. Nothing casts a hard shadow. If an edge needs definition, it gets a rim, not a shadow.\n3. The player is the brightest thing on screen, and the second brightest is whatever they can capture.\n\nSILHOUETTE\n\nA creature must be identifiable at play distance from its outline alone, with the fill removed. This is a hard gate, not a preference — we cut the Lumen Drifter on it.\n\nRun the silhouette grid before any texture work. Three creatures, black fill, 64px tall, side by side. If two of them read as the same animal, one of them is not a creature yet.'
		}
	},
	'asset-assess': {
		row: library[1],
		relatedWorkId: 'w-assess',
		relatedWorkTitle: 'Technical and product assessment for the next milestone',
		versions: [
			{ id: 'v-as-2', version: 2, runId: 'run-8975', producedBy: { actorId: RUNE, kind: 'ai', label: 'Rune Hallow' }, contentDigest: 'sha256:cc10f4d277b9e01591ac3f70', mediaType: 'text/markdown', recordedAt: ago(1 * DAY) },
			{ id: 'v-as-1', version: 1, runId: 'run-8931', producedBy: { actorId: RUNE, kind: 'ai', label: 'Rune Hallow' }, contentDigest: 'sha256:77b9e0152d5b6e04cc10f4d2', mediaType: 'text/markdown', recordedAt: ago(3 * DAY) }
		],
		latestVersionId: 'v-as-2',
		latestDigest: 'sha256:cc10f4d277b9e01591ac3f70',
		openComments: 0,
		content: {
			title: 'Technical and product assessment',
			body: 'RECOMMENDATION: a vertical slice, not a content push.\n\nWe have one zone that works and a capture loop that is close to working. The temptation is to add the second zone and the third creature, because that is what makes a build look like a game.\n\nIt is the wrong move. Adding content on top of an unproven battle system means we find the systemic problems late, in more places, with more art already committed to them. The cost of finding out that combat does not hold up goes up every week we spend widening instead of deepening.\n\nWHAT THE SLICE IS\n\nOne zone. Two creatures. One capture loop. One battle. Twelve minutes end to end, in a browser, under 40 MB.\n\nWHAT IT IS NOT\n\nIt is not a demo of the MMORPG. It is the smallest thing that can be wrong in an informative way.\n\nRISK\n\nThe multiplayer spike is the one thing that can invalidate the slice, because client prediction changes how the capture window has to be authored. Bo has two weeks. If the answer comes back expensive, we ship the slice single-player and say so.'
		}
	}
};

/* ============ the tape ============ */

const tape: TapeEntry[] = [
	{ id: 't-1', at: ago(35 * MINUTE), summary: 'Vesper recommended cutting the third creature and put the decision to the owner', you: false, actorLabel: 'Vesper', kind: 'event', category: 'work', policyOutcome: null, policyReason: null, eventCount: null, effectCount: null },
	{ id: 't-2', at: ago(50 * MINUTE), summary: 'Work run finished: escape window retune (41m, $12.40)', you: false, actorLabel: 'Cass Merrow', kind: 'event', category: 'work', policyOutcome: null, policyReason: null, eventCount: null, effectCount: null },
	{ id: 't-3', at: ago(3 * HOUR), summary: 'Asset version recorded: art direction bible v4', you: false, actorLabel: 'Indra Vale', kind: 'event', category: 'work', policyOutcome: null, policyReason: null, eventCount: null, effectCount: null },
	{ id: 't-4', at: ago(9 * HOUR), summary: 'Approval requested: send the playtest invitation to eleven recipients', you: false, actorLabel: 'Marlow Quill', kind: 'command', category: 'rules', policyOutcome: 'requires_approval', policyReason: 'External communication needs the owner’s signature.', eventCount: 2, effectCount: 0 },
	{ id: 't-5', at: ago(1 * DAY), summary: 'Spend recorded against the milestone envelope: $187.42 of $400.00', you: false, actorLabel: 'Vesper', kind: 'event', category: 'money', policyOutcome: null, policyReason: null, eventCount: null, effectCount: null },
	{ id: 't-6', at: ago(2 * DAY), summary: 'Escalated to the owner: registrar requires a director’s identity document', you: false, actorLabel: 'Marlow Quill', kind: 'command', category: 'rules', policyOutcome: 'escalated', policyReason: 'Irreducible human identity step.', eventCount: 3, effectCount: 0 },
	{ id: 't-7', at: ago(2 * DAY + 4 * HOUR), summary: 'You granted Bo Kestrel a $300/month envelope for the feasibility spike', you: true, actorLabel: 'You', kind: 'command', category: 'people', policyOutcome: 'allowed', policyReason: null, eventCount: 2, effectCount: 1 },
	{ id: 't-8', at: ago(3 * DAY), summary: 'Goal recorded: multiplayer feasibility, bounded to two weeks', you: true, actorLabel: 'You', kind: 'command', category: 'rules', policyOutcome: 'allowed', policyReason: null, eventCount: 1, effectCount: 0 },
	{ id: 't-9', at: ago(4 * DAY), summary: 'Work run finished: art direction bible first pass (3h 4m, $22.30)', you: false, actorLabel: 'Indra Vale', kind: 'event', category: 'work', policyOutcome: null, policyReason: null, eventCount: null, effectCount: null },
	{ id: 't-10', at: ago(6 * DAY), summary: 'Mission set: produce a working browser game proving the core loop', you: true, actorLabel: 'You', kind: 'command', category: 'rules', policyOutcome: 'allowed', policyReason: null, eventCount: 1, effectCount: 0 }
];

/* ============ the mission ============ */

const mission: MissionView = {
	mission:
		'Produce a working browser game with an end-to-end exploration–encounter–capture–battle loop and a credible foundation for later iteration.',
	set: 'set by you · 6 days ago · recorded',
	goals: [
		{ id: 'goal-playable', title: 'First playable', outcome: 'One explorable zone, creatures, a capture loop, and one battle — playable in a browser in twelve minutes.', status: 'active', targetDate: ahead(11 * DAY) },
		{ id: 'goal-multiplayer', title: 'Multiplayer feasibility', outcome: 'A defensible answer to whether this game can be multiplayer, and what it would cost.', status: 'active', targetDate: ahead(4 * DAY) },
		{ id: 'goal-playtest', title: 'First outside playtest', outcome: 'Eleven people play the build and tell us where they got bored and where they got stuck.', status: 'planned', targetDate: ahead(9 * DAY) },
		{ id: 'goal-register', title: 'Studio registered and named', outcome: 'The studio legally exists and owns its storefront name.', status: 'blocked', targetDate: ahead(5 * DAY) }
	],
	directives: [
		{ id: 'd-1', statement: 'Depth before breadth. Prove a system before building content on top of it.', status: 'active', createdAt: ago(6 * DAY) },
		{ id: 'd-2', statement: 'Nothing leaves the studio without my signature — no email, no post, no build.', status: 'active', createdAt: ago(6 * DAY) },
		{ id: 'd-3', statement: 'Every creature must be distinguishable at play distance by silhouette alone.', status: 'active', createdAt: ago(5 * DAY) },
		{ id: 'd-4', statement: 'Spikes are bounded when they start. Two weeks means two weeks.', status: 'active', createdAt: ago(3 * DAY) }
	],
	/* Derived from the same structured grants the authority board reads — one truth,
	 * phrased by whichever surface is doing the phrasing. */
	standingRules: []
};

const budgets: BudgetLine[] = [
	{ id: 'b-1', name: 'First playable milestone', status: 'active', amountCents: 400_000, currency: 'USD' },
	{ id: 'b-2', name: 'Multiplayer spike', status: 'active', amountCents: 30_000, currency: 'USD' },
	{ id: 'b-3', name: 'Registration and filings', status: 'active', amountCents: 5_000, currency: 'USD' }
];

/**
 * Standing authority as STRUCTURED grants, not rendered sentences.
 *
 * `mission.standingRules` says "Vesper may run builds (autonomous)" because that reads
 * well in a summary pane. This is the same truth in the shape the authority board reads:
 * a dotted capability it can group by domain, and a mode it can filter on — no prose to
 * reverse-engineer.
 */
const authority: AuthorityGrant[] = [
	{ id: 'g-1', actorId: VESPER, capability: 'work.open', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-2', actorId: VESPER, capability: 'work.close', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-3', actorId: VESPER, capability: 'people.assign', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-4', actorId: VESPER, capability: 'people.reorganise', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-5', actorId: VESPER, capability: 'billing.spend_within_envelope', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-6', actorId: VESPER, capability: 'billing.exceed_envelope', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-7', actorId: VESPER, capability: 'authority.grant', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-8', actorId: VESPER, capability: 'mission.revise', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-9', actorId: JUNO, capability: 'design.record_decision', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-10', actorId: JUNO, capability: 'design.change_scope', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-11', actorId: JUNO, capability: 'work.open', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-12', actorId: RUNE, capability: 'build.run', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-13', actorId: RUNE, capability: 'build.promote', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-14', actorId: RUNE, capability: 'work.open', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-15', actorId: CASS, capability: 'repo.push_feature_branch', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-16', actorId: CASS, capability: 'build.promote', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-17', actorId: CASS, capability: 'build.run', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-18', actorId: INDRA, capability: 'design.record_art_direction', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-19', actorId: INDRA, capability: 'market.commission_vendor', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-20', actorId: INDRA, capability: 'build.run_assets', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-21', actorId: MARLOW, capability: 'filing.prepare', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-22', actorId: MARLOW, capability: 'email.send_external', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-23', actorId: MARLOW, capability: 'schedule.create', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-24', actorId: MARLOW, capability: 'billing.spend_any', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-25', actorId: PELL, capability: 'work.open', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-26', actorId: PELL, capability: 'build.change_shipped_tuning', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-27', actorId: BO, capability: 'work.run_spike', mode: 'autonomous', version: 1, active: true, subject: 'staff' },
	{ id: 'g-28', actorId: BO, capability: 'network.expose_public_endpoint', mode: 'forbidden', version: 1, active: true, subject: 'staff' },
	{ id: 'g-29', actorId: BO, capability: 'billing.spend_hosting', mode: 'approval_required', version: 1, active: true, subject: 'staff' },
	{ id: 'g-30', actorId: 'you', capability: 'mission.revise', mode: 'autonomous', version: 1, active: true, subject: 'member' },
	{ id: 'g-31', actorId: 'you', capability: 'authority.grant', mode: 'autonomous', version: 1, active: true, subject: 'member' },
	{ id: 'g-32', actorId: 'you', capability: 'billing.set_budget', mode: 'autonomous', version: 1, active: true, subject: 'member' },
	{ id: 'g-33', actorId: 'you', capability: 'email.send_external', mode: 'autonomous', version: 1, active: true, subject: 'member' },
	{ id: 'g-34', actorId: 'you', capability: 'build.promote', mode: 'autonomous', version: 1, active: true, subject: 'member' }
];

/* The mission pane's chips and the authority board are the same grants, read once. */
mission.standingRules = authority.map((grant) => ({
	id: grant.id,
	holder: nameOf(grant.actorId) === 'unknown' ? 'You' : nameOf(grant.actorId),
	capability: grant.capability.split('.').slice(1).join(' ').replaceAll('_', ' ') || grant.capability,
	mode: grant.mode
}));

const serves: ServesRow[] = PEOPLE.map((person) => ({
	id: person.id,
	name: person.name,
	serves: staff[person.id]?.goalsServed.map((g) => g.title).join(' · ') || undefined,
	line: staff[person.id]?.mandate ?? ''
}));

/* ============ what is actually wired ============ */

/**
 * Every row here is what a LIVE probe returned, not what configuration claimed.
 * "never checked" is a real and distinct answer — it is not the same as "working",
 * and the surface must never round it up to one.
 */
const connections: ConnectionRow[] = [
	{ key: 'rt-vesper', name: 'Vesper', kind: 'runtime · acp', status: 'connected', ok: true, failed: false, when: 'checked 2 minutes ago' },
	{ key: 'rt-juno', name: 'Juno Alder', kind: 'runtime · acp', status: 'connected', ok: true, failed: false, when: 'checked 4 minutes ago' },
	{ key: 'rt-cass', name: 'Cass Merrow', kind: 'runtime · acp', status: 'connected', ok: true, failed: false, when: 'checked 4 minutes ago' },
	{ key: 'rt-bo', name: 'Bo Kestrel', kind: 'runtime · acp', status: 'unreachable', ok: false, failed: true, when: 'checked 6 hours ago' },
	{ key: 'cn-git', name: 'Studio repository', kind: 'connector · git', status: 'available', ok: true, failed: false, when: 'checked 11 minutes ago' },
	{ key: 'cn-mail', name: 'Studio mailbox', kind: 'connector · smtp', status: 'credentials expired', ok: false, failed: true, when: 'checked 1 hour ago' },
	{ key: 'cn-registrar', name: 'Registrar portal', kind: 'connector · browser', status: 'session held open', ok: true, failed: false, when: 'checked 2 days ago' },
	{ key: 'cn-hosting', name: 'Build hosting', kind: 'connector · http', status: 'unknown', ok: false, failed: false, when: 'never checked' }
];

/* ============ the marketplace ============ */

/**
 * Vendors offering accountability and credentials beyond what internal employees can:
 * a firm that carries professional indemnity, a collective that signs its reports.
 *
 * Nothing is invented. A vendor with no recorded price simply has no price shown, and a
 * party whose reconciliation has not been run reads as `unknown` rather than available.
 */
const parties: PartyLike[] = [
	{ id: 'mp-lex', name: 'Marris & Co Legal', status: 'sourced', roles: ['vendor'], serviceAreas: ['Company formation', 'IP assignment', 'Contracts'], jurisdictions: ['AU-NSW'], availabilityNote: 'Two-day turnaround on formation filings.', website: 'https://marris.example', email: 'hello@marris.example' },
	{ id: 'mp-audio', name: 'Halden Sound', status: 'sourced', roles: ['vendor'], serviceAreas: ['Original score', 'Interactive audio', 'Sound design'], jurisdictions: ['GB'], availabilityNote: 'Booked until the end of the month.', website: 'https://haldensound.example', email: 'brief@haldensound.example' },
	{ id: 'mp-qa', name: 'Fern QA Collective', status: 'not_sourced', roles: ['vendor'], serviceAreas: ['Structured playtests', 'Recorded sessions', 'Written reports'], jurisdictions: ['CA', 'US'], availabilityNote: null, website: 'https://fernqa.example', email: null },
	{ id: 'mp-net', name: 'Okonkwo Netcode', status: 'sourced', roles: ['vendor'], serviceAreas: ['Authoritative servers', 'Client prediction', 'Bounded spikes'], jurisdictions: ['NG', 'GB'], availabilityNote: 'Takes three-day minimum engagements.', website: null, email: 'ada@okonkwo.example' },
	{ id: 'mp-art', name: 'Sable Creature Works', status: 'inconsistent', roles: ['vendor'], serviceAreas: ['Concept art', 'Rigged models', 'Style matching'], jurisdictions: ['PL'], availabilityNote: 'Registry and our record disagree on their status — reconcile before engaging.', website: 'https://sable.example', email: null }
];

const offerings: OfferingLike[] = [
	{ id: 'of-1', name: 'Company formation, end to end', kind: 'service', description: 'Registration, constitution, and the first IP assignment, filed and confirmed.', priceCents: 180_000, currency: 'USD', billing: 'fixed', providerPartyId: 'mp-lex' },
	{ id: 'of-2', name: 'Interactive audio pass', kind: 'service', description: 'Capture chime, battle sting, and ambient drift bed. Stems delivered, not just a mix.', priceCents: 240_000, currency: 'USD', billing: 'fixed', providerPartyId: 'mp-audio' },
	{ id: 'of-3', name: 'Structured playtest, 12 participants', kind: 'service', description: 'Recorded sessions and a written report inside 48 hours.', priceCents: 96_000, currency: 'USD', billing: 'fixed', providerPartyId: 'mp-qa' },
	/* No recorded price. The surface shows nothing rather than a guess. */
	{ id: 'of-4', name: 'Netcode feasibility spike', kind: 'service', description: 'Three to five days on authoritative-server shape and client-prediction cost.', priceCents: null, currency: 'USD', billing: 'day rate', providerPartyId: 'mp-net' },
	{ id: 'of-5', name: 'Creature, concept to rig', kind: 'service', description: 'Silhouette brief in, three variants and one rigged model out.', priceCents: 320_000, currency: 'USD', billing: 'per creature', providerPartyId: 'mp-art' }
];

const vendorWorkers: VendorWorkerLike[] = [
	{ id: 'vw-1', name: 'Iris Marris', role: 'Principal', vendorPartyId: 'mp-lex' },
	{ id: 'vw-2', name: 'Tomas Halden', role: 'Composer', vendorPartyId: 'mp-audio' },
	{ id: 'vw-3', name: 'Ada Okonkwo', role: 'Netcode engineer', vendorPartyId: 'mp-net' },
	{ id: 'vw-4', name: 'Rill Fern', role: 'Test lead', vendorPartyId: 'mp-qa' }
];

const sourcingRequests: SourcingRequestLike[] = [
	{
		id: 'sr-audio',
		need: 'Interactive audio for the first playable — capture chime, battle sting, ambient drift bed',
		category: 'buy',
		status: 'open',
		budgetCapCents: 250_000,
		deadline: ahead(9 * DAY),
		requirements: ['Delivers stems, not just a mix', 'Works to a written brief', 'Available inside two weeks'],
		selectedPartyId: null,
		candidates: [
			{ partyId: 'mp-audio', partyName: 'Halden Sound', note: 'Quoted $2,400 fixed. Booked until the end of the month.' },
			{ partyId: 'mp-qa', partyName: 'Fern QA Collective', note: 'Out of discipline — listed themselves anyway.' }
		]
	},
	{
		id: 'sr-netcode',
		need: 'A second opinion on the multiplayer feasibility answer before we commit to it',
		category: 'hire',
		status: 'open',
		budgetCapCents: 150_000,
		deadline: ahead(5 * DAY),
		requirements: ['Has shipped an action game with authoritative servers', 'Takes a bounded three-day scope'],
		selectedPartyId: null,
		candidates: [
			{ partyId: 'mp-net', partyName: 'Okonkwo Netcode', note: 'Available from Monday. Three-day minimum.' }
		]
	},
	{
		id: 'sr-formation',
		need: 'Register the studio and assign the IP produced so far',
		category: 'buy',
		status: 'awarded',
		budgetCapCents: 200_000,
		deadline: ago(1 * DAY),
		requirements: ['Admitted in NSW', 'Carries professional indemnity'],
		selectedPartyId: 'mp-lex',
		candidates: [
			{ partyId: 'mp-lex', partyName: 'Marris & Co Legal', note: 'Selected. Blocked on the director identity check — that part is yours.' }
		]
	}
];

export const market = { parties, offerings, vendorWorkers, sourcingRequests };

/**
 * What this company has actually observed about each vendor. Reputation is composed from
 * these and nothing else — no imported rating, no star average. A vendor we have never
 * engaged reads as unproven, which is the truth.
 */
export const vendorEngagements = [
	{ partyId: 'mp-lex', stage: 'active' },
	{ partyId: 'mp-lex', stage: 'completed' },
	{ partyId: 'mp-audio', stage: 'proposed' },
	{ partyId: 'mp-net', stage: 'qualified' },
	{ partyId: 'mp-art', stage: 'lost' },
	{ partyId: 'mp-art', stage: 'cancelled' }
];

export const vendorCredentials = [
	{ vendorPartyId: 'mp-lex', status: 'valid' },
	{ vendorPartyId: 'mp-lex', status: 'valid' },
	{ vendorPartyId: 'mp-audio', status: 'valid' },
	{ vendorPartyId: 'mp-net', status: 'expiring_soon' },
	{ vendorPartyId: 'mp-art', status: 'expired' }
];

/* ============ the whole desk ============ */

export const cosmon: DeskView = {
	company: {
		id: COMPANY_ID,
		name: 'Cosmon',
		currency: 'USD',
		monthlyBudgetCents: 400_000,
		/* Recorded, not verified. The registration is still blocked on the director identity
		 * check, which is exactly why the legal name reads as pending rather than as fact. */
		legalName: null,
		tradingNames: ['Cosmon', 'Cosmon Studio'],
		jurisdictions: ['AU-NSW'],
		domains: ['cosmon.example'],
		ownership: 'Sole owner-operator',
		stage: 'forming',
		autonomyEnabled: true,
		providerDisclosureEnabled: true
	},
	needsYou,
	updates,
	threads,
	messages,
	hq,
	org,
	staff,
	library,
	records,
	tape,
	mission,
	budgets,
	serves,
	authority,
	connections,
	stops: [],
	/* Read from what is actually bound, not from a list of what the product supports. */
	boundProviders: ['codex-acp', 'anthropic'],
	/* Vesper answered a live ACP probe two minutes ago. This is the one field that must
	 * never be hardcoded true once there is a real runtime to ask. */
	executiveConnected: true,
	execName: 'Vesper',
	membershipRole: 'owner',
	providerDisclosureEnabled: true
};

/* ============ the money, as the spend page needs it ============ */

/**
 * Runs, work and goals in the raw shape `composeCostAttribution` reads.
 *
 * `driverProbe.kind` matters: `codex-acp` and `claude-acp` are subscription-billed,
 * so those runs record a cost of 0. That is measured-as-nothing, not free, and the
 * attribution pane is built to say so rather than draw a confident $0.00.
 */
export const spendInputs = {
	openCommitmentsCents: 96_000,
	goals: cosmon.mission.goals.map((goal) => ({ id: goal.id, title: goal.title })),
	work: [
		{ id: 'w-zone', goalId: 'goal-playable' },
		{ id: 'w-capture', goalId: 'goal-playable' },
		{ id: 'w-creatures', goalId: 'goal-playable' },
		{ id: 'w-loop', goalId: 'goal-playable' },
		{ id: 'w-artbible', goalId: 'goal-playable' },
		{ id: 'w-pitch', goalId: 'goal-playable' },
		{ id: 'w-assess', goalId: 'goal-playable' },
		{ id: 'w-battle', goalId: 'goal-playable' },
		{ id: 'w-audio', goalId: 'goal-playable' },
		{ id: 'w-build', goalId: 'goal-playable' },
		{ id: 'w-netspike', goalId: 'goal-multiplayer' },
		{ id: 'w-store', goalId: 'goal-register' }
	],
	runs: [
		{ workItemId: 'w-zone', costCents: 38_900, driverProbe: { kind: 'openai' } },
		{ workItemId: 'w-zone', costCents: 25_900, driverProbe: { kind: 'openai' } },
		{ workItemId: 'w-capture', costCents: 28_100, driverProbe: { kind: 'anthropic' } },
		{ workItemId: 'w-capture', costCents: 13_100, driverProbe: { kind: 'anthropic' } },
		{ workItemId: 'w-creatures', costCents: 38_400, driverProbe: { kind: 'anthropic' } },
		{ workItemId: 'w-loop', costCents: 51_600, driverProbe: { kind: 'openai' } },
		{ workItemId: 'w-artbible', costCents: 22_300, driverProbe: { kind: 'openai' } },
		{ workItemId: 'w-pitch', costCents: 8_700, driverProbe: { kind: 'anthropic' } },
		{ workItemId: 'w-assess', costCents: 12_100, driverProbe: { kind: 'anthropic' } },
		{ workItemId: 'w-netspike', costCents: 29_900, driverProbe: { kind: 'openai' } },
		{ workItemId: 'w-store', costCents: 900, driverProbe: { kind: 'openai' } }
	]
};

/** The door lists what you can open. One company today. */
export const companies = [{ id: COMPANY_ID, name: 'Cosmon', mission: cosmon.mission.mission }];

export const viewer = { name: 'You', role: 'owner' };
