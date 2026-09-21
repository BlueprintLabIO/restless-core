// Offline fallback only. Live suggestions come from models.dev via model-catalog.svelte.ts.
// They are choices, not a claim that provider credentials are connected.
export const MODEL_PRESETS = [
	{
		id: 'openai',
		name: 'OpenAI',
		models: [
			{ id: 'gpt-5.6-terra', name: 'GPT-5.6 Terra' },
			{ id: 'gpt-5.6-sol', name: 'GPT-5.6 Sol' }
		]
	},
	{
		id: 'google',
		name: 'Google Gemini',
		models: [
			{ id: 'gemini-3.5-flash', name: 'Gemini 3.5 Flash' },
			{ id: 'gemini-3.1-pro-preview', name: 'Gemini 3.1 Pro Preview' }
		]
	},
	{
		id: 'groq',
		name: 'Groq',
		models: [
			{ id: 'llama-3.3-70b-versatile', name: 'Llama 3.3 70B' },
			{ id: 'openai/gpt-oss-120b', name: 'GPT OSS 120B' }
		]
	},
	{
		id: 'mistral',
		name: 'Mistral',
		models: [
			{ id: 'mistral-medium-latest', name: 'Mistral Medium' },
			{ id: 'codestral-latest', name: 'Codestral' }
		]
	},
	{
		id: 'deepseek',
		name: 'DeepSeek',
		models: [
			{ id: 'deepseek-v4-pro', name: 'DeepSeek V4 Pro' },
			{ id: 'deepseek-v4-flash', name: 'DeepSeek V4 Flash' }
		]
	},
	{
		id: 'openrouter',
		name: 'OpenRouter',
		models: [
			{ id: 'anthropic/claude-sonnet-4.6', name: 'Claude Sonnet 4.6' },
			{ id: 'z-ai/glm-5.3', name: 'GLM-5.3' }
		]
	},
	{
		id: 'xai',
		name: 'xAI',
		models: [
			{ id: 'grok-4.3', name: 'Grok 4.3' },
			{ id: 'grok-code-fast-1', name: 'Grok Code Fast 1' }
		]
	},
	{
		id: 'anthropic',
		name: 'Anthropic',
		models: [
			{ id: 'claude-sonnet-4-6', name: 'Claude Sonnet 4.6' },
			{ id: 'claude-haiku-4-5', name: 'Claude Haiku 4.5' }
		]
	},
	{
		id: 'openai-codex',
		name: 'OpenAI Codex (subscription)',
		models: [{ id: 'gpt-5.6-sol', name: 'GPT-5.6 Sol' }]
	},
	{
		id: 'zai',
		name: 'Z.ai',
		models: [
			{ id: 'glm-5.3', name: 'GLM 5.3' },
			{ id: 'glm-5.3-flash', name: 'GLM 5.3 Flash' }
		]
	},
	{ id: 'moonshot', name: 'Moonshot', models: [{ id: 'kimi-k3', name: 'Kimi K3' }] },
	{
		id: 'litellm',
		name: 'OpenAI-compatible gateway',
		models: [
			{ id: 'gpt-5.6-terra', name: 'GPT-5.6 Terra' },
			{ id: 'gpt-5.6-sol', name: 'GPT-5.6 Sol' },
			{ id: 'gpt-5.6-luna', name: 'GPT-5.6 Luna' }
		]
	}
];
