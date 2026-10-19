/* The two forms an owner fills in for an account (S98, S101): adding one, and what a company may
 * do with it. Settings › Accounts and an Inbox card asking for an account share these, so a login
 * is added and given the same way wherever it is asked for. Nothing here talks to the network. */
import type { NewAccount } from './owner-accounts-client';
import {
	DEFAULT_ABILITIES,
	EMPTY_LIMITS,
	TIERS,
	limitsSummary,
	type Abilities,
	type AccountKind,
	type Grant,
	type GrantInput,
	type SpendPeriod
} from './owner-accounts.ts';

/** The kinds an owner adds with a form; the rest are set up with their provider. */
export const ADDABLE_KINDS: AccountKind[] = ['website_login', 'api', 'browser_profile', 'github'];

export type AccountForm = {
	label: string;
	url: string;
	username: string;
	password: string;
	totp: string;
	personal: boolean;
	baseUrl: string;
	auth: 'bearer' | 'header' | 'query' | 'basic';
	authName: string;
	secret: string;
};

/** An empty form; a site an agent named fills in where it goes. */
export function blankAccountForm(kind: AccountKind = 'website_login', site = ''): AccountForm {
	return {
		label: '',
		url: kind === 'api' || kind === 'github' ? '' : site,
		username: '',
		password: '',
		totp: '',
		personal: true,
		baseUrl: kind === 'api' ? site : '',
		auth: 'bearer',
		authName: '',
		secret: ''
	};
}

export function domainOf(url: string): string {
	try {
		return new URL(url.includes('://') ? url : `https://${url}`).hostname;
	} catch {
		return '';
	}
}

/** Owners type ebay.com.au/signin; the owner plane wants a full https:// URL. */
export function fullUrl(url: string): string {
	const trimmed = url.trim();
	return !trimmed || /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
}

/** What the form still needs, said when it is sent too early; empty when it is complete. */
export function missingForAccount(kind: AccountKind, form: AccountForm): string {
	switch (kind) {
		case 'website_login':
			return !domainOf(form.url)
				? 'Add the sign-in page.'
				: !form.username.trim()
					? 'Add the username or email.'
					: !form.password
						? 'Add the password.'
						: '';
		case 'api':
			return !domainOf(form.baseUrl)
				? 'Add the service address.'
				: !form.secret
					? 'Add the key or token.'
					: (form.auth === 'header' || form.auth === 'query') && !form.authName.trim()
						? `Add the ${form.auth === 'header' ? 'header' : 'parameter'} name.`
						: '';
		case 'github':
			return form.secret.trim() ? '' : 'Add the token.';
		case 'browser_profile':
			return domainOf(form.url) ? '' : 'Add the website.';
		default:
			return 'This kind of account is set up with its provider.';
	}
}

/** The account the form describes, as the owner plane takes it. Secrets go to the vault. */
export function accountInput(kind: AccountKind, form: AccountForm): NewAccount {
	const label = form.label.trim();
	switch (kind) {
		case 'website_login':
			return {
				kind,
				label: label || domainOf(form.url) || 'Website login',
				provider: domainOf(form.url) || null,
				personal: form.personal,
				credential: {
					kind: 'password',
					sign_in_url: fullUrl(form.url),
					domains: [domainOf(form.url)].filter(Boolean),
					username: form.username.trim(),
					password: form.password,
					...(form.totp.trim() ? { totp_secret: form.totp.trim() } : {})
				}
			};
		case 'api':
			return {
				kind,
				label: label || domainOf(form.baseUrl) || 'API key',
				provider: domainOf(form.baseUrl) || null,
				personal: false,
				credential: {
					kind: 'api_key',
					base_url: fullUrl(form.baseUrl),
					auth: form.auth,
					...(form.auth === 'header' || form.auth === 'query'
						? { auth_name: form.authName.trim() }
						: {}),
					secret: form.secret
				}
			};
		case 'github':
			return {
				kind,
				label: label || 'GitHub',
				provider: 'github.com',
				personal: form.personal,
				credential: { kind: 'token', secret: form.secret.trim() }
			};
		case 'browser_profile':
			return {
				kind,
				label: label || domainOf(form.url) || 'Browser sign-in',
				provider: domainOf(form.url) || null,
				personal: form.personal,
				credential: { kind: 'browser_profile', url: fullUrl(form.url) }
			};
		default:
			throw new Error('This kind of account is set up with its provider.');
	}
}

/** A grant being edited: what a company may do with an account, and its optional limits. */
export type GrantDraft = {
	company: string;
	abilities: Abilities;
	capOn: boolean;
	capDollars: string;
	capPeriod: SpendPeriod;
	scope: string;
	perDay: string;
	expires: string;
	quietOn: boolean;
	quietStart: string;
	quietEnd: string;
	everyone: boolean;
	actors: string;
};

/** A draft from a saved grant, or a new one for a company with the abilities asked for. */
export function grantDraft(
	grant: Grant | null,
	company = '',
	abilities: Abilities = DEFAULT_ABILITIES
): GrantDraft {
	const cap = grant?.spend_cap ?? null;
	const limits = grant?.limits ?? EMPTY_LIMITS;
	return {
		company: grant?.company ?? company,
		abilities: { ...(grant?.abilities ?? abilities) },
		capOn: !!cap,
		capDollars: cap ? String(cap.amount_micros / 1_000_000) : '50',
		capPeriod: cap?.period ?? 'week',
		scope: limits.scope.join(', '),
		perDay: limits.actions_per_day ? String(limits.actions_per_day) : '',
		expires: limits.expires_at?.slice(0, 10) ?? '',
		quietOn: !!limits.quiet_hours,
		quietStart: limits.quiet_hours?.start ?? '22:00',
		quietEnd: limits.quiet_hours?.end ?? '07:00',
		everyone: !grant || grant.who === 'everyone',
		actors: grant && grant.who !== 'everyone' ? grant.who.actors.join(', ') : ''
	};
}

/** Whether a draft's limits are worth showing open. */
export function draftHasLimits(grant: Grant | null): boolean {
	return limitsSummary(grant?.limits ?? EMPTY_LIMITS, grant?.who ?? 'everyone').length > 0;
}

const list = (value: string) =>
	value
		.split(',')
		.map((item) => item.trim())
		.filter(Boolean);

/** The grant a draft describes, as the owner plane takes it. */
export function grantInput(
	draft: GrantDraft,
	timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone
): Required<Pick<GrantInput, 'abilities' | 'spend_cap' | 'limits' | 'who'>> {
	const dollars = Number.parseFloat(draft.capDollars);
	return {
		abilities: draft.abilities,
		spend_cap:
			draft.capOn && draft.abilities.spend !== 'off' && dollars > 0
				? { amount_micros: Math.round(dollars * 1_000_000), period: draft.capPeriod }
				: null,
		limits: {
			scope: list(draft.scope),
			actions_per_day: Number.parseInt(draft.perDay, 10) || null,
			expires_at: draft.expires ? new Date(`${draft.expires}T23:59:59`).toISOString() : null,
			quiet_hours: draft.quietOn
				? { start: draft.quietStart, end: draft.quietEnd, tz: timeZone }
				: null
		},
		who: draft.everyone ? 'everyone' : { actors: list(draft.actors) }
	};
}

/** What an access level allows, in one line: "Read and Draft freely, Act telling you, asks
 * before Spend, no Admin." */
export function abilitySummary(abilities: Abilities): string {
	const by = (mode: string) =>
		TIERS.filter(({ tier }) => abilities[tier] === mode).map(({ label }) => label);
	const join = (names: string[]) =>
		names.length > 1 ? `${names.slice(0, -1).join(', ')} and ${names.at(-1)}` : names[0];
	const text = [
		by('allow').length ? `${join(by('allow'))} freely` : '',
		by('tell').length ? `${join(by('tell'))}, telling you` : '',
		by('ask').length ? `asks before ${join(by('ask'))}` : '',
		by('off').length ? `no ${join(by('off'))}` : ''
	]
		.filter(Boolean)
		.join(', ');
	return `${text.charAt(0).toUpperCase()}${text.slice(1)}.`;
}
