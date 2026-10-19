/* Answering a request for an app or an account inside its Inbox card (S101): reuse an account the
 * owner already holds, paste a token, or sign in, and the waiting Work resumes. Nothing here sends
 * the owner to another page; a provider's own sign-in opens in a small window the card watches. */
import { CATALOGUE, nameForLink } from './apps';
import { addAndAllow, fetchConnections, grantTools, signIn, type ToolConnection } from './connections';
import { DEFAULT_ABILITIES, type Abilities, type Account } from './owner-accounts';
import { ownerAccounts } from './owner-accounts-client';
import type { AppRequest } from './app-requests';
import { domainOf, fullUrl } from './inline-credentials-view';

export {
	domainOf,
	fullUrl,
	matchingAccounts,
	missingFor,
	popupFeatures,
	tokenHelp
} from './inline-credentials-view';

/** Give the company an account the owner already holds. The grant answers the request. */
export async function grantHeldAccount(company: string, account: Account) {
	return ownerAccounts.grant(account.id, { company, abilities: DEFAULT_ABILITIES });
}

/** A token pasted in the card: GitHub becomes an owner-level GitHub account (reusable by every
 * company); any other token app is added to this company with the token kept in the vault. */
export async function addTokenApp(company: string, request: AppRequest, token: string) {
	const secret = token.trim();
	if (!secret) throw new Error('Paste the token first.');
	if (request.catalogueKey === 'github') {
		const account = await ownerAccounts.create({
			kind: 'github',
			label: 'GitHub',
			provider: 'github.com',
			personal: true,
			credential: { kind: 'token', secret }
		});
		await grantHeldAccount(company, account);
		return { label: account.label };
	}
	const entry = CATALOGUE.find((candidate) => candidate.key === request.catalogueKey);
	const name = entry ? entry.key : nameForLink(request.app);
	const { connection } = await addAndAllow(company, {
		kind: 'remote',
		name,
		endpoint: entry?.endpoint ?? request.app,
		auth: { type: 'bearer', credential: secret },
		...(entry ? { source: `catalogue:${entry.key}` } : {})
	});
	return { label: entry?.name ?? connection.name };
}

/** An app that signs in with its provider: add it, open the provider's page in a small window,
 * and finish here once the window reports back (the card polls the connection). */
export async function startAppSignIn(
	company: string,
	request: AppRequest
): Promise<{ connection: ToolConnection; url: string | null }> {
	const entry = CATALOGUE.find((candidate) => candidate.key === request.catalogueKey);
	const name = entry ? entry.key : nameForLink(request.app);
	const existing = (await fetchConnections(company)).find(
		(connection) => connection.name === name && connection.status !== 'disconnected'
	);
	const connection =
		existing ??
		(
			await addAndAllow(company, {
				kind: 'remote',
				name,
				endpoint: entry?.endpoint ?? request.app,
				...(entry ? { source: `catalogue:${entry.key}` } : {})
			})
		).connection;
	if (connection.status !== 'awaiting_sign_in') return { connection, url: null };
	return { connection, url: await signIn(company, connection.name) };
}

/** An account request as its card carries it (S98-08). */
export type AccountAsk = {
	id: string;
	company: string;
	account_id: string | null;
	kind: string | null;
	abilities: Abilities;
	reason: string;
	site?: string | null;
	requested_by_display: string;
};

/** Grant an account for a request, with the abilities asked for, and answer the request. */
export async function grantForRequest(ask: AccountAsk, accountId: string, abilities: Abilities = ask.abilities) {
	const grant = await ownerAccounts.grant(accountId, { company: ask.company, abilities });
	await ownerAccounts.settleRequest(ask.id, { decision: 'approved', grant_id: grant.id });
	return grant;
}

export type NewAccountFields = {
	site: string;
	label: string;
	username: string;
	password: string;
	totp: string;
	secret: string;
	baseUrl: string;
};

/** Add the account a request asks for, from what was typed in the card. Secrets go straight to
 * the vault. A browser sign-in is created here and signed in on the card's live desktop. */
export async function createRequestedAccount(kind: string, fields: NewAccountFields): Promise<Account> {
	const domain = domainOf(fields.site);
	const label = fields.label.trim();
	switch (kind) {
		case 'website_login':
			return ownerAccounts.create({
				kind: 'website_login',
				label: label || domain || 'Website login',
				provider: domain || null,
				credential: {
					kind: 'password',
					sign_in_url: fullUrl(fields.site),
					domains: [domain].filter(Boolean),
					username: fields.username.trim(),
					password: fields.password,
					...(fields.totp.trim() ? { totp_secret: fields.totp.trim() } : {})
				}
			});
		case 'browser_profile':
			return ownerAccounts.create({
				kind: 'browser_profile',
				label: label || domain || 'Browser sign-in',
				provider: domain || null,
				credential: { kind: 'browser_profile', url: fullUrl(fields.site) }
			});
		case 'github':
			return ownerAccounts.create({
				kind: 'github',
				label: label || 'GitHub',
				provider: 'github.com',
				personal: true,
				credential: { kind: 'token', secret: fields.secret.trim() }
			});
		case 'api':
			return ownerAccounts.create({
				kind: 'api',
				label: label || domainOf(fields.baseUrl) || 'API key',
				provider: domainOf(fields.baseUrl) || null,
				credential: { kind: 'api_key', base_url: fullUrl(fields.baseUrl), auth: 'bearer', secret: fields.secret }
			});
		default:
			throw new Error('This kind of account is added in Settings › Accounts.');
	}
}

/** Once the sign-in window has done its work: allow what Exec proposed, so the Work resumes. */
export async function finishAppSignIn(company: string, name: string): Promise<boolean> {
	const view = (await fetchConnections(company)).find((row) => row.name === name);
	if (!view || view.status !== 'working') return false;
	if (view.proposed.length)
		await grantTools(
			company,
			view.name,
			view.proposed.map(({ tool, class: kind, party_args }) => ({ tool, class: kind, party_args }))
		);
	return true;
}
