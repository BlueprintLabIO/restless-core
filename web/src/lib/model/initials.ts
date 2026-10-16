/** The name a person goes by on screen. The executive actor is stored as "The Exec" but is
 * called Exec everywhere the owner reads it: the rail, the chat, People and the Library. */
export function personName(display: string): string {
	return display === 'The Exec' ? 'Exec' : display;
}

/** The name a team goes by on screen. Exec names teams with a handle such as
 * "aris-sales-readiness"; read as a name it is "Sales readiness". A name Exec
 * wrote as words is kept as written. */
export function teamName(name: string, companyId = ''): string {
	if (!/^[a-z0-9]+(?:[-_][a-z0-9]+)+$/.test(name)) return name;
	const bare =
		companyId && name.startsWith(`${companyId}-`) ? name.slice(companyId.length + 1) : name;
	const words = bare.replace(/[-_]+/g, ' ');
	return words.charAt(0).toUpperCase() + words.slice(1);
}

/** A name prefix for an avatar. Team characters are allocated in prefix order, so
 * multiword names keep that prefix too (Ebenezer Scrooge → Eb, Edna Mode → Ed).
 * A repeated character retains its number: Eeyore 2 → Ee2, distinct from Eeyore. */
export function initials(name: string): string {
	const words = name
		.trim()
		.split(/\s+/)
		.filter((part) => part && !/^(the|a|an)$/i.test(part));
	const round = words.length > 1 && /^\d+$/.test(words.at(-1) ?? '') ? (words.pop() ?? '') : '';
	const letters = [...(words[0] ?? '').replace(/[^\p{L}\p{N}]/gu, '')];
	return (letters[0]?.toUpperCase() ?? '') + (letters[1]?.toLowerCase() ?? '') + round;
}
