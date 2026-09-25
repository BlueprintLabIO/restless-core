import { MODEL_PRESETS } from './model-presets';
export type CatalogProvider = { id: string; name: string; models: { id: string; name: string }[] };
export type CatalogSnapshot = { updatedAt: number; providers: CatalogProvider[] };
const aliases: Record<string, string> = { moonshot: 'moonshotai' };
function record(value: unknown): Record<string, unknown> {
	return value && typeof value === 'object' && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: {};
}
function text(value: unknown): value is string {
	return (
		typeof value === 'string' &&
		value.length > 0 &&
		value.length <= 200 &&
		!/[\x00-\x1f]/.test(value)
	);
}
export function parseCatalog(value: unknown): CatalogProvider[] {
	const root = record(value);
	let found = 0;
	const providers = MODEL_PRESETS.map((p) => {
		// Subscription Codex has its own model availability; the OpenAI API
		// catalog is not evidence that a model works with a Codex sign-in.
		if (p.id === 'openai-codex') return p;
		// A local gateway has its own catalog; do not invent models for it.
		const entries = record(record(root[aliases[p.id] ?? p.id]).models);
		const models = Object.values(entries)
			.map(record)
			.filter(
				(m) =>
					text(m.id) &&
					text(m.name) &&
					m.tool_call === true &&
					m.status !== 'deprecated' &&
					Array.isArray(record(m.modalities).output) &&
					(record(m.modalities).output as unknown[]).includes('text')
			)
			.sort(
				(a, b) =>
					String(b.release_date ?? '').localeCompare(String(a.release_date ?? '')) ||
					String(a.id).localeCompare(String(b.id))
			)
			.map((m) => ({ id: m.id as string, name: m.name as string }));
		if (models.length) found++;
		return { id: p.id, name: p.name, models: models.length ? models : p.models };
	});
	if (!found) throw new Error('The model catalog contained no supported providers.');
	return providers;
}
export function readSnapshot(raw: string | null): CatalogSnapshot | undefined {
	try {
		const s = JSON.parse(raw ?? 'null');
		if (
			!s ||
			!Number.isFinite(s.updatedAt) ||
			s.updatedAt <= 0 ||
			s.updatedAt > Date.now() + 60000 ||
			!Array.isArray(s.providers) ||
			s.providers.length !== MODEL_PRESETS.length
		)
			return;
		const providers: CatalogProvider[] = s.providers.map((p: CatalogProvider, i: number) => {
			if (
				p.id !== MODEL_PRESETS[i].id ||
				!Array.isArray(p.models) ||
				!p.models.length ||
				p.models.length > 10000
			)
				throw new Error();
			return {
				id: p.id,
				name: MODEL_PRESETS[i].name,
				models: p.models.map((m) => {
					if (!text(m.id) || !text(m.name)) throw new Error();
					return { id: m.id, name: m.name };
				})
			};
		});
		return { updatedAt: s.updatedAt, providers };
	} catch {
		return;
	}
}
