/* The Apps catalogue drawn from public registries (Sprint 87): services from the official MCP
 * Registry and know-how from Anthropic's public skills, refreshed daily on the plane. Browse
 * shows them beside the hand-checked catalogue; nothing installs until the owner clicks. */
import { responseFailure } from './failure.ts';

export type RegistryItem = {
	key: string;
	kind: 'mcp' | 'skill';
	name: string;
	summary: string;
	/** "MCP Registry" or "Anthropic skills". */
	source: string;
	homepage: string | null;
	/** A service's remote address. */
	endpoint: string | null;
	/** A skill's files, relative to its folder. */
	files: string[];
	runs_scripts: boolean;
};

export type RegistryCatalogue = {
	updated_at: number | null;
	items: RegistryItem[];
	unavailable: string[];
};

export async function fetchAppCatalogue(): Promise<RegistryCatalogue> {
	const response = await fetch('/api/app-catalogue', { credentials: 'same-origin' });
	if (!response.ok) throw await responseFailure(response);
	return response.json();
}

/** Install a catalogue skill as candidate know-how. */
export async function installFromCatalogue(
	company: string,
	key: string
): Promise<{ skill: { name: string }; runs_scripts: boolean }> {
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/app-catalogue/install`,
		{
			method: 'POST',
			credentials: 'same-origin',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ key })
		}
	);
	if (!response.ok) throw await responseFailure(response);
	return response.json();
}

/** A connection-safe key for a registry service: lowercase words, at most 40 characters. */
export function registryKey(name: string): string {
	const slug = name
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, 34)
		.replace(/-+$/, '');
	return `r-${slug || 'service'}`;
}
