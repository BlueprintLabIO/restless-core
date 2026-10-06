/* Brand marks for the catalogued services, so an app is recognised at a glance.
 * Sources and licences: static/licenses/app-logos.txt. Marks belong to their
 * owners and only identify the service a connection reaches. */

const files = import.meta.glob('../assets/app-logos/*.{svg,png}', {
	eager: true,
	query: '?url',
	import: 'default'
}) as Record<string, string>;

const byName = Object.fromEntries(
	Object.entries(files).map(([path, url]) => [path.replace(/^.*\/|\.\w+$/g, ''), url])
);

/** Icons that carry their own background and fill the tile edge to edge. */
const FULL_BLEED = new Set(['context7', 'deepwiki', 'gamma']);
/** Site icons drawn with a wide margin of their own; shown larger to match the rest. */
const MARGINED = new Set(['klaviyo', 'monday']);

/** Catalogue keys whose file is named differently. */
const FILE_FOR: Record<string, string> = { 'cloudflare-docs': 'cloudflare' };

export type AppLogo = { src: string; fill: boolean; margined: boolean };

export function appLogo(catalogueKey: string | undefined | null): AppLogo | undefined {
	if (!catalogueKey) return undefined;
	const name = FILE_FOR[catalogueKey] ?? catalogueKey;
	const src = byName[name];
	return src ? { src, fill: FULL_BLEED.has(name), margined: MARGINED.has(name) } : undefined;
}
