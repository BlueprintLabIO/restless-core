/** One line of plain prose from a Markdown snippet, for places that show a
 * summary rather than the document (a table cell, a tooltip). */
export function plainText(markdown: string, { dropTitle = false } = {}): string {
	// A document's own title line repeats its name; a summary does not need it.
	const body = dropTitle ? markdown.replace(/^\s*#\s+[^\n]*\n+/, '') : markdown;
	return body
		.replace(/```[\s\S]*?```/g, ' ')
		.replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
		.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
		.replace(/^\s{0,3}(#{1,6}|>|[-*+]|\d+\.)\s+/gm, '')
		.replace(/(\*\*|__|\*|_|`|~~)(.+?)\1/g, '$2')
		.replace(/\s+/g, ' ')
		.trim();
}
