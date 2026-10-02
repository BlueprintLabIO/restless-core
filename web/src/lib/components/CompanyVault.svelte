<script lang="ts">
	import SettingsHeader from '$lib/ui/views/SettingsHeader.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import EmptyState from '$lib/ui/views/EmptyState.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { formatRelative } from '$lib/ui/time';
	import { failureSentence } from '$lib/model/failure';
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
	let adding = $state(false),
		newSecret = $state(true);
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
			const r = await fetch(`/api/companies/${companyId}/vault`, {
				cache: 'no-store',
				credentials: 'same-origin'
			});
			if (!r.ok) throw new Error('Could not read the company vault.');
			view = await r.json();
		} catch (e) {
			error = failureSentence(e, 'Could not read the vault.');
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
				body: JSON.stringify({ binding, secret, revision: view.revision, create: newSecret })
			});
			const result = await r.json().catch(() => null);
			if (!r.ok) throw new Error(result?.message ?? 'Could not save the secret.');
			notice = 'Secret stored securely';
			adding = false;
			binding = '';
			await refresh();
		} catch (e) {
			error = failureSentence(e, 'Could not save the secret.');
		} finally {
			secret = '';
			busy = false;
		}
	}
	onMount(() => {
		void refresh();
	});
</script>

<CompanyTitle title="Vault" {companyId} />
<div class="company-page vault-page">
	<SettingsHeader title="Vault" explanation="Company secrets. Values stay hidden."
		>{#snippet actions()}
			<button
				class="btn primary small"
				disabled={busy || view?.status !== 'connected'}
				title={view?.status === 'connected'
					? 'Store a company secret'
					: 'Secure storage must be connected first'}
				onclick={() => {
					adding = !adding;
					newSecret = true;
					binding = '';
				}}>{adding ? 'Cancel' : 'Add secret'}</button
			>
			<ActionMenu label="Vault options"
				><button onclick={refresh} disabled={busy}>Refresh</button></ActionMenu
			>
		{/snippet}</SettingsHeader
	>
	<p
		class="status"
		title="Secrets are kept in the company’s secure vault. You see their names, never their values."
		class:connected={!error && view?.status === 'connected'}
		class:unavailable={!!error || view?.status === 'unavailable'}
		role="status"
	>
		{error
			? 'Vault status unavailable'
			: view?.status === 'connected'
				? 'Secure storage connected'
				: view?.status === 'unavailable'
					? 'Secure storage unavailable'
					: busy
						? 'Checking secure storage…'
						: 'Secure storage is not set up'}
	</p>

	{#if error}<p role="alert">{error}</p>{/if}
	{#if notice}<p class="notice" role="status">{notice}</p>{/if}
	{#if view?.status === 'unavailable'}<p role="alert">{view.message}</p>{:else if view?.secrets}
		{#if adding}<section aria-label="Add or replace a secret">
				<h2>Store a secret</h2>
				<form onsubmit={saveSecret}>
					<label for="vault-binding">{newSecret ? 'Secret name' : 'Connection'}</label>
					{#if newSecret}<input
							id="vault-binding"
							bind:value={binding}
							placeholder="e.g. marketplace-key"
							pattern="[A-Za-z0-9_-]+"
							maxlength="64"
							required
							disabled={busy}
						/>
					{:else}<select id="vault-binding" bind:value={binding} required disabled={busy}
							><option value="" disabled>Choose a connection…</option>{#each writable as ref}<option
									value={ref.name}>{ref.name}</option
								>{/each}</select
						>{/if}
					{#if writable.length}<button
							class="btn small"
							type="button"
							onclick={() => {
								newSecret = !newSecret;
								binding = '';
							}}>{newSecret ? 'Replace an existing secret' : 'Add a new secret'}</button
						>{/if}
					<label for="vault-secret">Secret value</label><input
						id="vault-secret"
						type="password"
						bind:value={secret}
						autocomplete="new-password"
						placeholder="Paste secret value"
						required
						disabled={busy}
					/>
					<button class="btn primary small" type="submit" disabled={busy}
						>{busy ? 'Saving…' : 'Save secret'}</button
					>
				</form>
			</section>{/if}
		{#if view.secrets.length > 6}<label for="vault-search">Find a secret</label><input
				id="vault-search"
				type="search"
				bind:value={search}
				placeholder="Search names or folders"
			/>{/if}
		<section aria-label="Stored secrets">
			{#each rows as secret (secret.reference)}<article>
					<div>
						<strong>{secret.name}</strong><small>{uses(secret.reference)}</small>
					</div>
					<span title={secret.updated_at ?? ''}>{formatRelative(secret.updated_at, 'Stored')}</span>
				</article>{:else}<EmptyState
					title={search ? 'No matching secrets' : 'No secrets stored yet'}
					explanation="Add a secret to keep it within this company's secure storage."
				/>{/each}
		</section>{/if}
	{#if external.length}<details>
			<summary>Other credential locations ({external.length})</summary
			>{#each external as ref}<article>
					<div><strong>{ref.name}</strong><code>{ref.reference}</code></div>
				</article>{/each}
		</details>{/if}
</div>

<style>
	.vault-page {
		overflow-wrap: anywhere;
	}
	.status {
		color: var(--text-tertiary);
		margin-block: 0 var(--space-3);
	}
	.connected {
		color: var(--state-success);
	}
	.unavailable,
	[role='alert'] {
		color: var(--state-danger);
	}
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
	.status::before {
		content: '';
		display: inline-block;
		width: 7px;
		height: 7px;
		margin-right: 7px;
		border-radius: 50%;
		background: var(--status-offline);
		vertical-align: 1px;
	}
	.status.connected::before {
		background: var(--state-success);
		box-shadow: 0 0 0 3px color-mix(in srgb, var(--state-success) 16%, transparent);
	}
	.status.unavailable::before {
		background: var(--state-danger);
	}
</style>
