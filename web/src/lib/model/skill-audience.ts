import { personName } from './initials.ts';

/** Compress observed membership only when every current member is represented. */
export function skillAudience(
	ids: string[],
	people: { actor_id: string; display: string; team_id?: string | null }[],
	teams: { id: string; name: string; lead_actor_id?: string | null }[],
	activeIds: string[]
) {
	const selected = new Set(ids);
	const all = new Set(activeIds);
	const names = ids.map((id) => personName(people.find((p) => p.actor_id === id)?.display ?? id));
	if (!selected.size) return { label: 'None', title: 'No employees', count: 0 };
	if (all.size && selected.size === all.size && [...all].every((id) => selected.has(id)))
		return { label: 'All', title: names.join(', '), count: selected.size };
	const remaining = new Set(selected);
	const labels: string[] = [];
	for (const team of teams) {
		const members = activeIds.filter(
			(id) =>
				people.some((p) => p.actor_id === id && p.team_id === team.id) || team.lead_actor_id === id
		);
		if (members.length > 1 && members.every((id) => remaining.has(id))) {
			labels.push(team.name);
			members.forEach((id) => remaining.delete(id));
		}
	}
	labels.push(
		...[...remaining].map((id) => personName(people.find((p) => p.actor_id === id)?.display ?? id))
	);
	return {
		label:
			labels.length > 3
				? `${labels.slice(0, 2).join(', ')} +${labels.length - 2}`
				: labels.join(', '),
		title: names.join(', '),
		count: selected.size
	};
}
