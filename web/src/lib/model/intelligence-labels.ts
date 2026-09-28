/* One way to name an intelligence connection and a model for the owner.
 * Connection ids (`direct:openai`, `harness:codex`, `account:…@<id>`,
 * `account-harness:codex:…@<id>`, `harness:custom:…`) are internal routing
 * keys: they never reach a screen as-is. */

import { MODEL_PRESETS } from './model-presets.ts';

export interface LabelledConnection {
	id: string;
	provider: string;
	kind?: string;
	label?: string | null;
	account_provider?: string | null;
}

const HARNESS_NAMES: Record<string, string> = {
	codex: 'ChatGPT / Codex',
	'claude-agent': 'Claude Code',
	claude: 'Claude Code'
};

function providerName(provider: string): string {
	return (
		HARNESS_NAMES[provider] ??
		MODEL_PRESETS.find((preset) => preset.id === provider)?.name ??
		provider
	);
}

/** The owner-facing name of a connection id, using the listed connection when known. */
export function connectionLabel(
	id: string | null | undefined,
	connections: LabelledConnection[] = []
): string {
	if (!id) return 'Company default';
	const known = connections.find((connection) => connection.id === id);
	if (id.startsWith('account-harness:')) {
		const harness = id.slice('account-harness:'.length).split(':')[0];
		const name = HARNESS_NAMES[harness] ?? harness;
		return known?.label ? `${name} · ${known.label}` : name;
	}
	if (id.startsWith('account:')) {
		const provider = known?.provider ?? id.slice('account:'.length).split('@')[0];
		return known?.label ? `${providerName(provider)} · ${known.label}` : providerName(provider);
	}
	if (id.startsWith('harness:custom:'))
		return known?.provider ?? id.slice('harness:custom:'.length);
	if (id.startsWith('harness:')) return providerName(id.slice('harness:'.length));
	if (id.startsWith('direct:')) return providerName(id.slice('direct:'.length));
	return providerName(id);
}

/** The owner-facing name of a model id such as `openai/gpt-6-sol` or `claude-opus-5-5`. */
export function modelLabel(model: string | null | undefined): string {
	if (!model) return 'Unavailable';
	const bare = model.includes('/') ? model.slice(model.indexOf('/') + 1) : model;
	if (bare.startsWith('native-codex-') || bare.startsWith('native-claude-'))
		return 'Harness default';
	for (const preset of MODEL_PRESETS)
		for (const candidate of preset.models) if (candidate.id === bare) return candidate.name;
	// Fall back to a readable form of the id: words capitalised, the family
	// acronym upper-cased, and version pairs joined ("opus-5-5" -> "Opus 5.5").
	return bare
		.replace(/(\d+)-(\d+)(?=$|-)/g, '$1.$2')
		.split('-')
		.map((word) =>
			/^(gpt|glm|oss)$/i.test(word)
				? word.toUpperCase()
				: word.charAt(0).toUpperCase() + word.slice(1)
		)
		.join(' ')
		.replace(/^GPT (\d)/, 'GPT-$1');
}

export function effortLabel(effort: string | null | undefined): string {
	if (!effort) return 'Unavailable';
	if (effort === 'default') return 'Harness default';
	return effort.charAt(0).toUpperCase() + effort.slice(1);
}
