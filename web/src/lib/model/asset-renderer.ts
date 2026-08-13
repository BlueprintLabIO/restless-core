/**
 * Typed asset renderers.
 *
 * An asset's content is stored as free JSON, but how it should be *shown* depends on its shape: a document or
 * report reads as text, a dataset as a table, a presentation as a sequence of slides, a design as an image.
 * This pure dispatcher inspects the recorded content and returns a typed render descriptor so the asset canvas
 * can render each kind appropriately (and the owner still reviews it with the same controls). A generated
 * company application or arbitrary interface (content with an `html` body) is rendered in a strictly sandboxed
 * iframe (`app` kind — see the asset canvas), while live external pages are rendered by the sandboxed
 * URL-preview surface that sits ahead of this dispatcher; anything this dispatcher does not recognise falls back
 * to a readable raw view rather than a blank panel.
 */

export type AssetRenderKind = 'app' | 'text' | 'table' | 'slides' | 'image' | 'raw';

export interface AssetRender {
	kind: AssetRenderKind;
	/** For `app` — generated HTML rendered in a strict sandbox (no same-origin access). */
	html?: string;
	/** For `text`. */
	text?: string;
	/** For `table`. */
	table?: { columns: string[]; rows: string[][] };
	/** For `slides`. */
	slides?: Array<{ title: string | null; body: string }>;
	/** For `image`. */
	imageUrl?: string;
	/** For `raw` — a pretty-printed JSON fallback. */
	raw?: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function cell(value: unknown): string {
	if (value === null || value === undefined) return '';
	if (typeof value === 'object') return JSON.stringify(value);
	return String(value);
}

function tableFromRows(rows: readonly Record<string, unknown>[]): AssetRender {
	const columns: string[] = [];
	for (const row of rows) {
		for (const key of Object.keys(row)) if (!columns.includes(key)) columns.push(key);
	}
	return {
		kind: 'table',
		table: { columns, rows: rows.map((row) => columns.map((column) => cell(row[column]))) }
	};
}

/** Chooses a typed renderer for an asset's recorded content. Pure and deterministic. */
export function renderAsset(content: unknown): AssetRender {
	// Generated application / arbitrary interface — an HTML body, rendered in a strict sandbox by the canvas.
	if (isRecord(content) && typeof content.html === 'string' && content.html.trim().length > 0) {
		return { kind: 'app', html: content.html };
	}
	// Presentation — a list of slides.
	if (isRecord(content) && Array.isArray(content.slides)) {
		return {
			kind: 'slides',
			slides: (content.slides as unknown[]).map((slide) => {
				if (isRecord(slide)) {
					const title = slide.title ?? slide.heading ?? null;
					const body = slide.body ?? slide.text ?? slide.content ?? '';
					return { title: title == null ? null : String(title), body: cell(body) };
				}
				return { title: null, body: cell(slide) };
			})
		};
	}
	// Design / image — a single image reference.
	if (isRecord(content)) {
		const image = content.imageUrl ?? content.image ?? content.src;
		if (typeof image === 'string' && image.length > 0) return { kind: 'image', imageUrl: image };
	}
	// Dataset — an array of row objects, or an explicit rows/columns shape.
	if (Array.isArray(content) && content.every(isRecord)) {
		return tableFromRows(content as Record<string, unknown>[]);
	}
	if (
		isRecord(content) &&
		Array.isArray(content.rows) &&
		(content.rows as unknown[]).every(isRecord)
	) {
		return tableFromRows(content.rows as Record<string, unknown>[]);
	}
	// Document / report — a text body under a common key, or a bare string.
	if (typeof content === 'string') return { kind: 'text', text: content };
	if (isRecord(content)) {
		const text =
			content.text ?? content.body ?? content.summary ?? content.markdown ?? content.report;
		if (typeof text === 'string') return { kind: 'text', text };
	}
	// Fallback — show the recorded content readably rather than a blank panel.
	return { kind: 'raw', raw: JSON.stringify(content, null, 2) };
}
