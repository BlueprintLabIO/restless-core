/* Which account pays for a fallback model (M5).
 *
 * A fallback is saved as a provider/model ID, but the gateway bills it to the
 * credential this company holds for that provider: `model.inference.<provider>`,
 * or the main credential when the provider is the main model's. The
 * Intelligence view lists exactly those credentials as `direct` connections, so
 * that list is the billing route the owner sees before saving. Harness
 * connections (Codex, Claude Code) run their own sessions and never serve a
 * gateway fallback. Nothing here assumes the provider names one account: if
 * the view ever lists two direct connections for a provider, the route is
 * reported as ambiguous and cannot be chosen. */

import { connectionLabel, type LabelledConnection } from './intelligence-labels.ts';

export interface FallbackConnection extends LabelledConnection {
	kind?: string;
	account_kind?: 'api_key' | 'oauth';
	included?: boolean;
	loaded?: boolean;
}

export type FallbackRoute<C extends FallbackConnection = FallbackConnection> =
	| { state: 'billed'; connection: C }
	| { state: 'none' }
	| { state: 'ambiguous'; connections: C[] };

export function fallbackProvider(model: string): string {
	const slash = model.indexOf('/');
	return slash > 0 ? model.slice(0, slash) : '';
}

/** The connection that would pay for `model`, read from the company's own route data. */
export function fallbackRoute<C extends FallbackConnection>(
	model: string,
	connections: C[]
): FallbackRoute<C> {
	const provider = fallbackProvider(model);
	const matches = connections.filter(
		(connection) => connection.kind === 'direct' && connection.provider === provider
	);
	if (!matches.length) return { state: 'none' };
	if (matches.length > 1) return { state: 'ambiguous', connections: matches };
	return { state: 'billed', connection: matches[0] };
}

/** Connections a fallback can be added on: the owner's own accounts first, then this
 * company's own keys, then the plan's included AI. One per provider; a provider the
 * view lists twice is left out because its billing account cannot be told. */
export function fallbackAccounts<C extends FallbackConnection>(connections: C[]): C[] {
	const direct = connections.filter((connection) => connection.kind === 'direct');
	const rank = (connection: C) =>
		connection.included ? 2 : connection.id.startsWith('account:') ? 0 : 1;
	return direct
		.filter((connection) => fallbackRoute(`${connection.provider}/x`, direct).state === 'billed')
		.sort((a, b) => rank(a) - rank(b));
}

/** Who pays, in the words the row and the add form both use. */
export function billingSentence(connection: FallbackConnection, all: FallbackConnection[]): string {
	if (connection.included) return 'Uses the included AI in your plan';
	const name = connectionLabel(connection.id, all);
	if (connection.id.startsWith('account:')) return `Bills your ${name} account`;
	return `Bills this company's own ${name} key`;
}

/** The sentence under a saved fallback: who pays, or why it will be skipped. */
export function routeSentence(route: FallbackRoute, all: FallbackConnection[]): string {
	if (route.state === 'billed') return billingSentence(route.connection, all);
	if (route.state === 'ambiguous')
		return 'More than one connection for this provider; it is unclear which would pay';
	return 'Not usable: this company has no working connection for this provider';
}
