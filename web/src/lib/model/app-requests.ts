/* App requests (Sprint 63, finishing S57-T7): an owner handoff in which Exec
 * or a lead names the app that would unblock its Work. OrgIntel owns the
 * handoff; allowing the matching app resolves it as observed, so the Work
 * resumes without the owner reporting back. */

import { responseFailure } from './failure.ts';
import { CATALOGUE } from './apps.ts';

type AppRequestRow = {
	handoff_id: string;
	app: string;
	requested_action: string;
	work_id: string;
	work_title?: string | null;
	requested_by: string;
	requested_by_display?: string;
};

export type AppRequest = AppRequestRow & {
	/** The app's display name: a catalogue name, or the link as given. */
	name: string;
	/** Who asked, in words: Exec, or the lead or worker by role. */
	asker: string;
	/** Why, in the requester's words. */
	reason: string;
	href: (companyId: string) => string;
};

function toRequest(row: AppRequestRow): AppRequest {
	const entry = CATALOGUE.find(
		(candidate) => candidate.key === row.app || candidate.endpoint === row.app
	);
	const words = row.requested_by.replace(/[-_]+/g, ' ').trim();
	return {
		...row,
		name: entry?.name ?? row.app,
		asker:
			row.requested_by === 'exec'
				? 'Exec'
				: row.requested_by_display && row.requested_by_display !== row.requested_by
					? row.requested_by_display
					: words.charAt(0).toUpperCase() + words.slice(1),
		reason: row.requested_action,
		href: (companyId) =>
			`/${encodeURIComponent(companyId)}/apps/${encodeURIComponent(entry ? entry.key : `link:${row.app}`)}?request=${encodeURIComponent(row.handoff_id)}`
	};
}

export async function fetchAppRequests(company: string): Promise<AppRequest[]> {
	const response = await fetch(`/api/companies/${encodeURIComponent(company)}/app-requests`, {
		credentials: 'same-origin',
		cache: 'no-store'
	});
	if (!response.ok) throw await responseFailure(response);
	const body = (await response.json()) as { requests: AppRequestRow[] };
	return body.requests.map(toRequest);
}
