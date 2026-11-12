import assert from 'node:assert/strict';
import test from 'node:test';
import {
	earlierRate,
	goalLine,
	landedHeadline,
	landedPercent,
	reviewDueSentence,
	signalSentence,
	teamFacts,
	wideGoals
} from './org-review-text.ts';
import type { OrgTeamResult } from './org-review.ts';

const team = (fields: Partial<OrgTeamResult>): OrgTeamResult => ({
	team_id: 'team-1',
	team_name: 'Launch',
	lead_actor_id: 'aang',
	attempts: 0,
	landed: 0,
	failed: 0,
	blocked: 0,
	critic_attempts: 0,
	critic_catches: 0,
	minds: 0,
	work_items: 0,
	owner_sent_back: 0,
	...fields
});

test('a team reads as how much of its work landed, and one with no attempts says so', () => {
	assert.equal(landedHeadline(team({ attempts: 10, landed: 8 })), '8 of 10 landed');
	assert.equal(landedPercent(team({ attempts: 10, landed: 8 })), 80);
	assert.equal(landedPercent(team({ attempts: 3, landed: 1 })), 33);
	/* Never "0 of 0", and never a rate from a division by nothing. */
	assert.equal(landedHeadline(team({})), 'No work attempted');
	assert.equal(landedPercent(team({})), null);
});

test('the earlier rate is the same team at the last review, only when it had attempts then', () => {
	const earlier = [
		team({ team_id: 'team-1', attempts: 10, landed: 4 }),
		team({ team_id: null, attempts: 2, landed: 2 }),
		team({ team_id: 'team-2' })
	];
	assert.deepEqual(earlierRate(team({ team_id: 'team-1' }), earlier), {
		text: 'was 40%',
		detail: '4 of 10 landed'
	});
	/* The row for Exec and no team matches by its missing id, not by a name. */
	assert.equal(earlierRate(team({ team_id: null }), earlier)?.text, 'was 100%');
	assert.equal(earlierRate(team({ team_id: 'team-2' }), earlier), null);
	assert.equal(earlierRate(team({ team_id: 'team-3' }), earlier), null);
	assert.equal(earlierRate(team({ team_id: 'team-1' }), null), null);
});

test('every team shows the same labelled counts in the same order', () => {
	assert.deepEqual(
		teamFacts(
			team({
				failed: 1,
				blocked: 1,
				owner_sent_back: 2,
				minds: 4,
				critic_attempts: 5,
				critic_catches: 1
			})
		),
		['1 failed', '1 blocked', '2 sent back by you', '4 people', '5 checks, 1 sent work back']
	);
	/* Checks that changed nothing say so; a team nobody checked says that instead. */
	assert.equal(
		teamFacts(team({ minds: 1, critic_attempts: 1 })).at(-1),
		'1 check, none sent work back'
	);
	assert.deepEqual(teamFacts(team({ minds: 1 })), [
		'0 failed',
		'0 blocked',
		'0 sent back by you',
		'1 person',
		'No checks'
	]);
});

test('what the owner told the company is counted with its labels', () => {
	assert.equal(
		signalSentence({ sent_back: 2, lessons: 1, nudges: 3, accepted: 5, examples: [] }),
		'2\u00a0sent\u00a0back · 1\u00a0correction · 3\u00a0status\u00a0nudges · 5\u00a0accepted'
	);
	assert.equal(
		signalSentence({ sent_back: 0, lessons: 0, nudges: 0, accepted: 0, examples: [] }),
		'Nothing yet'
	);
	assert.equal(reviewDueSentence(true), 'A review is due on Exec’s next turn');
	assert.equal(reviewDueSentence(false), 'No review is due yet');
});

test('a goal is wide at five people or two reviewers', () => {
	const goal = (minds: number, critics: number) => ({
		goal_id: `${minds}-${critics}`,
		title: 'Ship the launch page',
		minds,
		critics,
		work_items: 9
	});
	assert.deepEqual(
		wideGoals([goal(5, 0), goal(4, 1), goal(1, 2), goal(4, 0)]).map((entry) => entry.goal_id),
		['5-0', '1-2']
	);
	assert.equal(goalLine(goal(6, 1)), '6 people · 1 reviewer · 9 pieces of work');
});
