/* Inviting takes what people paste: several addresses separated by commas, semicolons, spaces or
 * new lines, with "Name <address>" as mail clients copy it. Each address is invited once. */

const EMAIL = /^[^\s@<>(),;:"]+@[^\s@<>(),;:"]+\.[^\s@<>(),;:".]{2,}$/u;

export function looksLikeEmail(value: string): boolean {
	return EMAIL.test(value.trim());
}

export function parseInviteEmails(text: string): { emails: string[]; invalid: string[] } {
	const emails: string[] = [];
	const invalid: string[] = [];
	const seen = new Set<string>();
	const unwrapped = text.replace(/[^<>,;\n]*<([^<>]+)>/gu, ' $1 ');
	for (const token of unwrapped.split(/[\s,;]+/u)) {
		const candidate = token.trim();
		if (!candidate) continue;
		if (!looksLikeEmail(candidate)) {
			invalid.push(candidate);
			continue;
		}
		const key = candidate.toLowerCase();
		if (seen.has(key)) continue;
		seen.add(key);
		emails.push(candidate);
	}
	return { emails, invalid };
}
