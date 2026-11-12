/* The words Team results shows for the counts it reads: how much of a team's work landed, what
 * the owner told the company, and which goals are spread across many people. */
import type { OrgGoalWidth, OrgSignals, OrgTeamResult } from './org-review.ts';

/** The row for Exec and anyone on no team has no name of its own. */
export const NO_TEAM_NAME = 'Exec and people on no team';

const plural = (count: number, one: string, many = `${one}s`) =>
	`${count} ${count === 1 ? one : many}`;

/** The share of a team's attempts that landed, in whole percent; null when it made none. */
export function landedPercent(team: Pick<OrgTeamResult, 'attempts' | 'landed'>): number | null {
	if (team.attempts <= 0) return null;
	return Math.round((Math.min(team.landed, team.attempts) / team.attempts) * 100);
}

/** "8 of 10 landed", or that the team attempted nothing rather than "0 of 0". */
export function landedHeadline(team: Pick<OrgTeamResult, 'attempts' | 'landed'>): string {
	if (team.attempts <= 0) return 'No work attempted';
	return `${team.landed} of ${team.attempts} landed`;
}

/** The same team at the last review, when it had attempts then: "was 40%". */
export function earlierRate(
	team: Pick<OrgTeamResult, 'team_id'>,
	earlier: OrgTeamResult[] | null | undefined
): { text: string; detail: string } | null {
	const before = earlier?.find((candidate) => candidate.team_id === team.team_id);
	const percent = before ? landedPercent(before) : null;
	if (!before || percent === null) return null;
	return { text: `was ${percent}%`, detail: landedHeadline(before) };
}

/** A team's other counts, each with its label, in one order for every team. */
export function teamFacts(team: OrgTeamResult): string[] {
	const checks = team.critic_attempts
		? `${plural(team.critic_attempts, 'check')}, ${team.critic_catches || 'none'} sent work back`
		: 'No checks';
	return [
		`${team.failed} failed`,
		`${team.blocked} blocked`,
		`${team.owner_sent_back} sent back by you`,
		plural(team.minds, 'person', 'people'),
		checks
	];
}

/** What the owner told the company, as labelled counts. Each count keeps to one line with its
 * label, so a narrow page breaks between counts and never inside one. */
export function signalSentence(signals: OrgSignals): string {
	if (!signals.sent_back && !signals.lessons && !signals.nudges && !signals.accepted)
		return 'Nothing yet';
	return [
		`${signals.sent_back} sent back`,
		plural(signals.lessons, 'correction'),
		plural(signals.nudges, 'status nudge'),
		`${signals.accepted} accepted`
	]
		.map((part) => part.replaceAll(' ', '\u00a0'))
		.join(' · ');
}

export function reviewDueSentence(due: boolean): string {
	return due ? 'A review is due on Exec’s next turn' : 'No review is due yet';
}

/** Goals spread across many people: five or more working on one, or two or more checking it. */
export function wideGoals(goals: OrgGoalWidth[]): OrgGoalWidth[] {
	return goals.filter((goal) => goal.minds >= 5 || goal.critics >= 2);
}

export function goalLine(goal: OrgGoalWidth): string {
	return [
		plural(goal.minds, 'person', 'people'),
		plural(goal.critics, 'reviewer'),
		plural(goal.work_items, 'piece of work', 'pieces of work')
	].join(' · ');
}
