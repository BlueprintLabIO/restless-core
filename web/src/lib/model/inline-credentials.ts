/* Answering a request for an app or an account inside its Inbox card (S101): reuse an account the
 * owner already holds, paste a token, or sign in, and the waiting Work resumes. Nothing here sends
 * the owner to another page; a provider's own sign-in opens in a small window the card watches. */
import { CATALOGUE, nameForLink } from './apps';
import {
	addAndAllow,
	fetchConnections,
	grantTools,
	signIn,
	type ToolConnection
} from './connections';
import type { Abilities } from './owner-accounts';
import type { AppRequest } from './app-requests';

export {
	domainOf,
	fullUrl,
	matchingAccounts,
	popupFeatures,
	tokenHelp
} from './inline-credentials-view';

/** A token app added from its card with the Apps page's own token picker (S101): the token is
 * kept in the company Vault and the app is added with that secret, then allowed what Exec
 * proposed, so the waiting Work resumes. */
export async function addTokenApp(company: string, request: AppRequest, credential: string) {
	const entry = CATALOGUE.find((candidate) => candidate.key === request.catalogueKey);
	const name = entry ? entry.key : nameForLink(request.app);
	const { connection } = await addAndAllow(company, {
		kind: 'remote',
		name,
		endpoint: entry?.endpoint ?? request.app,
		auth: { type: 'bearer', credential },
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
