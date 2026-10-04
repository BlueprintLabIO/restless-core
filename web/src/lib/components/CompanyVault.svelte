<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Notice, Empty, Dot, Row } from '$lib/ui/page';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import Link from '@lucide/svelte/icons/link';
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

<Page
	title="Vault"
	info="Company secrets, kept in the company's secure storage. You see their names, never their values."
>
	{#snippet actions()}
		<span
			title={error
				? 'Vault status unavailable'
				: 'Secrets are stored in the company’s secure vault.'}
		>
			<Dot
				show
				tone={error || view?.status === 'unavailable'
					? 'danger'
					: view?.status === 'connected'
						? 'success'
						: 'muted'}
				label={error
					? 'Unavailable'
					: view?.status === 'connected'
						? 'Connected'
						: view?.status === 'unavailable'
							? 'Unavailable'
							: busy
								? 'Checking…'
								: 'Not set up'}
			/>
		</span>
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
	{/snippet}

	{#if error}<Notice tone="danger" title="Could not read the vault" details={error}>
			{#snippet actions()}<button class="btn small" onclick={refresh} disabled={busy}>Retry</button
				>{/snippet}
		</Notice>{/if}
	{#if notice}<Notice tone="success" title={notice} />{/if}
	{#if view?.status === 'unavailable'}<Notice
			tone="danger"
			title="Secure storage is unavailable"
			details={view.message}
		/>{/if}

	{#if adding && view?.secrets}
		<Section title="Store a secret">
			<form id="vault-form" onsubmit={saveSecret}>
				<Row
					label={newSecret ? 'Name' : 'Connection'}
					info={newSecret
						? 'Letters, numbers, - and _. Agents refer to the secret by this name.'
						: 'Replace the secret an existing connection uses.'}
				>
					{#if newSecret}<input
							class="wide"
							bind:value={binding}
							aria-label="Secret name"
							placeholder="marketplace-key"
							pattern="[A-Za-z0-9_-]+"
							maxlength="64"
							required
							disabled={busy}
						/>
					{:else}<select
							class="wide"
							bind:value={binding}
							aria-label="Connection"
							required
							disabled={busy}
							><option value="" disabled>Choose a connection…</option>{#each writable as ref}<option
									value={ref.name}>{ref.name}</option
								>{/each}</select
						>{/if}
				</Row>
				<Row label="Value">
					<input
						class="wide"
						type="password"
						bind:value={secret}
						aria-label="Secret value"
						autocomplete="new-password"
						placeholder="Paste the value"
						required
						disabled={busy}
					/>
				</Row>
				<div class="form-bar">
					<button class="btn primary small" type="submit" disabled={busy}
						>{busy ? 'Saving…' : 'Save secret'}</button
					>
					{#if writable.length}<button
							class="btn small ghost"
							type="button"
							onclick={() => {
								newSecret = !newSecret;
								binding = '';
							}}
							>{newSecret
								? 'Replace an existing secret instead'
								: 'Add a new secret instead'}</button
						>{/if}
				</div>
			</form>
		</Section>
	{/if}

	{#if view?.secrets}
		<Section title="Secrets" count={view.secrets.length}>
			{#snippet actions()}
				{#if view && view.secrets && view.secrets.length > 6}<input
						class="search"
						type="search"
						bind:value={search}
						aria-label="Find a secret"
						placeholder="Find a secret"
					/>{/if}
			{/snippet}
			{#each rows as item (item.reference)}
				<Item title={item.name} meta={uses(item.reference)}>
					{#snippet leading()}<KeyRound size={15} strokeWidth={1.8} />{/snippet}
					{#snippet trailing()}<time title={item.updated_at ?? ''}
							>{formatRelative(item.updated_at, 'Stored')}</time
						>{/snippet}
				</Item>
			{:else}
				<Empty
					compact
					title={search ? 'No matching secrets' : 'No secrets stored yet'}
					info="Add a secret to keep it in this company's secure storage."
				/>
			{/each}
		</Section>
	{/if}

	{#if external.length}
		<Section
			title="Stored elsewhere"
			count={external.length}
			info="Credentials this company uses that live outside its vault, such as account connections."
		>
			{#each external as ref (ref.name)}
				<Item title={ref.name} meta={ref.reference}>
					{#snippet leading()}<Link size={15} strokeWidth={1.8} />{/snippet}
				</Item>
			{/each}
		</Section>
	{/if}
</Page>

<style>
	.wide {
		width: min(320px, 100%);
	}
	.search {
		width: 200px;
	}
	.form-bar {
		display: flex;
		gap: var(--space-2);
		padding: 12px 16px;
	}
</style>
