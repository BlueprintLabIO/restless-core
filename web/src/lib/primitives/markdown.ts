/**
 * A bounded Markdown subset, parsed to a token tree.
 *
 * Employees write Markdown because models do, and an owner should read the structure
 * rather than the punctuation. What makes this safe is what it *doesn't* produce: there
 * is no HTML anywhere in the output, so there is nothing to sanitise and no `{@html}` at
 * the other end. `Markdown.svelte` renders these tokens through ordinary Svelte
 * interpolation, which escapes everything — a bug in this parser produces wrong
 * formatting, never script execution.
 *
 * Deliberately omitted: tables, images, footnotes, task lists, raw HTML passthrough, and
 * autolinking of bare URLs. A model that wants a table can write a list. Each is additive
 * later; none is worth the surface area now.
 */

/** Inline content — everything that can appear inside a line of text. */
export type InlineToken =
	| { kind: 'text'; value: string }
	| { kind: 'code'; value: string }
	| { kind: 'strong'; children: InlineToken[] }
	| { kind: 'emphasis'; children: InlineToken[] }
	| { kind: 'link'; href: string; children: InlineToken[] }
	| { kind: 'break' };

/** Block content — the shape of the message itself. */
export type BlockToken =
	| { kind: 'paragraph'; children: InlineToken[] }
	| { kind: 'heading'; level: 1 | 2 | 3 | 4 | 5 | 6; children: InlineToken[] }
	| { kind: 'code'; language: string | null; value: string }
	| { kind: 'list'; ordered: boolean; start: number; items: InlineToken[][] }
	| { kind: 'quote'; children: BlockToken[] }
	| { kind: 'rule' };

/**
 * The same ceiling the conversation command already enforces on a message
 * (`sendConversationMessageBodySchema`), so a reply can never cost more to render than it
 * cost to store. Anything beyond it is kept as literal text rather than dropped.
 */
const MAX_INPUT = 12_000;

/** How deep emphasis may nest before further markers are treated as literal characters. */
const MAX_INLINE_DEPTH = 6;

/** How deep blockquotes may nest. Beyond this the content is kept, the nesting is not. */
const MAX_QUOTE_DEPTH = 4;

/**
 * Schemes a link may carry. `javascript:`, `data:` and `vbscript:` are the classic
 * injection vectors; a scheme-less `//host` is protocol-relative and equally unreviewable.
 * A rejected link is not deleted — it degrades to its own visible text, so the owner still
 * sees what the employee wrote.
 */
const SAFE_SCHEME = /^(?:https?:\/\/|mailto:)[^\s<>]+$/i;

export function isSafeHref(href: string): boolean {
	const trimmed = href.trim();
	if (!trimmed || trimmed.length > 2_048) return false;
	// Control characters are how `java\nscript:` slips past a naive scheme test.
	// eslint-disable-next-line no-control-regex
	if (/[\u0000-\u0020\u007f]/.test(trimmed)) return false;
	return SAFE_SCHEME.test(trimmed);
}

/* ============ inline ============ */

function pushText(tokens: InlineToken[], value: string): void {
	if (!value) return;
	const last = tokens[tokens.length - 1];
	if (last?.kind === 'text') last.value += value;
	else tokens.push({ kind: 'text', value });
}

/**
 * Scans one line's worth of text. `depth` bounds emphasis nesting; at the ceiling every
 * marker is literal, which is also what makes a pathological `*`-only input cheap.
 */
function parseInline(source: string, depth = 0): InlineToken[] {
	const tokens: InlineToken[] = [];
	let index = 0;
	let plain = '';

	const flush = () => {
		pushText(tokens, plain);
		plain = '';
	};

	while (index < source.length) {
		const rest = source.slice(index);

		// Inline code first: nothing inside a code span is markup.
		const code = /^(`+)([\s\S]*?)\1/.exec(rest);
		if (code) {
			flush();
			tokens.push({ kind: 'code', value: code[2].trim() });
			index += code[0].length;
			continue;
		}

		// Escapes: \* is a literal asterisk.
		if (rest[0] === '\\' && rest.length > 1 && /[\\`*_[\]()#>-]/.test(rest[1])) {
			plain += rest[1];
			index += 2;
			continue;
		}

		if (depth < MAX_INLINE_DEPTH) {
			// Strong before emphasis, so `**x**` is not read as two emphases.
			const strong = /^(\*\*|__)(?=\S)([\s\S]+?)(?<=\S)\1/.exec(rest);
			if (strong) {
				flush();
				tokens.push({ kind: 'strong', children: parseInline(strong[2], depth + 1) });
				index += strong[0].length;
				continue;
			}
			const emphasis = /^(\*|_)(?=\S)([\s\S]+?)(?<=\S)\1/.exec(rest);
			if (emphasis) {
				flush();
				tokens.push({ kind: 'emphasis', children: parseInline(emphasis[2], depth + 1) });
				index += emphasis[0].length;
				continue;
			}
			const link = /^\[([^\]]*)\]\(([^()\s]*)\)/.exec(rest);
			if (link) {
				const label = link[1];
				const href = link[2];
				flush();
				if (isSafeHref(href)) {
					tokens.push({ kind: 'link', href: href.trim(), children: parseInline(label, depth + 1) });
				} else {
					// Refused, not removed: the owner still reads exactly what was written.
					pushText(tokens, link[0]);
				}
				index += link[0].length;
				continue;
			}
		}

		plain += rest[0];
		index += 1;
	}

	flush();
	return tokens;
}

/** Joins the lines of one paragraph, keeping intentional line breaks as `break` tokens. */
function parseInlineLines(lines: string[]): InlineToken[] {
	const tokens: InlineToken[] = [];
	lines.forEach((line, position) => {
		if (position > 0) tokens.push({ kind: 'break' });
		tokens.push(...parseInline(line));
	});
	return tokens;
}

/* ============ blocks ============ */

const HEADING = /^(#{1,6})\s+(.*)$/;
const FENCE = /^(?:```|~~~)\s*([A-Za-z0-9_+-]*)\s*$/;
const RULE = /^(?:\s*(?:-{3,}|\*{3,}|_{3,})\s*)$/;
const BULLET = /^[-*+]\s+(.*)$/;
const ORDERED = /^(\d{1,9})[.)]\s+(.*)$/;
const QUOTE = /^>\s?(.*)$/;

/**
 * Parses a message into blocks. Every branch either consumes its lines or falls through to
 * a paragraph, so no input can be swallowed: unterminated syntax degrades to literal text
 * rather than eating the rest of the reply.
 */
export function parseMarkdown(input: string, quoteDepth = 0): BlockToken[] {
	const source = typeof input === 'string' ? input : '';
	const text = source.length > MAX_INPUT ? source.slice(0, MAX_INPUT) : source;
	const lines = text.replace(/\r\n?/g, '\n').split('\n');
	const blocks: BlockToken[] = [];
	let index = 0;

	while (index < lines.length) {
		const line = lines[index];

		if (!line.trim()) {
			index += 1;
			continue;
		}

		const fence = FENCE.exec(line);
		if (fence) {
			const body: string[] = [];
			let cursor = index + 1;
			let closed = false;
			while (cursor < lines.length) {
				if (
					FENCE.test(lines[cursor]) &&
					!lines[cursor]
						.trim()
						.replace(/^(?:```|~~~)/, '')
						.trim()
				) {
					closed = true;
					break;
				}
				body.push(lines[cursor]);
				cursor += 1;
			}
			// An unclosed fence still renders as code to the end of the message — that is what
			// the employee meant, and it beats spilling backticks through the reply.
			blocks.push({ kind: 'code', language: fence[1] || null, value: body.join('\n') });
			index = closed ? cursor + 1 : cursor;
			continue;
		}

		if (RULE.test(line)) {
			blocks.push({ kind: 'rule' });
			index += 1;
			continue;
		}

		const heading = HEADING.exec(line);
		if (heading) {
			blocks.push({
				kind: 'heading',
				level: heading[1].length as 1 | 2 | 3 | 4 | 5 | 6,
				children: parseInline(heading[2].trim())
			});
			index += 1;
			continue;
		}

		const quote = QUOTE.exec(line);
		if (quote) {
			const body: string[] = [];
			let cursor = index;
			while (cursor < lines.length) {
				const inner = QUOTE.exec(lines[cursor]);
				if (inner) body.push(inner[1]);
				else if (lines[cursor].trim() && body.length) body.push(lines[cursor]);
				else break;
				cursor += 1;
			}
			blocks.push(
				quoteDepth < MAX_QUOTE_DEPTH
					? { kind: 'quote', children: parseMarkdown(body.join('\n'), quoteDepth + 1) }
					: { kind: 'paragraph', children: parseInlineLines(body) }
			);
			index = cursor;
			continue;
		}

		const bullet = BULLET.exec(line);
		const ordered = ORDERED.exec(line);
		if (bullet || ordered) {
			const isOrdered = Boolean(ordered);
			const start = ordered ? Number.parseInt(ordered[1], 10) : 1;
			const items: InlineToken[][] = [];
			let cursor = index;
			while (cursor < lines.length) {
				const current = lines[cursor];
				const nextBullet = BULLET.exec(current);
				const nextOrdered = ORDERED.exec(current);
				const matched = isOrdered ? nextOrdered : nextBullet;
				// A switch of marker kind starts a new list rather than silently absorbing it.
				if (!matched || Boolean(nextOrdered) !== isOrdered) break;
				const itemLines = [isOrdered ? nextOrdered![2] : nextBullet![1]];
				cursor += 1;
				// A plain indented continuation line belongs to the item above it.
				while (cursor < lines.length && /^\s{2,}\S/.test(lines[cursor])) {
					if (BULLET.test(lines[cursor].trim()) || ORDERED.test(lines[cursor].trim())) break;
					itemLines.push(lines[cursor].trim());
					cursor += 1;
				}
				items.push(parseInlineLines(itemLines));
			}
			blocks.push({ kind: 'list', ordered: isOrdered, start, items });
			index = cursor;
			continue;
		}

		// Paragraph: everything up to a blank line or the start of another block.
		const paragraph: string[] = [];
		let cursor = index;
		while (cursor < lines.length) {
			const current = lines[cursor];
			if (!current.trim()) break;
			if (cursor > index) {
				if (
					HEADING.test(current) ||
					FENCE.test(current) ||
					RULE.test(current) ||
					QUOTE.test(current) ||
					BULLET.test(current) ||
					ORDERED.test(current)
				) {
					break;
				}
			}
			paragraph.push(current);
			cursor += 1;
		}
		blocks.push({ kind: 'paragraph', children: parseInlineLines(paragraph) });
		index = cursor;
	}

	return blocks;
}

/** True when the text carries anything this renderer would present differently. */
export function looksLikeMarkdown(text: string): boolean {
	return /(^|\n)\s*(?:#{1,6}\s|[-*+]\s|\d{1,9}[.)]\s|>|```|~~~)|\*\*|__|`[^`]+`|\[[^\]]*\]\(/.test(
		text
	);
}
