/* How each team's work is landing, and the company's own reviews of how it is organised. The
 * owner plane counts these; this page only reads them. */
import { ownerJson } from './failure.ts';

/** One team's production work over the scorecard's window. `team_id` is null for Exec and
 * anyone on no team. */
export interface OrgTeamResult {
	team_id: string | null;
	team_name: string;
	lead_actor_id: string | null;
	/** Production attempts, and how they ended. */
	attempts: number;
	landed: number;
	failed: number;
	blocked: number;
	/** Checks by a separate reviewer, and how many sent work back. */
	critic_attempts: number;
	critic_catches: number;
	/** People who worked on it. */
	minds: number;
	work_items: number;
	owner_sent_back: number;
}

export interface OrgGoalWidth {
	goal_id: string;
	title: string;
	minds: number;
	critics: number;
	work_items: number;
}

/** What the owner told the company since its last review. */
export interface OrgSignals {
	sent_back: number;
	accepted: number;
	nudges: number;
	lessons: number;
	examples: string[];
}

/** How much the owner had to pull, correct and resend over the last week. Every owner message
 * is classified by fixed rules; counts are of owner messages unless named otherwise. A run is a
 * correction followed by another on the same thread. `new_*` counts only what came after the
 * last review. */
export interface OrgOwnerFriction {
	owner_messages: number;
	status_checks: number;
	/** Asks that state an outcome with no method, and how many were corrected next. */
	outcome_only: number;
	outcome_only_redirected: number;
	instructions: number;
	instructions_redirected: number;
	redirects: number;
	redirect_runs: number;
	/** Work sent back after it was done. */
	work_reopened: number;
	work_landed: number;
	new_status_checks: number;
	new_redirect_runs: number;
	new_work_reopened: number;
}

export interface OrgFrictionBaseline {
	status_check_percent: number;
	redirect_run_percent: number;
	/** The run baseline is the last review's rate; otherwise the owner's own correction rate. */
	from_last_review: boolean;
}

export interface OrgScorecard {
	since: string;
	signals_since: string;
	signals_through: string;
	/** Enough was said for the company to review itself on Exec's next turn. */
	due: boolean;
	teams: OrgTeamResult[];
	goals: OrgGoalWidth[];
	signals: OrgSignals;
	friction_since: string;
	friction: OrgOwnerFriction;
	friction_baseline: OrgFrictionBaseline;
	/** Why the friction alone makes a review due, one line each. */
	friction_due: string[];
	last_review: { at: string; conclusion: string; teams: OrgTeamResult[] } | null;
}

/** One review of how the company is organised, in Exec's words. */
export interface OrgReviewEntry {
	id: number;
	reviewed_at: string;
	conclusion: string;
	signal_count: number;
}

export interface OrgReviewView {
	scorecard: OrgScorecard;
	/** Newest first. */
	reviews: OrgReviewEntry[];
}

export async function getOrgReview(company: string, signal?: AbortSignal): Promise<OrgReviewView> {
	return ownerJson<OrgReviewView>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/org-review`, {
			credentials: 'same-origin',
			cache: 'no-store',
			signal
		})
	);
}

/** The page's read, and the same read started early from a link to the page. */
export function orgReviewOptions(company: string) {
	return {
		queryKey: ['org-review', company] as const,
		queryFn: ({ signal }: { signal?: AbortSignal }) => getOrgReview(company, signal)
	};
}
