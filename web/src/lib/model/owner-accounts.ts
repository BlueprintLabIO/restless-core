/* Accounts the owner holds and grants to companies (S98). An account is an identity (a GitHub
 * account, an eBay seller account, a website login, an API); its credentials stay with the owner;
 * a grant says what one company may do with it. These are the records the owner plane serves
 * (S98-01) and the plain words the cockpit shows for them. Nothing here talks to the network. */

export type AccountKind =
	'ai_provider' | 'api' | 'website_login' | 'browser_profile' | 'github' | 'mcp_tool';
export type AccountHealth =
	'signed_in' | 'signed_out' | 'expiring' | 'expired' | 'refused' | 'unknown';
export type CredentialKind =
	'oauth' | 'api_key' | 'password' | 'totp' | 'browser_profile' | 'app_install';
export type Mode = 'allow' | 'tell' | 'ask' | 'off';
export type Tier = 'read' | 'draft' | 'act' | 'spend' | 'admin';
export type Abilities = Record<Tier, Mode>;
export type SpendPeriod = 'day' | 'week' | 'month';

export type Credential = {
	id: string;
	kind: CredentialKind;
	label: string | null;
	pinned: boolean;
	health: AccountHealth;
	updated_at: string;
};
export type GrantLimits = {
	scope: string[];
	actions_per_day: number | null;
	expires_at: string | null;
	quiet_hours: { start: string; end: string; tz: string } | null;
};
export type Grant = {
	id: string;
	account_id: string;
	company: string;
	abilities: Abilities;
	spend_cap: { amount_micros: number; period: SpendPeriod } | null;
	limits: GrantLimits;
	who: 'everyone' | { actors: string[] };
	frozen: boolean;
	created_at: string;
	created_by: string;
};
export type Account = {
	id: string;
	kind: AccountKind;
	label: string;
	provider: string | null;
	identity: { display: string | null; key: string | null };
	personal: boolean;
	health: AccountHealth;
	frozen: boolean;
	created_at: string;
	updated_at: string;
	credentials: Credential[];
	grants: Grant[];
	/** Uses the owner was told about and has not answered (S98-10). */
	open_notices?: number;
};
/** Why the owner is told about a use (S98-10): notices, not blocks. */
export type NoticeKind = 'first_use' | 'burst' | 'first_spend' | 'new_destination';
export type UseNotice = { id: string; kind: NoticeKind; state: 'open' | 'fine' | 'frozen' };
export type UseResult = 'ok' | 'refused' | 'failed' | 'asked';
export type AccountUse = {
	id: number | string;
	account_id: string;
	grant_id: string | null;
	company: string;
	actor: string;
	actor_display?: string | null;
	directed_by: string | null;
	directed_by_display?: string | null;
	action: string;
	tier: Tier;
	target: string | null;
	result: UseResult;
	cost_micros: number | null;
	notice?: UseNotice | null;
	at: string;
};
/** A use as it travels on an Inbox item or a chat message (S98-09): which account, which ability,
 * which company, who did it and at whose request. */
export type AccountUseRef = {
	account: Pick<Account, 'id' | 'label' | 'kind' | 'provider'>;
	grant_id: string | null;
	company: string;
	tier: Tier;
	action: string;
	target: string | null;
	actor_display: string;
	directed_by_display: string | null;
	cost_micros: number | null;
};
export type AccountRequest = {
	id: string;
	company: string;
	account_id: string | null;
	kind: AccountKind;
	abilities: Abilities;
	reason: string;
	/** The site an agent named, to fill in the sign-in page (S101). */
	site?: string | null;
	requested_by_display: string;
};
export type AccountUseReceipt = AccountUseRef & { use_id: number | string; result: UseResult };
export type GrantInput = Partial<
	Pick<Grant, 'abilities' | 'spend_cap' | 'limits' | 'who' | 'frozen'>
> & { company?: string };

/** The kinds an owner adds here. AI providers keep their own section above. */
export const ACCOUNT_KINDS: { kind: AccountKind; label: string; mark: string; hint: string }[] = [
	{
		kind: 'website_login',
		label: 'Website login',
		mark: 'WEB',
		hint: 'A stored username and password. New website sign-ins happen on the company computer.'
	},
	{
		kind: 'api',
		label: 'API key',
		mark: 'API',
		hint: 'A service your companies call with a key or token.'
	},
	{
		kind: 'browser_profile',
		label: 'Website sign-in',
		mark: 'WEB',
		hint: 'Any site you sign in to, like eBay or a supplier portal. You sign in once on the company computer; companies use that session, never a password.'
	},
	{
		kind: 'github',
		label: 'GitHub',
		mark: 'GH',
		hint: 'Repositories, issues and pull requests, with a GitHub token.'
	},
	{
		kind: 'mcp_tool',
		label: 'Tool connection',
		mark: 'MCP',
		hint: 'An app connected through MCP, such as Gmail or Linear.'
	},
	{ kind: 'ai_provider', label: 'AI account', mark: 'AI', hint: 'A model provider.' }
];

/** "an API key", "a website login", "a GitHub account": for a sentence about a kind. */
export function kindPhrase(kind: AccountKind) {
	const label = kindOf(kind).label;
	const words = /[A-Z]/.test(label.slice(1)) ? label : label.toLowerCase();
	return `${/^[aeiou]/i.test(words) ? 'an' : 'a'} ${words}`;
}
export function kindOf(kind: AccountKind) {
	return ACCOUNT_KINDS.find((entry) => entry.kind === kind) ?? ACCOUNT_KINDS[0];
}

export const TIERS: { tier: Tier; label: string; hint: string }[] = [
	{ tier: 'read', label: 'Read', hint: 'Look things up: issues, listings, orders, messages.' },
	{
		tier: 'draft',
		label: 'Draft',
		hint: 'Prepare without sending: a post, a listing, a pull request.'
	},
	{ tier: 'act', label: 'Act', hint: 'Publish, send, list an item, reply to a customer, merge.' },
	{ tier: 'spend', label: 'Spend', hint: 'Buy, pay, run ads, refund.' },
	{ tier: 'admin', label: 'Admin', hint: 'Change the account’s settings, users or password.' }
];

export const MODES: { mode: Mode; label: string }[] = [
	{ mode: 'allow', label: 'Allowed' },
	{ mode: 'tell', label: 'Allowed, tell me' },
	{ mode: 'ask', label: 'Asks me first' },
	{ mode: 'off', label: 'Off' }
];

/** A new grant's abilities: permissive, as the owner chose (S98). */
export const DEFAULT_ABILITIES: Abilities = {
	read: 'allow',
	draft: 'allow',
	act: 'tell',
	spend: 'ask',
	admin: 'off'
};

export const PRESETS: { key: 'standard' | 'full' | 'read'; label: string; abilities: Abilities }[] =
	[
		{ key: 'standard', label: 'Standard', abilities: DEFAULT_ABILITIES },
		{
			key: 'full',
			label: 'Full access',
			abilities: { read: 'allow', draft: 'allow', act: 'allow', spend: 'allow', admin: 'allow' }
		},
		{
			key: 'read',
			label: 'Read only',
			abilities: { read: 'allow', draft: 'off', act: 'off', spend: 'off', admin: 'off' }
		}
	];

/** Which preset a set of abilities is, if any. */
export function presetOf(abilities: Abilities): (typeof PRESETS)[number]['key'] | null {
	return (
		PRESETS.find((preset) => TIERS.every(({ tier }) => preset.abilities[tier] === abilities[tier]))
			?.key ?? null
	);
}

const PERIOD: Record<SpendPeriod, string> = { day: 'a day', week: 'a week', month: 'a month' };

/** Money in the owner's currency from micros: "$50", "$12.50". */
export function money(micros: number): string {
	const dollars = micros / 1_000_000;
	return `$${Number.isInteger(dollars) ? dollars : dollars.toFixed(2)}`;
}

/** A grant in a line: what the company may do, in plain words. */
export function grantSummary(grant: Pick<Grant, 'abilities' | 'spend_cap'>): string {
	const parts: string[] = [];
	for (const { tier, label } of TIERS) {
		const mode = grant.abilities[tier];
		if (mode === 'off') continue;
		if (tier === 'spend' && grant.spend_cap)
			parts.push(
				`Spend up to ${money(grant.spend_cap.amount_micros)} ${PERIOD[grant.spend_cap.period]}${mode === 'allow' || mode === 'tell' ? '' : ', then asks'}`
			);
		else if (mode === 'allow') parts.push(label);
		else if (mode === 'tell') parts.push(`${label}, tells you`);
		else parts.push(`${label} after asking`);
	}
	return parts.length ? parts.join(' · ') : 'No access';
}

/** The optional limits on a grant, in plain words; empty when there are none. */
export function limitsSummary(limits: GrantLimits, who: Grant['who']): string[] {
	const out: string[] = [];
	if (limits.scope.length) out.push(`Only ${limits.scope.join(', ')}`);
	if (limits.actions_per_day) out.push(`Up to ${limits.actions_per_day} actions a day`);
	if (limits.quiet_hours)
		out.push(`Not between ${limits.quiet_hours.start} and ${limits.quiet_hours.end}`);
	if (limits.expires_at) out.push(`Until ${limits.expires_at.slice(0, 10)}`);
	if (who !== 'everyone') out.push(`Only ${who.actors.join(', ')}`);
	return out;
}

export const EMPTY_LIMITS: GrantLimits = {
	scope: [],
	actions_per_day: null,
	expires_at: null,
	quiet_hours: null
};

export const HEALTH_LABEL: Record<AccountHealth, string> = {
	signed_in: 'Ready',
	signed_out: 'Signed out',
	expiring: 'Expiring',
	expired: 'Expired',
	refused: 'Refused',
	unknown: 'Not checked'
};

/** Shown once, quietly, for a personal login or browser sign-in (owner decision, S98). */
export const PERSONAL_NOTICE =
	'Use this account within the platform’s own rules. You are responsible for how it is used; Restless is not responsible for misuse.';

export const NOTICE_LABEL: Record<NoticeKind, string> = {
	first_use: 'First use by this company',
	burst: 'Much busier than usual',
	first_spend: 'First spend',
	new_destination: 'Somewhere new'
};

export const RESULT_LABEL: Record<UseResult, string> = {
	ok: 'Done',
	refused: 'Refused',
	failed: 'Failed',
	asked: 'Waiting for you'
};

export const TIER_LABEL = Object.fromEntries(
	TIERS.map(({ tier, label }) => [tier, label])
) as Record<Tier, string>;

/** An action as the engine names it ("listing.create", "listed an item"), in words. */
function actionWords(action: string): string {
	return action.includes(' ') ? action : action.replace(/[._-]+/g, ' ').trim();
}

/** One use as one sentence: "Exec, at Bianca's request, listed “Canon AE-1”, using eBay in
 * Sydney Resale." `account` and `company` are left out where the page already says them. */
export function useSentence(
	use: {
		actor: string;
		actor_display?: string | null;
		directed_by?: string | null;
		directed_by_display?: string | null;
		action: string;
		target: string | null;
	},
	where: { account?: string; company?: string } = {}
): string {
	const actor = use.actor_display || use.actor;
	const directed = use.directed_by_display || use.directed_by;
	let sentence = actor;
	if (directed && directed !== actor) sentence += `, at ${directed}’s request,`;
	sentence += ` ${actionWords(use.action)}`;
	if (use.target) sentence += ` “${use.target}”`;
	if (where.account) sentence += `${where.company ? ',' : ''} using ${where.account}`;
	if (where.company) sentence += ` in ${where.company}`;
	return `${sentence}.`;
}

/** A use reference from an Inbox item or a message, in a line: "eBay · Act · Exec, for Bianca". */
export function useRefLine(ref: AccountUseRef): string {
	const who =
		ref.directed_by_display && ref.directed_by_display !== ref.actor_display
			? `${ref.actor_display}, for ${ref.directed_by_display}`
			: ref.actor_display;
	return [
		ref.account.label,
		TIER_LABEL[ref.tier],
		who,
		...(ref.cost_micros ? [money(ref.cost_micros)] : [])
	].join(' · ');
}
