/* The words Team results shows for the counts it reads: how much of a team's work landed, what
 * the owner told the company, and which goals are spread across many people. */
import type {
	OrgFrictionBaseline,
	OrgGoalWidth,
	OrgOwnerFriction,
	OrgSignals,
	OrgTeamResult
} from './org-review.ts';

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

const nbsp = (text: string) => text.replaceAll(' ', ' ');

/** "2 of 10 · 20%", or `none` when there was nothing to count against. */
export function shareValue(part: number, whole: number, none: string): string {
	if (whole <= 0) return none;
	const percent = Math.round((Math.min(part, whole) / whole) * 100);
	return `${nbsp(`${part} of ${whole}`)} · ${percent}%`;
}

/** The owner's friction over the last week as labelled rows, in one order. Each value is short
 * and keeps to one line, so a row never grows. */
export function frictionRows(
	friction: OrgOwnerFriction,
	baseline: OrgFrictionBaseline
): { label: string; value: string; info: string }[] {
	const perLanded =
		friction.work_landed > 0
			? `${(friction.owner_messages / friction.work_landed).toFixed(1)} per landed`
			: 'No Work landed';
	return [
		{
			label: 'Status checks',
			value: shareValue(friction.status_checks, friction.owner_messages, 'No messages'),
			info: `Messages asking where things are, out of everything you sent. With one agent this was about ${baseline.status_check_percent}%; above that the company reviews itself.`
		},
		{
			label: 'Corrected after a big ask',
			value: shareValue(friction.outcome_only_redirected, friction.outcome_only, 'No big asks'),
			info: `Asks that named an outcome but no method, corrected in your next message on the thread. Plain instructions: ${friction.instructions_redirected} of ${friction.instructions}.`
		},
		{
			label: 'Corrections in a row',
			value: shareValue(friction.redirect_runs, friction.redirects, 'No corrections'),
			info: `Corrections followed by another on the same thread. Baseline ${baseline.redirect_run_percent}%: ${baseline.from_last_review ? 'the rate at the last review' : 'how often any of your messages is a correction'}. Above it the company reviews itself.`
		},
		{
			label: 'Work sent back',
			value: friction.work_reopened ? nbsp(plural(friction.work_reopened, 'item')) : 'None',
			info: 'Work reopened after it was done. Any of it makes the company review itself.'
		},
		{
			label: 'Messages per landed Work',
			value: perLanded,
			info: `Your messages (${friction.owner_messages}) for each piece of Work that landed (${friction.work_landed}).`
		}
	];
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
