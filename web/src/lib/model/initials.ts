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

/** Up to two initials for an avatar. Leading articles are skipped, so
 * "The Exec" reads E wherever a person appears. */
export function initials(name: string): string {
	return name
		.split(/\s+/)
		.filter((part) => part && !/^(the|a|an)$/i.test(part))
		.slice(0, 2)
		.map((part) => part[0]?.toUpperCase() ?? '')
		.join('');
}
