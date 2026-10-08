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

/** Two letters for an avatar. A team shares its lead's initial, so one letter cannot tell its
 * people apart: a one-word name shows its first two letters ("Bea" is Be, "Bodhi" Bo), and two
 * words their initials. Leading articles and a trailing number are skipped ("The Exec" is Ex). */
export function initials(name: string): string {
	const words = name
		.split(/\s+/)
		.filter((part) => part && !/^(the|a|an)$/i.test(part) && !/^\d+$/.test(part));
	const [first = '', second] = words;
	if (second && /^\p{L}/u.test(second))
		return `${first[0] ?? ''}${second[0] ?? ''}`.toUpperCase();
	return `${first.charAt(0).toUpperCase()}${first.charAt(1).toLowerCase()}`;
}
