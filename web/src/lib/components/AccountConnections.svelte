<script lang="ts">
	import { onMount } from 'svelte';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	type CompanyUse = { id: string; name: string; in_use?: boolean };
	type AccountConnection = {
		id: string;
		label: string;
		provider: string;
		status?: 'present' | 'absent' | 'invalid';
		detail?: string | null;
		companies: CompanyUse[];
	};
	const providerOptions = [
		['anthropic', 'Anthropic'], ['openai', 'OpenAI'], ['google', 'Google Gemini'], ['groq', 'Groq'],
		['mistral', 'Mistral'], ['deepseek', 'DeepSeek'], ['openrouter', 'OpenRouter'], ['xai', 'xAI'],
		['zai', 'Z.ai'], ['moonshot', 'Moonshot'], ['litellm', 'OpenAI-compatible gateway']
	];
	let connections = $state<AccountConnection[]>([]);
	let loading = $state(true);
	let error = $state('');
	let addOpen = $state(false);
	let busy = $state(false);
	let label = $state('');
	let provider = $state('anthropic');
	let secret = $state('');

	async function refresh() {
		loading = true;
		error = '';
		try {
			const response = await fetch('/api/connections', { cache: 'no-store' });
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not load account connections.');
			connections = body.connections ?? [];
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not load account connections.';
		} finally { loading = false; }
	}
	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		busy = true;
		error = '';
		try {
			const response = await fetch('/api/connections', {
				method: 'POST', headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ label: label.trim(), provider, secret })
			});
			const body = await response.json();
			if (!response.ok) throw new Error(body.message ?? 'Could not save this connection.');
			label = '';
			secret = '';
			addOpen = false;
			await refresh();
		} catch (cause) {
			secret = '';
			error = cause instanceof Error ? cause.message : 'Could not save this connection.';
		} finally { busy = false; }
	}
	function statusText(item: AccountConnection) {
		if (item.status === 'present') return 'Key stored';
		if (item.status === 'invalid') return 'Key unavailable';
		return 'Key missing';
	}
	function providerName(id: string) {
		return providerOptions.find(([key]) => key === id)?.[1] ?? id;
	}

	onMount(() => { void refresh(); });
</script>

<svelte:head><title>Account connections — {PRODUCT_NAME}</title></svelte:head>
<main class="account-connections">
	<header class="page-head">
		<div><a class="back-link" href="/">← Projects</a><h1 title="Provider API keys kept under your account. Grant access from a company's Intelligence page.">Connections</h1></div>
		<button class="btn primary" onclick={() => (addOpen = !addOpen)}>{addOpen ? 'Close' : 'Add connection'}</button>
	</header>
	{#if error}<div class="error" role="alert">{error}<button class="btn small" onclick={() => void refresh()}>Try again</button></div>{/if}
	{#if addOpen}
		<form class="add-form" onsubmit={create}>
			<h2>New provider connection</h2>
			<p>This connection is stored once in your account. Choose which companies can use it from each company’s Intelligence page.</p>
			<div class="form-grid"><label>Provider<select bind:value={provider}>{#each providerOptions as [id, name]}<option value={id}>{name}</option>{/each}</select></label><label>Name<input bind:value={label} required maxlength="80" placeholder="e.g. Anthropic team key" /></label><label class="full">API key<input bind:value={secret} type="password" required autocomplete="new-password" placeholder="Paste API key" /></label></div>
			<div class="actions"><button class="btn primary" disabled={busy}>{busy ? 'Saving…' : 'Save connection'}</button><button class="btn" type="button" disabled={busy} onclick={() => { addOpen = false; secret = ''; }}>Cancel</button></div>
			<p class="fine-print">Native Codex and Claude sign-ins stay with the company where they were set up.</p>
		</form>
	{/if}
	{#if loading}<p class="loading" role="status">Loading connections…</p>
	{:else if connections.length}
		<section class="connection-list" aria-label="Account connections">
			{#each connections as item (item.id)}
				<article class="connection">
					<div class="connection-head"><div><h2>{item.label}</h2><span>{providerName(item.provider)}</span></div><span class="status" class:connected={item.status === 'present'}>{statusText(item)}</span></div>
					{#if item.detail}<p class="connection-detail">{item.detail}</p>{/if}
					<div class="uses"><strong>Available to</strong>{#if item.companies.length}<div class="company-list">{#each item.companies as company (company.id)}<a href={`/${encodeURIComponent(company.id)}/company/provider`}>{company.name}<span aria-hidden="true">↗</span></a>{/each}</div>{:else}<span class="unused">No company access yet</span>{/if}</div>
				</article>
			{/each}
		</section>
	{:else}
		<div class="empty"><h2>No account connections yet</h2><p>Add an API key here, or bring an existing company connection into your account from that company’s Intelligence page.</p><button class="btn primary" onclick={() => (addOpen = true)}>Add your first connection</button></div>
	{/if}
</main>

<style>
	.account-connections { width: min(100%, 900px); min-height: 100%; margin: 0 auto; padding: var(--space-6); box-sizing: border-box; overflow: auto; }
	.page-head, .connection-head, .actions, .company-list, .uses { display: flex; align-items: center; }
	.page-head, .connection-head { justify-content: space-between; gap: var(--space-4); }
	.back-link { display: inline-block; margin-bottom: var(--space-3); color: var(--text-tertiary); font-size: var(--t-label); text-decoration: none; }
	h1 { margin: 0; font-size: var(--t-title); }
	.add-form p, .connection-head span, .connection-detail, .unused, .fine-print { color: var(--text-tertiary); font-size: var(--t-label); line-height: 1.5; }
	.connection-list { display: grid; gap: var(--space-3); margin-top: var(--space-5); }
	.connection, .add-form, .empty { padding: var(--space-4); background: var(--surface-pane); border: 1px solid var(--border); border-radius: var(--radius-pane); }
	.connection-head h2, .add-form h2, .empty h2 { margin: 0 0 3px; font-size: var(--t-head); }
	.connection-head > div { display: grid; gap: 2px; }
	.status { flex: none; padding: 4px 8px; border-radius: 99px; background: var(--surface-alt); }
	.status.connected { color: var(--state-success); }
	.uses { flex-wrap: wrap; gap: var(--space-3); padding-top: var(--space-3); margin-top: var(--space-3); border-top: 1px solid var(--border); font-size: var(--t-label); }
	.uses > strong { color: var(--text-secondary); font-weight: 500; }
	.company-list { flex-wrap: wrap; gap: var(--space-2); }
	.company-list a { display: inline-flex; gap: var(--space-1); padding: 5px 8px; border: 1px solid var(--border); border-radius: var(--radius-control); color: var(--text-primary); text-decoration: none; }
	.company-list a:hover { border-color: var(--intent-conversation); }
	.add-form { margin-top: var(--space-5); }
	.add-form p { margin: var(--space-2) 0 var(--space-4); }
	.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-3); }
	.form-grid label { display: grid; gap: var(--space-1); color: var(--text-secondary); font-size: var(--t-label); }
	.form-grid .full { grid-column: 1 / -1; }
	.form-grid input, .form-grid select { width: 100%; min-width: 0; min-height: 40px; padding: var(--space-2) var(--space-3); border: 1px solid var(--control-edge); border-radius: var(--radius-control); color: var(--ink); background: var(--surface-pane); font: inherit; }
	.actions { flex-wrap: wrap; gap: var(--space-2); margin-top: var(--space-4); }
	.fine-print { margin: var(--space-3) 0 0 !important; }
	.error { display: flex; align-items: center; flex-wrap: wrap; gap: var(--space-3); padding: var(--space-3); margin-top: var(--space-4); color: var(--state-danger); background: color-mix(in srgb, var(--state-danger) 7%, var(--surface-pane)); border-radius: var(--radius-control); }
	.loading, .empty { margin-top: var(--space-5); color: var(--text-secondary); }
	.empty p { max-width: 540px; color: var(--text-tertiary); line-height: 1.5; }
	@media (max-width: 620px) { .account-connections { padding: var(--space-4); } .page-head { align-items: flex-start; flex-direction: column; } .form-grid { grid-template-columns: 1fr; } .form-grid .full { grid-column: auto; } .connection-head { align-items: flex-start; flex-direction: column; } }
</style>
