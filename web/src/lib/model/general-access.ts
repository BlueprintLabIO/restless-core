/* Sharing a document or a sheet reads the same in both: one general access for the company's
 * people, then the people added by name. Exec and Staff never see anything through general
 * access; they are added like anyone else. */

/** What every person in the company may do without being added. */
export type GeneralAccess = 'private' | 'view' | 'comment' | 'edit';

export const PRIVATE_HINT = 'Only people added can open it';

/** The levels documents and sheets both offer. */
export const GENERAL_ACCESS_OPTIONS: ReadonlyArray<{ value: GeneralAccess; label: string }> = [
	{ value: 'private', label: 'Private' },
	{ value: 'view', label: 'Company can view' },
	{ value: 'comment', label: 'Company can comment' },
	{ value: 'edit', label: 'Company can edit' }
];


export interface ShareCandidate {
	actor_id: string;
	display: string;
	kind?: string;
}

/** The colleagues a name finds, people and agents alike, leaving out the system and anyone
 * already with access. Names starting with the query come first. */
export function matchPeople<T extends ShareCandidate>(
	people: readonly T[],
	query: string,
	exclude: ReadonlySet<string>,
	limit = 6
): T[] {
	const needle = query.trim().toLowerCase();
	if (!needle) return [];
	const open = people.filter(
		(person) => person.kind !== 'system' && !exclude.has(person.actor_id)
	);
	const starts = open.filter((person) => person.display.toLowerCase().startsWith(needle));
	const contains = open.filter(
		(person) =>
			!starts.includes(person) &&
			(person.display.toLowerCase().includes(needle) ||
				person.actor_id.toLowerCase().includes(needle))
	);
	return [...starts, ...contains].slice(0, limit);
}
