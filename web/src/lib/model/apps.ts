/* Apps (Sprint 63): one owner view of everything that gives the company an
 * ability. An App is a projection, not a record: it joins the Authority
 * connections (ADR 0014), the company skill library (Sprint 55), plugin origin
 * (a connection's `source`, a skill's `origin_url`) and Exec's pending app
 * requests. Each underlying record keeps its one owner and its own rules.
 *
 * Owner language first: what an app lets the company do. Its kind (a service,
 * a local tool, know-how or a bundle) organises the power-user views, and the
 * mechanism behind it (MCP, CLI, skill, plugin) is explained on hover. */

import type { SkillAssignmentRow, SkillLibrary, SkillRow } from './skills';
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

/** Services owners ask for that have no address in the catalogue, said honestly. None of them
 * offered a hosted MCP address that passed the probe above (Sprint 63), so none is pretended
 * here. Email as the company already has a real path, its email permissions; for the rest, Exec
 * looks for a working server and brings it back to turn on. */
export type UnlistedService = {
	key: string;
	name: string;
	/** Words an owner types for it. */
	terms: string[];
	/** Why it is not one click, in owner words. */
	note: string;
	/** `email`: the company's own email permissions answer this ask. */
	path: 'email' | 'exec';
};

export const UNLISTED: UnlistedService[] = [
	{
		key: 'email',
		name: 'Email as the company',
		terms: ['email', 'e-mail', 'mail', 'inbox', 'gmail', 'outlook', 'newsletter'],
		note: 'Exec sends email as the company within permissions you set: who it may write to, why, and how many a day.',
		path: 'email'
	},
	{
		key: 'google-workspace',
		name: 'Google Workspace',
		terms: ['google', 'gmail', 'calendar', 'drive', 'docs', 'sheets', 'workspace'],
		note: 'Google offers no hosted address we could check yet. Exec can look for a working one and bring it here.',
		path: 'exec'
	},
	{
		key: 'microsoft-365',
		name: 'Microsoft 365',
		terms: [
			'microsoft',
			'outlook',
			'office',
			'teams',
			'onedrive',
			'sharepoint',
			'calendar',
			'excel'
		],
		note: 'Microsoft offers no hosted address we could check yet. Exec can look for a working one and bring it here.',
		path: 'exec'
	},
	{
		key: 'quickbooks',
		name: 'QuickBooks',
		terms: ['quickbooks', 'intuit', 'accounting', 'bookkeeping', 'bookkeeper'],
		note: 'Not checked yet. Xero, Stripe and PayPal are ready to add; Exec can look for a working QuickBooks server.',
		path: 'exec'
	}
];

/** The unlisted services a typed ask names, by whole word. */
export function unlistedFor(text: string): UnlistedService[] {
	const words = new Set(
		text
			.toLowerCase()
			.split(/[^a-z0-9-]+/)
			.filter(Boolean)
	);
	return UNLISTED.filter((service) => service.terms.some((term) => words.has(term)));
}

export type AppState = 'in_use' | 'needs_you' | 'paused' | 'available';

/** What sort of ability an app is. Each grants differently, so experts can tell them apart. */
export type AppKind = 'service' | 'local' | 'skill' | 'bundle';

export const APP_KINDS: AppKind[] = ['service', 'local', 'skill', 'bundle'];

export const KIND_LABEL: Record<AppKind, string> = {
	service: 'Service',
	local: 'Local tool',
	skill: 'Know-how',
	bundle: 'Bundle'
};

export const KIND_SECTION: Record<AppKind, string> = {
	service: 'Services',
	local: 'Local tools',
	skill: 'Know-how',
	bundle: 'Bundles'
};

/** Where a local tool really runs: a fresh worker built from the company computer's image. */
export const LOCAL_INFO =
	'A program, such as a command-line (CLI) tool or a local MCP server, run in its own sandbox built from the company computer’s image, with its own data folder.';

/** One sentence each, on hover: what it is, and what it can and cannot do. */
export const KIND_INFO: Record<AppKind, string> = {
	service:
		'An online service the company reaches through its MCP server. You sign in once, then choose what the company may do with it.',
	local: LOCAL_INFO,
	skill:
		'Instructions (and sometimes scripts) the company follows, known as a skill. It grants no access, spending or sign-in.',
	bundle: 'A plugin: services, tools and know-how that came together from one source.'
};

export type App = {
	/** Stable route key: `c-<connection>`, `s-<skill>`, `p-<plugin>` or a catalogue key. */
	key: string;
	kind: AppKind;
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
	/* Turned off on purpose: signed in, nothing allowed until it is turned back on. */
	if (connection.status === 'working' && !connection.grants.length && connection.ended_grant)
		return { state: 'paused', attention: 'Off' };
	if (connection.status === 'working' && !connection.grants.length)
		return { state: 'needs_you', attention: 'Choose what it may do' };
	if (connection.changed.length) return { state: 'needs_you', attention: 'Changed upstream' };
	if (connection.frozen) return { state: 'paused', attention: 'Frozen' };
	return { state: 'in_use' };
}

/** Know-how turned off for everyone reads as off, or as given to the few it was given to. */
function skillState(
	skill: SkillRow,
	assignments: SkillAssignmentRow[] = []
): { state: AppState; attention?: string } {
	if (skill.disposition === 'candidate') return { state: 'needs_you', attention: 'Review' };
	const mine = assignments.filter((row) => row.skill_name === skill.name);
	if (mine.some((row) => row.scope === 'company' && !row.enabled)) {
		const given = mine.filter((row) => row.scope !== 'company' && row.enabled).length;
		return { state: 'paused', attention: given ? `Given to ${given}` : 'Off' };
	}
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

function skillApp(skill: SkillRow, assignments: SkillAssignmentRow[] = []): App {
	return {
		key: `s-${skill.name}`,
		kind: 'skill',
		name: skillName(skill.name),
		description: skill.description || 'Know-how the company follows.',
		how: 'Know-how, nothing to sign in to',
		howTip:
			skill.source === 'builtin'
				? 'Know-how that comes with Restless (a skill): instructions the company follows. It grants no access.'
				: 'Know-how (a skill): instructions, and sometimes scripts, the company follows. It grants no credential, spending or approval.',
		category: 'Know-how',
		...skillState(skill, assignments),
		connections: [],
		skills: [skill]
	};
}

/** Everything the company has, grouped into apps, plus the catalogue it does
 * not, and know-how the owner removed (kept, to restore). `requested` catalogue
 * keys are Exec's open requests; they are offered there, not again in Browse. */
export function buildApps(
	connections: ToolConnection[],
	library: SkillLibrary | null,
	requested: string[] = []
): { mine: App[]; browse: App[]; removed: App[] } {
	const live = connections.filter((connection) => connection.status !== 'disconnected');
	const skills = (library?.skills ?? []).filter((skill) => skill.disposition !== 'retired');
	const retired = (library?.skills ?? []).filter((skill) => skill.disposition === 'retired');
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
				kind: 'bundle',
				name,
				description: 'A bundle of services and know-how.',
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
			kind: connection.kind === 'local' ? 'local' : 'service',
			name: entry?.name ?? skillName(connection.server_name ?? connection.name),
			description:
				entry?.description ??
				(connection.tools.length
					? `${connection.tools.length} tools from ${connection.server_name ?? connection.name}.`
					: 'A connected service.'),
			how:
				entry?.how ??
				(connection.kind === 'local' ? 'Runs in its own sandbox' : 'Connected by address'),
			howTip:
				entry?.howTip ??
				(connection.kind === 'local'
					? `Runs ${connection.command ?? 'a command'} in its own sandbox built from the company computer’s image, with its own data folder.`
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
		mine.push(skillApp(skill, library?.assignments));
	}

	for (const app of plugins.values()) {
		const parts = [
			...app.connections.map(connectionState),
			...app.skills.map((skill) => skillState(skill, library?.assignments))
		];
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
		kind: 'service' as const,
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
	const removed: App[] = retired.map((skill) => ({
		...skillApp(skill),
		state: 'paused' as const,
		attention: 'Removed'
	}));
	return { mine, browse, removed };
}

/** What the company may do with an app, in a few words, and why on hover. */
export function appLevel(app: App): { label: string; info: string } {
	if (app.kind === 'skill' && !app.connections.length)
		return app.state === 'paused' && app.attention
			? {
					label: app.attention,
					info:
						app.attention === 'Off'
							? 'Off for everyone. No agent uses it until you turn it on or give it to someone.'
							: 'Off for everyone except the people and teams it was given to.'
				}
			: { label: 'Know-how', info: 'Instructions the company follows. It grants no access.' };
	const classes = app.connections.flatMap((connection) =>
		connection.grants.flatMap((grant) => grant.tools.map((tool) => tool.class))
	);
	if (!classes.length)
		return {
			label: 'Not allowed yet',
			info: 'Nothing is granted yet. Open it to choose what the company may do.'
		};
	if (classes.includes('acts'))
		return {
			label: classes.includes('reserved') ? 'Acts, some ask first' : 'Can act',
			info: 'Acts without asking, with a receipt for everything it does. A first contact with someone new still waits for you.'
		};
	if (classes.includes('reserved'))
		return {
			label: 'Asks first',
			info: 'Reads freely. Before it acts, you see the real draft and choose: once, or don’t ask again.'
		};
	return { label: 'Reads only', info: 'Can read, never change anything.' };
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

/** How a catalogued app connects, in two words, so a chooser sees it before adding. */
export function connectTag(app: App): string {
	if (app.catalogue?.auth === 'token') return 'Token';
	if (app.how.startsWith('Sign in')) return 'Sign in';
	return 'No sign-in';
}
