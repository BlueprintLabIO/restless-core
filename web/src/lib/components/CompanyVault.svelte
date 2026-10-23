<script lang="ts">
	import Field from '$lib/ui/forms/Field.svelte';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Notice, Empty, Dot } from '$lib/ui/page';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import Link from '@lucide/svelte/icons/link';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { formatRelative } from '$lib/ui/time';
	import { failureSentence } from '$lib/model/failure';
	import { settingQuery } from '$lib/model/setting.svelte';
	import {
		deleteVaultSecret,
		readVault,
		referenceSource,
		saveVaultSecret,
		vaultKey,
		type VaultSecret
	} from '$lib/model/harness-auth';
	import { keep } from '$lib/search/search';
	let { companyId }: { companyId: string } = $props();
	const source = settingQuery({
		key: () => vaultKey(companyId),
		read: (signal) => readVault(companyId, signal)
	});
	const view = $derived(source.view);
	let adding = $state(false);
	/* The secret whose value is being replaced from its row. */
	let replacing = $state<VaultSecret | null>(null);
	let busy = $state(false),
		saveError = $state(''),
		search = $state(''),
		binding = $state(''),
		secret = $state(''),
		notice = $state('');
	const rows = $derived(keep(view?.secrets ?? [], search, (s) => [s.name, s.path]));
	const external = $derived(
		view?.references.filter((r) => !view?.secrets?.some((s) => s.reference === r.reference)) ?? []
	);
	function usedBy(reference: string) {
		return view?.references.filter((r) => r.reference === reference).map((r) => r.name) ?? [];
	}
	function uses(reference: string) {
		const names = usedBy(reference);
		return names.length ? `Used by ${names.join(', ')}` : 'Agents use it by name';
	}
	const error = $derived(
		saveError ||
			(source.failure ? failureSentence(source.failure, 'Could not read the vault.') : '')
	);
	const refresh = () => source.refresh();
	function openForm(target: VaultSecret | null) {
		replacing = target;
		adding = !target;
		binding = '';
		secret = '';
		saveError = '';
	}
	function closeForm() {
		adding = false;
		replacing = null;
		binding = '';
		secret = '';
	}
	async function saveSecret(event: SubmitEvent) {
		event.preventDefault();
		if (!view || !secret || (!replacing && !binding)) return;
		const name = binding.trim();
		/* A new name that is already taken would silently replace that secret's value. */
		const taken =
			!replacing &&
			(view.secrets?.some((s) => s.name === name) || view.references.some((r) => r.name === name));
		if (
			taken &&
			!(await confirmAction({
				title: `Replace the secret named ${name}?`,
				body: 'A secret with this name already exists. Saving gives it this new value, and everything that uses it picks the new one up.',
				action: 'Replace'
			}))
		)
			return;
		busy = true;
		saveError = '';
		notice = '';
		try {
			if (replacing)
				await saveVaultSecret(companyId, {
					reference: replacing.reference,
					secret,
					revision: view.revision
				});
			else
				await saveVaultSecret(companyId, {
					binding: name,
					secret,
					revision: view.revision,
					create: true,
					replace: taken
				});
			notice = replacing ? `${replacing.name} replaced` : 'Secret stored securely';
			closeForm();
			await refresh();
		} catch (e) {
			saveError = failureSentence(e, 'Could not save the secret.');
		} finally {
			secret = '';
			busy = false;
		}
	}
	async function remove(item: VaultSecret) {
		if (!view) return;
		const users = usedBy(item.reference);
		if (
			!(await confirmAction({
				title: `Delete ${item.name}?`,
				body: users.length
					? `${users.join(', ')} ${users.length === 1 ? 'uses' : 'use'} this secret and will stop working until given a new one. The value cannot be recovered.`
					: 'Agents that use this secret by name will no longer find it. The value cannot be recovered.',
				action: 'Delete',
				danger: true
			}))
		)
			return;
		busy = true;
		saveError = '';
		notice = '';
		try {
			await deleteVaultSecret(companyId, { reference: item.reference, revision: view.revision });
			if (replacing?.reference === item.reference) closeForm();
			notice = `${item.name} deleted`;
			await refresh();
		} catch (e) {
			saveError = failureSentence(e, 'Could not delete the secret.');
		} finally {
			busy = false;
		}
	}
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
							: source.loading
								? 'Checking…'
								: 'Not set up'}
			/>
		</span>
		<button
			class="btn primary small"
			disabled={busy || adding || view?.status !== 'connected'}
			title={view?.status === 'connected'
				? 'Store a company secret'
				: 'Secure storage must be connected first'}
			onclick={() => openForm(null)}>Add secret</button
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

	{#if (adding || replacing) && view?.secrets}
		<Section title={replacing ? `Replace ${replacing.name}` : 'Store a secret'}>
			<form class="form plain" id="vault-form" onsubmit={saveSecret}>
				{#if !replacing}<Field
						label="Name"
						hint="Letters, numbers, - and _. Agents refer to the secret by this name."
					>
						<input
							class="wide"
							bind:value={binding}
							aria-label="Secret name"
							placeholder="marketplace-key"
							pattern="[A-Za-z0-9_-]+"
							maxlength="64"
							required
							disabled={busy}
						/>
					</Field>{/if}
				<Field
					label={replacing ? 'New value' : 'Value'}
					info={replacing
						? 'The old value is replaced. Everything that uses this secret picks up the new one.'
						: undefined}
				>
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
				</Field>
				<footer class="form-foot">
					<button class="btn small" type="button" disabled={busy} onclick={closeForm}>Cancel</button
					><button class="btn primary small" type="submit" disabled={busy}
						>{busy ? 'Saving…' : replacing ? 'Replace' : 'Save secret'}</button
					>
				</footer>
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
					{#snippet actions()}<ActionMenu label={`${item.name} options`}
							><button
								disabled={busy || view?.status !== 'connected'}
								onclick={() => openForm(item)}>Replace value</button
							><button disabled={busy || view?.status !== 'connected'} onclick={() => remove(item)}
								>Delete</button
							></ActionMenu
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
				<Item title={ref.name} meta={referenceSource(ref.reference)}>
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
</style>
