/* Everything the page states about itself, in one place, so it cannot drift from the README. */

export const GITHUB_URL = 'https://github.com/BlueprintLabIO/restless-core';
export const EXPERIMENT_URL = `${GITHUB_URL}/blob/dev/experiment/coordination/experiments/EXP-17/RESULTS.md`;
export const CONTACT_EMAIL = 'yao@restless.run';

export const CLONE_COMMAND = `git clone --branch dev ${GITHUB_URL}.git`;

export const QUICKSTART = [
	{ label: 'Get the source', command: CLONE_COMMAND },
	{ label: 'Install the workspace', command: 'cd restless-core && npm --prefix web ci' },
	{ label: 'Start a company', command: './scripts/restless-dev demo_test --reconcile' }
] as const;

export const REQUIREMENTS =
	'A Linux host with Rust/Cargo, Node.js 24, Docker with Compose v2, curl, jq and OpenSSL, and about 30 GiB free.';

export const PROVIDERS = [
	'Codex',
	'Claude',
	'OpenAI',
	'Anthropic',
	'Google Gemini',
	'OpenRouter',
	'Groq',
	'Mistral',
	'DeepSeek',
	'xAI',
	'Moonshot',
	'Z.ai',
	'Any OpenAI-compatible gateway'
] as const;

export const FEATURES = [
	{
		mark: 'people',
		title: 'Human multiplayer',
		body: 'Bring your cofounder, designer or reviewer into the same company, with their own accounts and permissions.'
	},
	{
		mark: 'attention',
		title: 'Attention is the budget',
		body: 'A decision arrives prepared: the context, the recommendation, the output and what happens next.'
	},
	{
		mark: 'work',
		title: 'Goals that survive restarts',
		body: 'An agent can switch models, restart or hand work over without losing the goal, who owns it or what comes next.'
	},
	{
		mark: 'executive',
		title: 'A real company computer',
		body: 'A persistent Linux workspace with files, Git, a browser and a desktop you can take over.'
	},
	{
		mark: 'authority',
		title: 'Authority, enforced by code',
		body: 'Choose what agents may do alone and what needs you. Budgets, a vault and records of every external action.'
	},
	{
		mark: 'direction',
		title: 'A company that knows itself',
		body: 'Approved facts, voice, visual language and culture travel with the work, and outdated content is flagged when they change.'
	}
] as const;
