import { keep } from '../search/search.ts';

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
		const key = familyKey(fact.base);
		const held = named.get(key);
		if (!held || (held.alias && !fact.alias)) named.set(key, fact);
	}
	const kept = facts.filter(
		(fact) => named.get(familyKey(fact.base)) === fact || fact.model.id === current
	);
	kept.sort((a, b) => b.version - a.version || a.index - b.index);
	const ordered = kept.map((fact) => fact.model);
	const latest = ordered.slice(0, count);
	const older = ordered.slice(count);
	const inUse = older.findIndex((model) => model.id === current);
	if (inUse >= 0) latest.push(...older.splice(inUse, 1));
	return { latest, older };
}

/** One key per release whatever the word order: catalogues list both "Claude Sonnet 4" and
 * "Claude 4 Sonnet" for one model (21 October). */
function familyKey(base: string): string {
	return base.toLowerCase().split(/[\s-]+/).filter(Boolean).sort().join(' ');
}

/** Models whose name or id matches what was typed (lib/search), newest first as listed. */
export function filterModels(models: ModelChoice[], query: string): ModelChoice[] {
	return keep(models, query, (model) => [model.name, model.id]);
}

/** A list long enough to need a filter. */
export const FILTER_FROM = 15;

/* Every model call resends the company's whole context, so a new connection starts on the
 * provider's everyday tier when it has one; the owner can still give any agent a stronger model. */
const EVERYDAY_TIER = /\b(terra|sonnet|flash)\b/i;

/** The model a new connection starts on: the newest everyday-tier model, else the provider's own
 * default, else the first listed. */
export function defaultChoice(models: ModelChoice[]): ModelChoice | undefined {
	return (
		splitModels(models).latest.find((model) => EVERYDAY_TIER.test(model.name || model.id)) ??
		models.find((model) => model.default) ??
		models[0]
	);
}
