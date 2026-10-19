/* A company's model fixed inside its Inbox card (S101): out of credit, at a usage limit, close to
 * one, or unable to start. Switch to another account you hold, or add it as a fallback that takes
 * over by itself next time. */
import { grantAccountConnection } from './account-connections';
import { MODEL_PRESETS } from './model-presets';
import { readProvider, readReusableConnections, saveModelFailover, type ProviderStatus, type ReusableConnection } from './provider';

/** The model a company runs on an account by default: what that account is for. */
export function suggestedModel(provider: string): string | null {
	if (provider === 'openai-codex') return 'openai-codex/gpt-5.6-sol';
	const preset = MODEL_PRESETS.find((entry) => entry.id === provider);
	return preset?.models[0] ? `${provider}/${preset.models[0].id}` : null;
}

export type ModelOption = { connection: ReusableConnection; model: string; inUse: boolean };

/** The accounts this company could think with instead, best first: not the one that ran out,
 * signed in or keyed, with a model to suggest. */
export function alternatives(status: ProviderStatus, connections: ReusableConnection[], company: string): ModelOption[] {
	const current = status.primary_model?.split('/')[0] ?? status.primary_provider;
	const fallbacks = status.model_failover ?? [];
	return connections
		.filter((connection) => connection.provider !== current && connection.status !== 'invalid')
		.map((connection) => ({ connection, model: suggestedModel(connection.provider) ?? '' }))
		.filter((option) => option.model)
		.map((option) => ({
			...option,
			inUse: fallbacks.includes(option.model) || option.connection.companies.some((use) => use.id === company && use.in_use)
		}));
}

export async function loadModelChoices(company: string) {
	const [status, reusable] = await Promise.all([readProvider(company), readReusableConnections()]);
	return { status, options: alternatives(status, reusable.connections, company) };
}

/** Run the company on another account from now on. */
export async function switchModel(company: string, option: ModelOption, revision: string) {
	return grantAccountConnection(company, option.connection.id, {
		model: option.model,
		make_default: true,
		revision,
		replace_existing: true
	});
}

/** Keep the current model, and fall back to this account by itself when it cannot answer. */
export async function addFallback(company: string, option: ModelOption, status: ProviderStatus) {
	let revision = status.revision;
	if (!option.connection.companies.some((use) => use.id === company)) {
		const granted = await grantAccountConnection(company, option.connection.id, {
			model: option.model,
			make_default: false,
			revision
		});
		revision = granted.provider?.revision ?? (await readProvider(company)).revision;
	}
	const fallbacks = [...(status.model_failover ?? []).filter((model) => model !== option.model), option.model];
	return saveModelFailover(company, fallbacks, revision);
}
