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
