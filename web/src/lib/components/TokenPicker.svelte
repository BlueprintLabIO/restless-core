<script lang="ts">
	/* The token an app uses, chosen where it is needed: paste it (kept in the company Vault under
	 * the app's own name) or pick a secret the Vault already holds. The token itself never leaves
	 * the owner's plane; the app keeps only which secret to use. */
	import { failureSentence } from '$lib/model/failure';
	import { readVault, saveVaultSecret, type VaultView } from '$lib/model/harness-auth';

	let {
		companyId,
		app,
		action,
		busy = false,
		onchoose
	}: {
		companyId: string;
		/** The app's name, which names a pasted token's secret (`github-token`). */
		app: string;
		/** The button, named for the act: "Add GitHub", "Replace token". */
		action: string;
		busy?: boolean;
		/** Called with the credential to use: a Vault binding name or secret reference. */
		onchoose: (credential: string) => Promise<unknown>;
	} = $props();

	let vault = $state<VaultView | null>(null);
	let readFailure = $state('');
	let mode = $state<'paste' | 'pick'>('paste');
	let token = $state('');
	let picked = $state('');
	let saving = $state(false);
	let failure = $state('');

	const binding = $derived(
		`${app
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '')}-token`.slice(0, 64)
	);
	const secrets = $derived(vault?.secrets ?? []);
	const ready = $derived(vault?.status === 'connected');

	$effect(() => {
		void companyId;
		readVault(companyId)
			.then((view) => {
				vault = view;
				readFailure = '';
			})
			.catch((cause) => (readFailure = failureSentence(cause, 'The Vault could not be read.')));
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (saving || busy || !vault) return;
		saving = true;
		failure = '';
		try {
			if (mode === 'paste') {
				await saveVaultSecret(companyId, {
					binding,
					secret: token,
					revision: vault.revision,
					create: true
				});
				await onchoose(binding);
			} else {
				await onchoose(picked);
			}
		} catch (cause) {
			failure = failureSentence(cause, 'The token was not saved.');
		} finally {
			token = '';
			saving = false;
		}
	}
</script>

<form class="token" onsubmit={submit}>
	{#if readFailure || (vault && !ready)}
		<p class="quiet" role="status">
			{readFailure ||
				vault?.message ||
				'The company Vault is not connected, so a token cannot be kept yet.'}
			<a href={`/${encodeURIComponent(companyId)}/company/vault`}>Vault</a>
		</p>
	{:else}
		{#if mode === 'paste'}
			<input
				type="password"
				aria-label="Token"
				placeholder="Paste the token"
				title={`Kept in the company Vault as “${binding}”. Agents never see it; Restless sends it to the app for them.`}
				autocomplete="new-password"
				required
				disabled={!vault || saving || busy}
				bind:value={token}
			/>
		{:else}
			<select aria-label="Vault secret" required disabled={saving || busy} bind:value={picked}>
				<option value="" disabled>Choose a Vault secret…</option>
				{#each secrets as secret (secret.reference)}<option value={secret.reference}
						>{secret.name}</option
					>{/each}
			</select>
		{/if}
		<button
			class="btn small primary"
			type="submit"
			disabled={!vault || saving || busy || (mode === 'paste' ? !token : !picked)}
			>{saving || busy ? 'Saving…' : action}</button
		>
		{#if secrets.length}
			<button
				class="link-button"
				type="button"
				onclick={() => (mode = mode === 'paste' ? 'pick' : 'paste')}
				>{mode === 'paste' ? 'Use a Vault secret' : 'Paste a token'}</button
			>
		{/if}
	{/if}
	{#if failure}<p class="error" role="alert">{failure}</p>{/if}
</form>

<style>
	.token {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		align-items: center;
	}
	.token input,
	.token select {
		width: min(280px, 100%);
	}
	.quiet {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
	.link-button {
		padding: 0;
		border: 0;
		background: none;
		color: var(--text-tertiary);
		font: inherit;
		font-size: var(--t-body);
		cursor: pointer;
	}
	.link-button:hover {
		color: var(--ink);
	}
	.error {
		flex-basis: 100%;
		margin: 0;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
