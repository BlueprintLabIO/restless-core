/* A model list a person can scan: the newest releases first, older ones folded away, and a filter
 * when a provider offers many (OpenRouter offers hundreds). The model in use always stays in view. */

export type ModelChoice = { id: string; name: string; default?: boolean };

/** How many of the newest releases show before "Older models". */
export const LATEST_COUNT = 5;

const ALIAS = /\s*\(\s*latest\s*\)|[\s-]+latest\b/gi;

/** "Claude Opus 4.5 (latest)" → base "Claude Opus 4.5", version 4.5, an alias. The version is
 * the first number in the name wherever it sits; dates (20250514) are not versions. A name with
 * no version is version 0. */
export function release(name: string): { base: string; version: number; alias: boolean } {
	const trimmed = name.trim();
	const alias = /\blatest\b/i.test(trimmed);
	const base = trimmed.replace(ALIAS, '').replace(/\s+/g, ' ').trim();
	let version = 0;
	for (const match of base.matchAll(/(?<![\d.])(\d+(?:\.\d+){0,2})(?![\d.])/g)) {
		const parts = match[1].split('.').map(Number);
		if (parts[0] >= 1000) continue;
		version = parts.reduce((total, part, index) => total + part / 1000 ** index, 0);
		break;
	}
	return { base, version, alias };
}

/** The newest releases first (highest version, then the catalogue's order), then everything older.
 * A "(latest)" alias of a model already listed is left out unless it is the one in use. */
export function splitModels(
	models: ModelChoice[],
	current = '',
	count = LATEST_COUNT
): { latest: ModelChoice[]; older: ModelChoice[] } {
	const facts = models.map((model, index) => ({ model, index, ...release(model.name || model.id) }));
	const named = new Map<string, (typeof facts)[number]>();
	for (const fact of facts) {
		const key = fact.base.toLowerCase();
		const held = named.get(key);
		if (!held || (held.alias && !fact.alias)) named.set(key, fact);
	}
	const kept = facts.filter(
		(fact) => named.get(fact.base.toLowerCase()) === fact || fact.model.id === current
	);
	kept.sort((a, b) => b.version - a.version || a.index - b.index);
	const ordered = kept.map((fact) => fact.model);
	const latest = ordered.slice(0, count);
	const older = ordered.slice(count);
	const inUse = older.findIndex((model) => model.id === current);
	if (inUse >= 0) latest.push(...older.splice(inUse, 1));
	return { latest, older };
}

/** Models whose name or id contains every word typed. */
export function filterModels(models: ModelChoice[], query: string): ModelChoice[] {
	const words = query.toLowerCase().split(/\s+/).filter(Boolean);
	if (!words.length) return models;
	return models.filter((model) => {
		const text = `${model.name} ${model.id}`.toLowerCase();
		return words.every((word) => text.includes(word));
	});
}

/** A list long enough to need a filter. */
export const FILTER_FROM = 15;
