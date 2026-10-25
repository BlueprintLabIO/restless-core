/* M6: a new company can start on the AI included in the plan while the owner already holds
 * working ChatGPT or Claude sign-ins. Setup offers those first and lets the owner choose; it never
 * grants access or changes who pays by itself. Pure, so the choice is tested without a browser. */
import type { IntelligenceView } from './intelligence.svelte.ts';
import type { ReusableConnection } from './provider.ts';
import { defaultChoice, type ModelChoice } from './model-groups.ts';

/** The connection Exec thinks with, when it is the plan's included AI. */
export function includedRoute(view: IntelligenceView | null, actorId = 'exec') {
	if (!view) return null;
	const agent = view.agents.find((row) => row.id === actorId);
	const selected = agent?.assignment ?? view.default ?? view.base_default ?? null;
	if (!selected) return null;
	const connection = view.connections.find((row) => row.id === selected.connection);
	return connection?.included ? connection : null;
}

/** Account connections that can think right now, sign-ins (ChatGPT, Claude) before stored
 * keys. Providers the included AI serves take no key of the owner's, so they are left out. */
export function usableAccountConnections(
	connections: ReusableConnection[],
	includedProviders: string[] = []
): ReusableConnection[] {
	const rank = (row: ReusableConnection) => (row.kind === 'oauth' ? 0 : 1);
	return connections
		.filter((row) => row.status === 'present' && !includedProviders.includes(row.provider))
		.map((row, index) => ({ row, index }))
		.sort((a, b) => rank(a.row) - rank(b.row) || a.index - b.index)
		.map(({ row }) => row);
}

/** What setup offers before Exec's questions: the owner's own working accounts, when Exec would
 * otherwise carry on with the included AI. Nothing once the owner has chosen, or has started
 * answering, so the interview is never interrupted. */
export function setupAccountOffer(state: {
	view: IntelligenceView | null;
	connections: ReusableConnection[];
	includedProviders: string[];
	chosen: boolean;
	interviewStarted: boolean;
}): ReusableConnection[] {
	if (state.chosen || state.interviewStarted || !includedRoute(state.view)) return [];
	return usableAccountConnections(state.connections, state.includedProviders);
}

/** "ChatGPT and Claude": the accounts on offer, named as the owner knows them. */
export function offerSentence(names: string[]): string {
	const unique = [...new Set(names)];
	if (unique.length <= 1) return unique[0] ?? '';
	return `${unique.slice(0, -1).join(', ')} and ${unique.at(-1)}`;
}

/** The model a connection starts on: the provider's everyday choice, written provider/model. */
export function startingModel(provider: string, models: ModelChoice[]): string {
	const model = defaultChoice(models)?.id;
	if (!model) return '';
	return model.startsWith(`${provider}/`) ? model : `${provider}/${model}`;
}

/** What the owner calls an account: a plan sign-in by its product, a key by its saved name. */
export function accountName(row: ReusableConnection, providerLabels: Record<string, string> = {}) {
	if (row.kind === 'oauth' && row.provider.includes('codex')) return 'ChatGPT';
	if (row.kind === 'oauth' && row.provider === 'anthropic') return 'Claude';
	return row.label?.trim() || providerLabels[row.provider] || row.provider;
}
