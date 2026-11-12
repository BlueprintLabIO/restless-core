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

export interface OrgScorecard {
	since: string;
	signals_since: string;
	signals_through: string;
	/** Enough was said for the company to review itself on Exec's next turn. */
	due: boolean;
	teams: OrgTeamResult[];
	goals: OrgGoalWidth[];
	signals: OrgSignals;
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
