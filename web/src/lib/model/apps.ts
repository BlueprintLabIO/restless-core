/* Apps (Sprint 63): one owner view of everything that gives the company an
 * ability. An App is a projection, not a record: it joins the Authority
 * connections (ADR 0014), the company skill library (Sprint 55), plugin origin
 * (a connection's `source`, a skill's `origin_url`) and Exec's pending app
 * requests. Each underlying record keeps its one owner and its own rules.
 *
 * Owner language only: what an app lets the company do and how it is added.
 * The mechanism (MCP, skill, plugin, CLI) appears in tooltips, never as the
 * headline. */

import type { SkillLibrary, SkillRow } from './skills';
import type { ToolConnection } from './connections';

export type AppCategory =
	| 'Communication'
	| 'Engineering'
	| 'Finance'
	| 'Sales'
	| 'Design'
	| 'Projects'
	| 'Documents'
	| 'Websites'
	| 'Know-how';

export const BROWSE_CATEGORIES: AppCategory[] = [
	'Communication',
	'Projects',
	'Engineering',
	'Finance',
	'Sales',
	'Design',
	'Documents',
	'Websites'
];

/** A known service address. It is not an adapter: adding it runs the same
 * probe, sign-in and grant as any MCP address, and the probe decides. */
export type CatalogueEntry = {
	key: string;
	name: string;
	description: string;
	category: AppCategory;
	endpoint: string;
	/** How the owner adds it, in owner words. */
	how: string;
	/** The mechanism, for the curious. */
	howTip: string;
	/** `token`: the service needs a Vault secret rather than a sign-in. */
	auth?: 'token';
};

const signIn = (service: string): Pick<CatalogueEntry, 'how' | 'howTip'> => ({
	how: `Sign in with ${service}`,
	howTip: `${service}'s own MCP server. You sign in with ${service}; the sign-in stays on your plane, never in the company computer.`
});
const open = (service: string): Pick<CatalogueEntry, 'how' | 'howTip'> => ({
	how: 'Nothing to sign in to',
	howTip: `${service}'s public MCP server. It needs no account.`
});

/* Every entry answered an unauthenticated MCP `initialize` on 6 October 2026,
 * either with an MCP sign-in challenge (401 + Bearer) or with a result. That
 * proves the address speaks MCP; it does not prove a signed-in run. Add an
 * entry only after the same probe passes, and remove one that stops passing. */
export const CATALOGUE: CatalogueEntry[] = [
	{
		key: 'slack',
		name: 'Slack',
		category: 'Communication',
		endpoint: 'https://mcp.slack.com/mcp',
		description: 'Read channels and post updates as the company.',
		...signIn('Slack')
	},
	{
		key: 'intercom',
		name: 'Intercom',
		category: 'Communication',
		endpoint: 'https://mcp.intercom.com/mcp',
		description: 'Customer conversations, contacts and help articles.',
		...signIn('Intercom')
	},
	{
		key: 'linear',
		name: 'Linear',
		category: 'Projects',
		endpoint: 'https://mcp.linear.app/mcp',
		description: 'Triage issues, update projects and comment.',
		...signIn('Linear')
	},
	{
		key: 'asana',
		name: 'Asana',
		category: 'Projects',
		endpoint: 'https://mcp.asana.com/v2/mcp',
		description: 'Tasks, projects and who is doing what.',
		...signIn('Asana')
	},
	{
		key: 'monday',
		name: 'monday.com',
		category: 'Projects',
		endpoint: 'https://mcp.monday.com/mcp',
		description: 'Boards, items and updates.',
		...signIn('monday.com')
	},
	{
		key: 'atlassian',
		name: 'Jira and Confluence',
		category: 'Projects',
		endpoint: 'https://mcp.atlassian.com/v1/mcp',
		description: 'Issues in Jira and pages in Confluence.',
		...signIn('Atlassian')
	},
	{
		key: 'github',
		name: 'GitHub',
		category: 'Engineering',
		endpoint: 'https://api.githubcopilot.com/mcp/',
		description: 'Issues, pull requests and repositories.',
		how: 'Uses a GitHub token',
		howTip: "GitHub's hosted MCP server, with a personal access token kept in your Vault.",
		auth: 'token'
	},
	{
		key: 'sentry',
		name: 'Sentry',
		category: 'Engineering',
		endpoint: 'https://mcp.sentry.dev/mcp',
		description: "Errors and releases for the company's services.",
		...signIn('Sentry')
	},
	{
		key: 'vercel',
		name: 'Vercel',
		category: 'Engineering',
		endpoint: 'https://mcp.vercel.com',
		description: "Deployments and logs for the company's sites.",
		...signIn('Vercel')
	},
	{
		key: 'cloudflare-docs',
		name: 'Cloudflare docs',
		category: 'Engineering',
		endpoint: 'https://docs.mcp.cloudflare.com/mcp',
		description: "Search Cloudflare's documentation.",
		...open('Cloudflare')
	},
	{
		key: 'context7',
		name: 'Context7',
		category: 'Engineering',
		endpoint: 'https://mcp.context7.com/mcp',
		description: 'Current documentation for libraries and frameworks.',
		...open('Context7')
	},
	{
		key: 'deepwiki',
		name: 'DeepWiki',
		category: 'Engineering',
		endpoint: 'https://mcp.deepwiki.com/mcp',
		description: 'Explanations of public code repositories.',
		...open('DeepWiki')
	},
	{
		key: 'stripe',
		name: 'Stripe',
		category: 'Finance',
		endpoint: 'https://mcp.stripe.com',
		description: 'Customers, invoices and payments.',
		...signIn('Stripe')
	},
	{
		key: 'paypal',
		name: 'PayPal',
		category: 'Finance',
		endpoint: 'https://mcp.paypal.com/mcp',
		description: 'Invoices, orders and disputes.',
		...signIn('PayPal')
	},
	{
		key: 'square',
		name: 'Square',
		category: 'Finance',
		endpoint: 'https://mcp.squareup.com/mcp',
		description: 'Payments, orders and catalogue.',
		...signIn('Square')
	},
	{
		key: 'xero',
		name: 'Xero',
		category: 'Finance',
		endpoint: 'https://mcp.xero.com/mcp',
		description: 'Invoices, bills and bank reconciliation.',
		...signIn('Xero')
	},
	{
		key: 'hubspot',
		name: 'HubSpot',
		category: 'Sales',
		endpoint: 'https://mcp.hubspot.com/',
		description: 'Contacts, deals and the sales pipeline.',
		...signIn('HubSpot')
	},
	{
		key: 'klaviyo',
		name: 'Klaviyo',
		category: 'Sales',
		endpoint: 'https://mcp.klaviyo.com/mcp',
		description: 'Email and SMS campaigns and audiences.',
		...signIn('Klaviyo')
	},
	{
		key: 'canva',
		name: 'Canva',
		category: 'Design',
		endpoint: 'https://mcp.canva.com/mcp',
		description: 'Designs, templates and exports.',
		...signIn('Canva')
	},
	{
		key: 'figma',
		name: 'Figma',
		category: 'Design',
		endpoint: 'https://mcp.figma.com/mcp',
		description: 'Read designs and their components.',
		...signIn('Figma')
	},
	{
		key: 'gamma',
		name: 'Gamma',
		category: 'Design',
		endpoint: 'https://mcp.gamma.app/mcp',
		description: 'Presentations and documents from an outline.',
		...signIn('Gamma')
	},
	{
		key: 'notion',
		name: 'Notion',
		category: 'Documents',
		endpoint: 'https://mcp.notion.com/mcp',
		description: 'Read and write pages and databases.',
		...signIn('Notion')
	},
	{
		key: 'box',
		name: 'Box',
		category: 'Documents',
		endpoint: 'https://mcp.box.com',
		description: 'Files and folders in Box.',
		...signIn('Box')
	},
	{
		key: 'airtable',
		name: 'Airtable',
		category: 'Documents',
		endpoint: 'https://mcp.airtable.com/mcp',
		description: 'Bases, tables and records.',
		...signIn('Airtable')
	},
	{
		key: 'webflow',
		name: 'Webflow',
		category: 'Websites',
		endpoint: 'https://mcp.webflow.com/mcp',
		description: 'Site pages, CMS items and publishing.',
		...signIn('Webflow')
	},
	{
		key: 'wix',
		name: 'Wix',
		category: 'Websites',
		endpoint: 'https://mcp.wix.com/mcp',
		description: 'Sites, bookings and store.',
		...signIn('Wix')
	}
];

export type AppState = 'in_use' | 'needs_you' | 'paused' | 'available';

export type App = {
	/** Stable route key: `c-<connection>`, `s-<skill>`, `p-<plugin>` or a catalogue key. */
	key: string;
	name: string;
	description: string;
	how: string;
	howTip: string;
	category: AppCategory;
	state: AppState;
	/** One owner sentence for why it needs them, when it does. */
	attention?: string;
	connections: ToolConnection[];
	skills: SkillRow[];
	catalogue?: CatalogueEntry;
	/** The plugin this app came from, as `<git url>`. */
	plugin?: string;
};

const SKILL_WORDS = /[-_]+/g;

export function skillName(name: string): string {
	const words = name.replace(SKILL_WORDS, ' ').trim();
	return words.charAt(0).toUpperCase() + words.slice(1);
}

/** The plugin URL a connection or skill came from, if any. */
export function pluginOrigin(source?: string | null): string | undefined {
	if (!source?.startsWith('plugin:')) return undefined;
	const rest = source.slice('plugin:'.length);
	const at = rest.lastIndexOf('@');
	return at > 0 ? rest.slice(0, at) : rest;
}

export function catalogueFor(connection: ToolConnection): CatalogueEntry | undefined {
	const endpoint = connection.endpoint?.replace(/\/+$/, '');
	return CATALOGUE.find(
		(entry) => entry.key === connection.name || entry.endpoint.replace(/\/+$/, '') === endpoint
	);
}

function connectionState(connection: ToolConnection): { state: AppState; attention?: string } {
	if (connection.status === 'awaiting_sign_in' || connection.failure === 'sign_in_required')
		/* A connection that never listed tools has never been signed in. */
		return { state: 'needs_you', attention: connection.tools.length ? 'Sign in again' : 'Sign in' };
	if (connection.status === 'failed') return { state: 'needs_you', attention: 'Not answering' };
	if (connection.status === 'awaiting_probe') return { state: 'needs_you', attention: 'Checking' };
	if (connection.status === 'working' && !connection.grants.length)
		return { state: 'needs_you', attention: 'Choose what it may do' };
	if (connection.changed.length) return { state: 'needs_you', attention: 'Changed upstream' };
	if (connection.frozen) return { state: 'paused', attention: 'Frozen' };
	return { state: 'in_use' };
}

function skillState(skill: SkillRow): { state: AppState; attention?: string } {
	if (skill.disposition === 'candidate') return { state: 'needs_you', attention: 'Review' };
	return { state: 'in_use' };
}

const RANK: Record<AppState, number> = { needs_you: 0, paused: 1, in_use: 2, available: 3 };

/** Know-how that ships with Restless rather than being added by the company. */
export function builtIn(app: App): boolean {
	return (
		!app.connections.length &&
		app.skills.length > 0 &&
		app.skills.every((skill) => skill.source === 'builtin')
	);
}

/** Everything the company has, grouped into apps, plus the catalogue it does
 * not. `requested` catalogue keys are Exec's open requests; they are offered
 * there, not again in Browse. */
export function buildApps(
	connections: ToolConnection[],
	library: SkillLibrary | null,
	requested: string[] = []
): { mine: App[]; browse: App[] } {
	const live = connections.filter((connection) => connection.status !== 'disconnected');
	const skills = (library?.skills ?? []).filter((skill) => skill.disposition !== 'retired');
	const plugins = new Map<string, App>();
	const mine: App[] = [];

	const pluginApp = (url: string): App => {
		let app = plugins.get(url);
		if (!app) {
			const name = skillName(
				url
					.replace(/\.git$/, '')
					.split('/')
					.pop() ?? 'Plugin'
			);
			app = {
				key: `p-${encodeURIComponent(url)}`,
				name,
				description: 'A bundle of connections and know-how.',
				how: 'Sign-in and know-how',
				howTip: `A plugin from ${url}: its MCP servers became connections and its skills became company know-how.`,
				category: 'Know-how',
				state: 'in_use',
				connections: [],
				skills: [],
				plugin: url
			};
			plugins.set(url, app);
		}
		return app;
	};

	for (const connection of live) {
		const origin = pluginOrigin(connection.source);
		if (origin) {
			pluginApp(origin).connections.push(connection);
			continue;
		}
		const entry = catalogueFor(connection);
		const { state, attention } = connectionState(connection);
		mine.push({
			key: `c-${connection.name}`,
			name: entry?.name ?? skillName(connection.server_name ?? connection.name),
			description:
				entry?.description ??
				(connection.tools.length
					? `${connection.tools.length} tools from ${connection.server_name ?? connection.name}.`
					: 'A connected service.'),
			how:
				entry?.how ?? (connection.kind === 'local' ? 'Runs on your plane' : 'Connected by address'),
			howTip:
				entry?.howTip ??
				(connection.kind === 'local'
					? `A local MCP server (${connection.command ?? 'command'}), run on your plane outside the company computer.`
					: `An MCP server at ${connection.endpoint ?? 'its address'}.`),
			category: entry?.category ?? 'Engineering',
			state,
			attention,
			connections: [connection],
			skills: [],
			catalogue: entry
		});
	}

	for (const skill of skills) {
		const origin = skill.origin_url ? skill.origin_url.split('#')[0] : undefined;
		const bundle = origin ? plugins.get(origin) : undefined;
		if (bundle) {
			bundle.skills.push(skill);
			continue;
		}
		const { state, attention } = skillState(skill);
		mine.push({
			key: `s-${skill.name}`,
			name: skillName(skill.name),
			description: skill.description || 'Know-how the company follows.',
			how: 'Know-how, nothing to sign in to',
			howTip:
				skill.source === 'builtin'
					? 'A skill that ships with Restless: instructions the company follows. It grants no access.'
					: 'A skill: instructions (and sometimes scripts) the company follows. It grants no credential, spending or approval.',
			category: 'Know-how',
			state,
			attention,
			connections: [],
			skills: [skill]
		});
	}

	for (const app of plugins.values()) {
		const parts = [...app.connections.map(connectionState), ...app.skills.map(skillState)];
		const worst = parts.sort((a, b) => RANK[a.state] - RANK[b.state])[0];
		app.state = worst?.state ?? 'in_use';
		app.attention = worst?.attention;
		mine.push(app);
	}

	mine.sort((a, b) => RANK[a.state] - RANK[b.state] || a.name.localeCompare(b.name));

	const owned = new Set([
		...mine.flatMap((app) => (app.catalogue ? [app.catalogue.key] : [])),
		...requested
	]);
	const browse: App[] = CATALOGUE.filter((entry) => !owned.has(entry.key)).map((entry) => ({
		key: entry.key,
		name: entry.name,
		description: entry.description,
		how: entry.how,
		howTip: entry.howTip,
		category: entry.category,
		state: 'available',
		connections: [],
		skills: [],
		catalogue: entry
	}));
	return { mine, browse };
}

/** Whether anything in Apps needs the owner: the tab's dot. */
export function appsNeedingOwner(apps: App[]): number {
	return apps.filter((app) => app.state === 'needs_you').length;
}

/** Two letters for an app's tile, from its name. */
export function monogram(name: string): string {
	const words = name
		.replace(/[^A-Za-z0-9 ]+/g, ' ')
		.trim()
		.split(/\s+/);
	return (words.length > 1 ? words[0][0] + words[1][0] : name.slice(0, 2)).replace(/^./, (c) =>
		c.toUpperCase()
	);
}

/** What a link the owner pastes most likely is. The server still decides:
 * a Git repository is imported as a plugin (which may hold only skills), any
 * other https address is probed as an MCP server, and anything else is a
 * command run on the plane. */
export type LinkKind = 'plugin' | 'remote' | 'command';

export function classifyLink(value: string): LinkKind {
	const text = value.trim();
	if (/^https?:\/\//i.test(text)) {
		let url: URL;
		try {
			url = new URL(text);
		} catch {
			return 'remote';
		}
		const gitHosts = ['github.com', 'gitlab.com', 'bitbucket.org', 'codeberg.org'];
		if (url.pathname.endsWith('.git') || gitHosts.includes(url.hostname.replace(/^www\./, '')))
			return 'plugin';
		return 'remote';
	}
	return 'command';
}

/** A connection name from a link: the service host or the command's package. */
export function nameForLink(value: string): string {
	const text = value.trim();
	let base = text;
	try {
		const url = new URL(text);
		base = url.hostname.replace(/^(www|mcp|api)\./, '').split('.')[0];
	} catch {
		base =
			text
				.split(/\s+/)
				.find(
					(part) =>
						!part.startsWith('-') && !['npx', 'uvx', 'node', 'python', 'python3'].includes(part)
				) ?? text;
	}
	return (
		base
			.toLowerCase()
			.replace(/^@/, '')
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '')
			.slice(0, 40) || 'app'
	);
}
