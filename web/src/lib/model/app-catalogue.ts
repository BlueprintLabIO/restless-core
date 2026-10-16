import { bootFetch } from './boot-fetch.ts';
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
	refreshing?: boolean;
	sources?: Record<string, { updated_at: number | null; error: string | null }>;
	updated_at: number | null;
	items: RegistryItem[];
	unavailable: string[];
};

export async function fetchAppCatalogue(
	signal?: AbortSignal,
	refresh = false
): Promise<RegistryCatalogue> {
	const response = await bootFetch('/api/app-catalogue' + (refresh ? '?refresh=true' : ''), {
		credentials: 'same-origin',
		signal
	});
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

/** Route identity is a reversible encoding of the registry's canonical key, never its title. */
export function registryKey(key: string): string {
	const bytes = new TextEncoder().encode(key);
	return `r-${btoa(String.fromCharCode(...bytes))
		.replaceAll('+', '-')
		.replaceAll('/', '_')
		.replace(/=+$/, '')}`;
}

/** Old bookmarks are resolved only when exactly one app has this former display-name key. */
export function legacyRegistryKey(name: string): string {
	const slug = name
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, 34)
		.replace(/-+$/, '');
	return `r-${slug || 'service'}`;
}

/** Connection names have a separate bounded format; the source retains the full catalogue ID. */
export async function catalogueConnectionName(entry: {
	key: string;
	registryId?: string;
}): Promise<string> {
	if (!entry.registryId) return entry.key;
	const hash = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(entry.registryId));
	return `r-${Array.from(new Uint8Array(hash).slice(0, 16), (byte) => byte.toString(16).padStart(2, '0')).join('')}`;
}
