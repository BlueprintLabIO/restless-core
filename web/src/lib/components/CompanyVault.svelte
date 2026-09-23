<script lang="ts">
	import { onMount } from 'svelte';
	let { companyId }: { companyId: string } = $props();
	type Secret = { name: string; path: string; reference: string; updated_at: string | null };
	type Reference = { name: string; reference: string };
	type Vault = {
		status: string;
		secrets: Secret[] | null;
		references: Reference[];
		revision: string;
		message?: string;
	};
	let view = $state<Vault | null>(null),
		busy = $state(false),
		error = $state(''),
		search = $state(''),
		binding = $state(''),
		secret = $state(''),
		notice = $state('');
	const rows = $derived(
		view?.secrets?.filter((s) =>
			`${s.name} ${s.path}`.toLowerCase().includes(search.toLowerCase())
		) ?? []
	);
	const external = $derived(
		view?.references.filter((r) => !view?.secrets?.some((s) => s.reference === r.reference)) ?? []
	);
	const writable = $derived(
		(view?.references ?? []).filter(
			(r) =>
				!r.name.startsWith('model.inference') &&
				r.reference.startsWith(`infisical:/companies/${companyId}/`)
		)
	);
	function uses(reference: string) {
		return (
			view?.references
				.filter((r) => r.reference === reference)
				.map((r) => r.name)
				.join(', ') || 'No configured connection'
		);
	}
	async function refresh() {
		busy = true;
		error = '';
		try {
			const r = await fetch(`/api/companies/${companyId}/vault`, { cache: 'no-store', credentials: 'same-origin' });
			if (!r.ok) throw new Error('Could not read the company vault.');
			view = await r.json();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Could not read the vault.';
		} finally {
			busy = false;
		}
	}
	async function saveSecret(event: SubmitEvent) {
		event.preventDefault();
		if (!view || !binding || !secret) return;
		busy = true;
		error = '';
		notice = '';
		try {
			const r = await fetch(`/api/companies/${companyId}/vault/secret`, {
				method: 'POST',
				cache: 'no-store',
				credentials: 'same-origin',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ binding, secret, revision: view.revision })
			});
			const result = await r.json().catch(() => null);
			if (!r.ok) throw new Error(result?.message ?? 'Could not save the secret.');
			notice = `${binding} is stored in Infisical.`;
			binding = '';
			await refresh();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Could not save the secret.';
		} finally {
			secret = '';
			busy = false;
		}
	}
	onMount(() => {
		void refresh();
	});
</script>

<svelte:head><title>Vault — Company</title></svelte:head>
<div class="vault-page">
	<header>
		<h1>Vault</h1>
		<button class="btn small" onclick={refresh} disabled={busy}
			>{busy ? 'Checking…' : 'Refresh'}</button
		>
	</header>
	<p
		class="status"
		class:connected={!error && view?.status === 'connected'}
		class:unavailable={!!error || view?.status === 'unavailable'}
		role="status"
	>
		● {error
			? 'Vault status unavailable'
			: view?.status === 'connected'
				? 'Infisical connected'
				: view?.status === 'unavailable'
					? 'Infisical unavailable'
					: busy
						? 'Checking Infisical…'
						: 'Infisical is not configured'}
	</p>
	<p class="scope">Secrets stored for this company. Values remain hidden.</p>
	{#if error}<p role="alert">{error}</p>{/if}
	{#if notice}<p class="notice" role="status">{notice}</p>{/if}
	{#if view?.status === 'unavailable'}<p role="alert">{view.message}</p>{:else if view?.secrets}
		{#if writable.length}<section aria-label="Add or replace a secret">
			<h2>Add or replace a secret</h2>
			<form onsubmit={saveSecret}>
				<label for="vault-binding">Connection</label><select
					id="vault-binding"
					bind:value={binding}
					required
					disabled={busy}
				>
					<option value="" disabled>Choose a connection…</option>
					{#each writable as ref}<option value={ref.name}>{ref.name}</option>{/each}
				</select>
				<label for="vault-secret">API key</label><input
					id="vault-secret"
					type="password"
					bind:value={secret}
					autocomplete="new-password"
					placeholder="Paste API key"
					required
					disabled={busy}
				/>
				<button class="btn primary small" type="submit" disabled={busy}
					>{busy ? 'Saving…' : 'Save in Infisical'}</button
				>
			</form>
		</section>{/if}
		<label for="vault-search">Find a secret</label><input
			id="vault-search"
			type="search"
			bind:value={search}
			placeholder="Search names or folders"
		/>
		<section aria-label="Stored secrets">
			{#each rows as secret (secret.reference)}<article>
					<div>
						<strong>{secret.name}</strong><code>{secret.path}</code><small
							>{uses(secret.reference)}</small
						>
					</div>
					<span title={secret.updated_at ?? ''}
						>{secret.updated_at ? new Date(secret.updated_at).toLocaleDateString() : 'Stored'}</span
					>
				</article>{:else}<p>
					{search ? 'No matching secrets.' : 'No secrets stored for this company yet.'}
				</p>{/each}
		</section>{/if}
	{#if external.length}<details>
			<summary>Other credential locations ({external.length})</summary
			>{#each external as ref}<article>
					<div><strong>{ref.name}</strong><code>{ref.reference}</code></div>
				</article>{/each}
		</details>{/if}
	<a href={`/${companyId}/company/provider`}>Manage intelligence connections →</a>
</div>

<style>
	.vault-page {
		width: 100%;
		max-width: 880px;
		min-width: 0;
		min-height: 0;
		box-sizing: border-box;
		padding: var(--space-6);
		margin: 0 auto;
		overflow-y: auto;
		overflow-wrap: anywhere;
	}
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
	h1 {
		margin: 0;
		font-size: var(--t-title);
	}
	.status {
		color: var(--text-tertiary);
		margin-block: var(--space-5);
	}
	.connected {
		color: var(--state-success);
	}
	.unavailable,
	[role='alert'] {
		color: var(--state-danger);
	}
	.scope,
	small,
	code {
		color: var(--text-tertiary);
	}
	label {
		display: block;
		margin-top: var(--space-5);
		margin-bottom: var(--space-2);
	}
	input,
	select,
	button {
		box-sizing: border-box;
		font: inherit;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		padding: var(--space-2) var(--space-3);
		min-height: 40px;
		color: var(--ink);
		background: var(--surface-pane);
		max-width: 100%;
	}
	input {
		width: 100%;
		min-width: 0;
	}
	select {
		width: 100%;
	}
	form button {
		margin-top: var(--space-4);
	}
	.notice {
		color: var(--state-success);
	}
	button,
	summary {
		cursor: pointer;
	}
	button:disabled {
		opacity: 0.5;
	}
	article {
		display: flex;
		justify-content: space-between;
		gap: var(--space-3);
		padding-block: var(--space-4);
		border-bottom: 1px solid var(--control-edge);
	}
	article div {
		display: grid;
		min-width: 0;
		gap: var(--space-2);
	}
	article > span {
		flex: none;
		font-size: var(--t-label);
	}
	code,
	small {
		font-size: var(--t-label);
	}
	section,
	details {
		margin-bottom: var(--space-5);
	}
	@container company-canvas (max-width:640px) {
		.vault-page {
			padding: var(--space-4);
		}
		article {
			flex-direction: column;
		}
	}
</style>
