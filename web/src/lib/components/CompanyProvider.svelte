<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	const catalog = modelCatalog();
	import HarnessConnections from './HarnessConnections.svelte';
	import AgentIntelligence from './AgentIntelligence.svelte';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	let { companyId }: { companyId: string } = $props();
	const intelligence = $derived(intelligenceQuery(companyId));
	let editorOpen = $state(false);
	type Connection = {
		provider: string;
		reference: string | null;
		credential_status: string;
		credential_detail: string | null;
		gateway_loaded: boolean;
	};
	type ProviderStatus = {
		revision: string;
		primary_provider: string;
		connections: Connection[];
		infisical_configured: boolean;
		infisical_status: string;
		infisical_detail: string | null;
		startup_issue: string | null;
	};
	const labels: Record<string, string> = {
		anthropic: 'Anthropic',
		openai: 'OpenAI',
		'openai-codex': 'OpenAI Codex',
		google: 'Google Gemini',
		groq: 'Groq',
		mistral: 'Mistral',
		deepseek: 'DeepSeek',
		openrouter: 'OpenRouter',
		xai: 'xAI',
		zai: 'Z.ai',
		moonshot: 'Moonshot',
		litellm: 'OpenAI-compatible gateway'
	};
	let status = $state<ProviderStatus | null>(null);
	let selected = $state('');
	let mode = $state('infisical');
	let reference = $state('');
	let secret = $state('');
	let error = $state('');
	let notice = $state('');
	let busy = $state(false);
	let refreshing = $state(false);
	let edited = false;
	let editor = $state<HTMLElement>();
	let requestSequence = 0;
	const endpoint = $derived(`/api/companies/${encodeURIComponent(companyId)}/provider`);
	const connection = $derived(status?.connections.find((c) => c.provider === selected));
	const savedCount = $derived(status?.connections.filter((c) => c.reference).length ?? 0);
	function defaultReference(provider: string) {
		return `infisical:/companies/${companyId}/MODEL_${provider.replace(/[^a-zA-Z0-9_]/g, '_')}_API_KEY`;
	}
	function choose(provider: string, reveal = false) {
		selected = provider;
		edited = false;
		secret = '';
		error = '';
		notice = '';
		const row = status?.connections.find((c) => c.provider === provider);
		mode =
			row?.reference?.startsWith('omp-oauth:') || provider === 'openai-codex'
				? 'oauth'
				: row?.reference?.startsWith('env:')
					? 'env'
					: 'infisical';
		if (reveal) editorOpen = true;
		if (reveal) void tick().then(() => editor?.scrollIntoView({ block: 'start' }));
		reference =
			row?.reference ?? (mode === 'oauth' ? `omp-oauth:${provider}` : defaultReference(provider));
	}
	async function refresh() {
		const sequence = ++requestSequence;
		refreshing = true;
		try {
			const response = await fetch(endpoint, { cache: 'no-store' });
			const body = await response.json();
			if (sequence !== requestSequence) return;
			if (!response.ok) throw new Error(body.message ?? 'Could not read connections.');
			status = body;
			if (!selected) {
				choose(body.primary_provider);
				editorOpen = !body.connections.some((c: Connection) => c.reference);
			} else if (!edited && !busy) {
				const previousNotice = notice;
				choose(selected);
				notice = previousNotice;
			}
		} catch (cause) {
			if (sequence === requestSequence)
				error = cause instanceof Error ? cause.message : 'Could not read connections.';
		} finally {
			if (sequence === requestSequence) refreshing = false;
		}
	}
	function selectMode() {
		edited = true;
		secret = '';
		error = '';
		notice = '';
		reference =
			mode === 'oauth'
				? `omp-oauth:${selected}`
				: mode === 'env'
					? 'env:'
					: defaultReference(selected);
	}
	async function connect(event?: SubmitEvent, disconnect = false) {
		event?.preventDefault();
		if (!status || busy || refreshing) return;
		busy = true;
		error = '';
		notice = '';
		++requestSequence;
		try {
			const response = await fetch(endpoint, {
				method: 'PUT',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({
					provider: selected,
					model: catalog.models(selected)[0]?.id,
					reference,
					secret: secret || undefined,
					revision: status.revision,
					disconnect
				})
			});
			secret = '';
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not save this connection.');
			status = body;
			await intelligence.refresh();
			editorOpen = false;
			choose(selected);
			notice = disconnect
				? 'Disconnected. Restart Restless to unload this route.'
				: 'Connection saved. Restart Restless to activate this provider.';
		} catch (cause) {
			secret = '';
			error = cause instanceof Error ? cause.message : 'Could not save this connection.';
		} finally {
			busy = false;
		}
	}
	function stateLabel(row: Connection) {
		if (row.credential_status === 'invalid') return 'Connection error';
		if (row.credential_status !== 'present')
			return row.reference ? 'Credential missing' : 'Not connected';
		return row.gateway_loaded ? 'Route loaded' : 'Credential available';
	}
	function tone(row: Connection) {
		return row.credential_status === 'invalid'
			? 'error'
			: row.credential_status === 'present'
				? 'success'
				: row.reference
					? 'warning'
					: 'neutral';
	}
	onMount(() => {
		void refresh();
		return () => {
			++requestSequence;
			secret = '';
		};
	});
</script>

<svelte:head><title>Intelligence provider — Company</title></svelte:head>
<div class="provider-page">
	<header>
		<h1>Intelligence provider</h1>
		<button
			class="primary"
			disabled={!status || busy}
			onclick={() => {
				choose('openai');
				editorOpen = true;
			}}>+ Add connection</button
		>
	</header>
	<div class="storage">
		<span
			class:good={status?.infisical_status === 'present'}
			title={status?.infisical_detail ??
				'API keys are stored securely in Infisical; only references are saved in company settings.'}
			>● {status?.infisical_status === 'present'
				? 'Secure key storage ready'
				: status
					? 'Key storage unavailable'
					: 'Checking key storage…'}</span
		><a href={`/${companyId}/company/vault`}>Vault →</a><button
			class="text-button"
			disabled={busy || refreshing}
			onclick={() => refresh()}>{refreshing ? 'Checking…' : 'Refresh'}</button
		>
	</div>
	{#if status}
		{#if savedCount}
			<section aria-label="Your connections" class="connections">
				{#each status.connections.filter((c) => c.reference) as row (row.provider)}
					<div class="connection-row">
						<strong>{labels[row.provider] ?? row.provider}</strong><span class="badge {tone(row)}"
							>● {stateLabel(row)}</span
						><button
							disabled={busy}
							onclick={() => choose(row.provider, true)}
							aria-label={`Edit ${labels[row.provider] ?? row.provider}`}>Edit</button
						>
					</div>
				{/each}
			</section>
		{:else}<div class="empty">
				<h2>Add your first connection</h2>
				<p>Choose a provider and paste its API key to get started.</p>
			</div>{/if}
	{/if}
	{#if editorOpen && status}
		<section class="connection-editor" bind:this={editor} aria-label="Connection settings">
			<form class="credentials" onsubmit={(e) => connect(e)}>
				<label for="provider-choice">Provider</label><select
					id="provider-choice"
					disabled={busy}
					value={labels[selected] ? selected : 'custom'}
					onchange={(e) => choose(e.currentTarget.value)}
				>
					{#each Object.entries(labels).filter(([id]) => id !== 'openai-codex' || status?.connections.some((c) => c.provider === id && c.reference)) as [id, label]}<option
							value={id}>{label}</option
						>{/each}<option value="custom">Custom provider…</option>
				</select>
				{#if !labels[selected]}<label for="custom-provider">Provider ID</label><input
						id="custom-provider"
						placeholder="Provider ID"
						value={selected === 'custom' ? '' : selected}
						oninput={(e) => choose(e.currentTarget.value)}
						pattern="[a-zA-Z0-9_.\-]+"
						required
						disabled={busy}
					/>{/if}
				{#if mode === 'infisical'}<label for="provider-key">API key</label><input
						id="provider-key"
						type="password"
						bind:value={secret}
						oninput={() => (edited = true)}
						autocomplete="new-password"
						placeholder={connection?.reference
							? 'Paste a replacement key, or leave blank to keep it'
							: 'Paste your API key'}
						disabled={busy || status.infisical_status !== 'present'}
						required={!connection?.reference}
					/>{/if}
				{#if connection?.credential_detail}<p role="alert">{connection.credential_detail}</p>{/if}
				<details>
					<summary>Advanced settings</summary><label for="provider-auth">Credential source</label
					><select id="provider-auth" bind:value={mode} onchange={selectMode} disabled={busy}
						><option value="infisical">API key stored securely</option><option value="env"
							>Existing host environment variable</option
						><option value="oauth">Existing host broker login</option></select
					>
					<label for="provider-reference">Credential reference</label><input
						id="provider-reference"
						bind:value={reference}
						oninput={() => (edited = true)}
						disabled={busy}
						required
						spellcheck="false"
					/>
					{#if mode === 'oauth'}<p>
							This uses an existing host broker login. For native Codex or Claude sign-in, open <a
								href="#harnesses">Harness connections</a
							>.
						</p>{/if}
				</details>
				<div class="actions">
					<button
						class="primary"
						disabled={busy ||
							refreshing ||
							!selected ||
							selected === 'custom' ||
							(mode === 'infisical' && status.infisical_status !== 'present')}
						>{busy ? 'Saving…' : connection?.reference ? 'Save connection' : 'Connect'}</button
					><button
						type="button"
						disabled={busy}
						onclick={() => {
							editorOpen = false;
							secret = '';
						}}>Cancel</button
					>{#if connection?.reference}<button
							type="button"
							class="text-button danger"
							disabled={busy}
							onclick={() => connect(undefined, true)}>Disconnect</button
						>{/if}
				</div>
			</form>
		</section>
	{/if}
	{#if error}<p role="alert">{error}</p>{/if}{#if notice}<p class="notice" role="status">
			{notice}
		</p>{/if}
	<div
		class="catalog-status"
		title="Model suggestions refresh hourly from models.dev. Catalog inclusion does not confirm account access or harness compatibility. Local gateways keep bundled suggestions and custom IDs. Saved model choices are never changed automatically."
	>
		<span
			>{catalog.pending
				? 'Refreshing model catalog…'
				: catalog.failed
					? catalog.updatedAt
						? 'Using cached model catalog'
						: 'Using bundled model suggestions'
					: catalog.updatedAt
						? 'Model catalog updated ' + new Date(catalog.updatedAt).toLocaleString()
						: 'Bundled model suggestions'}</span
		>
		<button class="text-button" disabled={catalog.pending} onclick={() => catalog.refresh()}
			>Refresh models</button
		>
	</div>
	<section id="harnesses" class="harness-section">
		<h2 title="Native sign-in is independent of direct API connections.">
			Subscription connections
		</h2>
		<HarnessConnections {companyId} />
	</section>
	<AgentIntelligence {companyId} />
</div>

<style>
	.provider-page {
		width: 100%;
		min-width: 0;
		max-width: 880px;
		margin: 0 auto;
		padding: var(--space-6);
		overflow-y: auto;
		box-sizing: border-box;
	}
	header,
	.storage,
	.actions,
	.connection-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.catalog-status {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		font-size: var(--t-label);
		color: var(--text-tertiary);
		margin-block: var(--space-4);
	}
	header,
	.storage {
		justify-content: space-between;
		flex-wrap: wrap;
	}
	h1 {
		font-size: var(--t-title);
		margin: 0;
	}
	h2 {
		font-size: var(--t-head);
		margin: 0;
	}
	.storage {
		margin-block: var(--space-4) var(--space-5);
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.good {
		color: var(--state-success);
	}
	button,
	input,
	select {
		font: inherit;
		color: var(--ink);
		background: var(--surface-pane);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		min-height: 40px;
		padding: var(--space-2) var(--space-3);
		box-sizing: border-box;
	}
	button {
		cursor: pointer;
		flex: none;
	}
	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.primary {
		background: var(--ink);
		color: var(--text-inverse);
	}
	.text-button {
		border: 0;
		background: transparent;
		padding-inline: var(--space-1);
	}
	.connection-row {
		padding-block: var(--space-3);
		border-bottom: 1px solid var(--control-edge);
		flex-wrap: wrap;
	}
	.connection-row strong {
		flex: 1;
	}
	.badge {
		font-size: var(--t-label);
	}
	.success {
		color: var(--state-success);
	}
	.warning {
		color: var(--company-amber);
	}
	.error,
	.danger,
	[role='alert'] {
		color: var(--state-danger);
	}
	.neutral {
		color: var(--text-tertiary);
	}
	.empty {
		margin-block: var(--space-5);
	}
	p {
		color: var(--text-tertiary);
		line-height: 1.5;
	}
	.connection-editor {
		background: var(--surface-pane);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-pane);
		padding: var(--space-5);
		margin-block: var(--space-4);
	}
	.credentials {
		display: grid;
		gap: var(--space-2);
	}
	input,
	select {
		width: 100%;
		min-width: 0;
	}
	label {
		margin-top: var(--space-2);
	}
	details {
		margin-block: var(--space-3);
	}
	details label {
		display: block;
		margin-bottom: var(--space-2);
	}
	summary {
		cursor: pointer;
		color: var(--text-secondary);
	}
	.actions {
		flex-wrap: wrap;
		margin-top: var(--space-3);
	}
	.harness-section {
		margin-top: var(--space-6);
		padding-top: var(--space-5);
		border-top: 1px solid var(--control-edge);
	}
	.provider-page {
		min-height: 0;
		overflow-wrap: anywhere;
	}
	.provider-page :is(button, select, input) {
		max-width: 100%;
	}
	.notice {
		padding: var(--space-3);
		background: var(--surface-alt);
		border-radius: var(--radius-control);
	}
	@container company-canvas (max-width: 640px) {
		.provider-page {
			padding: var(--space-4);
		}
		.connection-editor {
			padding: var(--space-4);
		}
	}
</style>
