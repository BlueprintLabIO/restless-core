<script lang="ts">
	import { onMount } from 'svelte';
	let { companyId }: { companyId: string } = $props();
	type Secret = { name: string; path: string; reference: string; updated_at: string | null };
	type Reference = { name: string; reference: string };
	type Vault = {
		status: string;
		secrets: Secret[] | null;
		references: Reference[];
		message?: string;
	};
	let view = $state<Vault | null>(null),
		busy = $state(false),
		error = $state(''),
		search = $state('');
	const rows = $derived(
		view?.secrets?.filter((s) =>
			`${s.name} ${s.path}`.toLowerCase().includes(search.toLowerCase())
		) ?? []
	);
	const external = $derived(
		view?.references.filter((r) => !view?.secrets?.some((s) => s.reference === r.reference)) ?? []
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
			const r = await fetch(`/api/companies/${companyId}/vault`);
			if (!r.ok) throw new Error('Could not read the company vault.');
			view = await r.json();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Could not read the vault.';
		} finally {
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
		<button onclick={refresh} disabled={busy}>{busy ? 'Checking…' : 'Refresh'}</button>
	</header>
	<p
		class="status"
		class:connected={view?.status === 'connected'}
		class:unavailable={view?.status === 'unavailable'}
		role="status"
	>
		● {view?.status === 'connected'
			? 'Infisical connected'
			: view?.status === 'unavailable'
				? 'Infisical unavailable'
				: 'Checking Infisical…'}
	</p>
	<p class="scope">Secrets stored for this company. Values remain hidden.</p>
	{#if error}<p role="alert">{error}</p>{/if}
	{#if view?.status === 'unavailable'}<p role="alert">{view.message}</p>{:else if view?.secrets}
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
