/* A model list a person can scan: the newest model of each family first, older versions folded
 * away, and a filter when a provider offers many (OpenRouter offers hundreds). The model in use
 * always stays in view. */

export type ModelChoice = { id: string; name: string; default?: boolean };

/** "Claude Sonnet 5.5" → family "claude sonnet", version 5.5. Names without a version are their
 * own family at version 0. */
export function family(name: string): { family: string; version: number } {
	const match = name.trim().match(/^(.*?)[\s-]*(\d+(?:\.\d+)*)\s*([a-z-]*)$/i);
	if (!match) return { family: name.trim().toLowerCase(), version: 0 };
	const parts = match[2].split('.').map(Number);
	const version = parts.reduce((total, part, index) => total + part / 1000 ** index, 0);
	return { family: `${match[1]} ${match[3]}`.trim().toLowerCase(), version };
}

/** Newest per family first (in the catalogue's order), then everything older. */
export function splitModels(
	models: ModelChoice[],
	current = ''
): { latest: ModelChoice[]; older: ModelChoice[] } {
	const newest = new Map<string, number>();
	for (const model of models) {
		const { family: name, version } = family(model.name || model.id);
		newest.set(name, Math.max(newest.get(name) ?? -1, version));
	}
	const latest: ModelChoice[] = [];
	const older: ModelChoice[] = [];
	for (const model of models) {
		const { family: name, version } = family(model.name || model.id);
		if (version === newest.get(name) || model.id === current || model.default) latest.push(model);
		else older.push(model);
	}
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
