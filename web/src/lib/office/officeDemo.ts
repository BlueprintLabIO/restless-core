import type { CockpitTeam } from '$lib/model/cockpit';
import { DEFAULT_OFFICE_PREFERENCES, type OfficePreferences } from '$lib/office/officePlan';
import type { OfficeMember } from '$lib/office/projection';

const CREATED_AT = '2026-08-21T00:00:00Z';

const teamSources = [
	{ id: 'design', name: 'Design', brief: 'Shape a clear and humane product.' },
	{ id: 'engineering', name: 'Engineering', brief: 'Build and operate the product.' },
	{ id: 'research', name: 'Research', brief: 'Turn evidence into direction.' },
	{ id: 'operations', name: 'Operations', brief: 'Keep the company moving.' }
] as const;

const people = [
	['anika', 'Anika', 'Design lead', 'design'],
	['jules', 'Jules', 'Experience designer', 'design'],
	['theo', 'Theo', 'Visual designer', 'design'],
	['imani', 'Imani', 'Content designer', 'design'],
	['hana', 'Hana', 'Engineering lead', 'engineering'],
	['omar', 'Omar', 'Frontend engineer', 'engineering'],
	['leo', 'Leo', 'Systems engineer', 'engineering'],
	['priya', 'Priya', 'Product engineer', 'engineering'],
	['sora', 'Sora', 'Research lead', 'research'],
	['mateo', 'Mateo', 'Market researcher', 'research'],
	['nia', 'Nia', 'Insight analyst', 'research'],
	['beck', 'Beck', 'Customer researcher', 'research'],
	['mei', 'Mei', 'Operations lead', 'operations'],
	['arun', 'Arun', 'Company operator', 'operations'],
	['camille', 'Camille', 'Programme operator', 'operations'],
	['noah', 'Noah', 'Customer partner', 'operations']
] as const;

export const OFFICE_DEMO_TEAMS: CockpitTeam[] = teamSources.map((team) => ({
	...team,
	outcome_standard: 'exceptional',
	outcome_standard_source: 'company_default',
	standard_source_message_id: null,
	frontier_phase: 'commissioned',
	lead_actor_id: people.find((person) => person[3] === team.id)?.[0] ?? '',
	created_by: 'demo',
	created_at: CREATED_AT,
	member_count: people.filter((person) => person[3] === team.id).length,
	in_motion_count: 0,
	blocked_count: 0
}));

export const OFFICE_DEMO_MEMBERS: OfficeMember[] = people.map(
	([actorId, display, role, teamId], index) => {
		const team = OFFICE_DEMO_TEAMS.find((candidate) => candidate.id === teamId)!;
		return {
			actorId,
			numericId: index + 1,
			display,
			role,
			teamId,
			teamName: team.name,
			isTeamLead: team.lead_actor_id === actorId,
			palette: index % 6,
			presence: 'available',
			presenceLabel: 'Available',
			semanticActivity: false,
			sessionObserved: false,
			presenceObservedAt: null,
			currentStep: null,
			work: null,
			attempt: null,
			attemptLabel: 'No Attempt started',
			activityDetail: 'Taking a restorative pause in the shared commons.',
			evidenceLabel: 'Demo scene',
			outputCount: 0,
			workHref: null,
			personHref: ''
		};
	}
);

export const OFFICE_DEMO_PREFERENCES: OfficePreferences = {
	...DEFAULT_OFFICE_PREFERENCES,
	decorDensity: 'lush',
	pets: true,
	decorations: []
};
