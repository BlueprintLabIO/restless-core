<script lang="ts">
	/* Connect a model where one is missing (S97-02), without leaving the page: use a connection the
	 * account already has, or paste an API key, which is saved to the account and given to this
	 * company. Sign-ins with a ChatGPT or Claude plan stay on Intelligence, which returns here by
	 * itself. Once connected, the card waits until Exec's route is live, then says so. */
	import Field from '$lib/ui/forms/Field.svelte';
	import FormFoot from '$lib/ui/forms/FormFoot.svelte';
	import { untrack } from 'svelte';
	import { failureSentence } from '$lib/model/failure';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { startingModel, usableAccountConnections } from '$lib/model/setup-model-offer';
	import { agentRouteState, intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	import { companiesQuery } from '$lib/model/queries.svelte';
	import {
		createConnection,
		readReusableConnections,
		useAccountConnection,
		type ReusableConnection
	} from '$lib/model/provider';
	import { PROVIDER_LABELS } from '$lib/model/provider-labels';

	let {
		companyId,
		returnTo = '',
		onready
	}: {
		companyId: string;
		/** Where Intelligence brings the owner back to, for the ways this card does not cover. */
		returnTo?: string;
		/** Exec can think: the surface carries on. */
		onready?: () => void;
	} = $props();

	const catalog = modelCatalog();
	const companies = companiesQuery();
	const intelligence = untrack(() => intelligenceQuery(companyId));
	let reusable = $state<ReusableConnection[]>([]);
	let loaded = $state(false);
	/* Keys belong to the account. Where the account lives on this computer a key can be pasted here;
	 * on Cloud the account page holds them. */
	let canPaste = $state(true);
	let manageUrl = $state('/account/settings/accounts');
	let busy = $state(false);
	let waiting = $state(false);
	let failure = $state('');
	let pasting = $state(false);
	let provider = $state('anthropic');
	let secret = $state('');

	$effect(() => {
		let live = true;
		void readReusableConnections()
			.then((view) => {
				if (!live) return;
				reusable = usableAccountConnections(view.connections, view.includedProviders);
				canPaste = view.scope === 'account';
				manageUrl = view.manageUrl;
				pasting = canPaste && !reusable.length;
				loaded = true;
			})
			.catch(() => {
				if (!live) return;
				pasting = true;
				loaded = true;
			});
		return () => {
			live = false;
		};
	});

	const PROVIDERS = Object.keys(PROVIDER_LABELS);
	function defaultModel(id: string, kind: 'api_key' | 'oauth' = 'api_key') {
		return startingModel(id, catalog.models(id, kind));
	}
	const label = (row: ReusableConnection) =>
		row.label || PROVIDER_LABELS[row.provider] || row.provider;

	/* The account connection becomes this company's default; a company key in the way is replaced,
	 * since a company that cannot start has nothing working to keep. */
	async function use(connectionId: string, providerId: string, kind: 'api_key' | 'oauth') {
		const model = defaultModel(providerId, kind);
		if (!model)
			throw new Error('No model is listed for this provider yet. Use Intelligence instead.');
		await useAccountConnection(companyId, connectionId, model, true);
		announceIntelligenceChange(companyId);
	}

	async function run(work: () => Promise<void>) {
		if (busy) return;
		busy = true;
		failure = '';
		try {
			await work();
			await settle();
		} catch (cause) {
			failure = failureSentence(cause, 'That model was not connected. Try again.');
		} finally {
			busy = false;
			waiting = false;
		}
	}

	/* A new connection loads on the company's computer for a few seconds; the card says so until
	 * Exec's own route is ready, then hands back. */
	async function settle() {
		waiting = true;
		const until = Date.now() + 90_000;
		while (Date.now() < until) {
			await Promise.all([intelligence.refresh(), companies.refresh()]).catch(() => {});
			const entry = companies.view.find((row) => row.id === companyId);
			if (!entry?.unstartable_reason && agentRouteState(intelligence.view, 'exec') === 'ready') {
				onready?.();
				return;
			}
			await new Promise((resolve) => setTimeout(resolve, 2000));
		}
		throw new Error('The connection is saved, but Exec is not ready yet. Give it a minute.');
	}

	function pasteKey(event: SubmitEvent) {
		event.preventDefault();
		const key = secret.trim();
		if (!key) {
			failure = 'Paste the API key.';
			return;
		}
		void run(async () => {
			const created = await createConnection({
				label: PROVIDER_LABELS[provider] ?? provider,
				provider,
				secret: key
			});
			secret = '';
			const id = created.id ?? '';
			if (!id)
				throw new Error('The key was saved, but could not be found. Use Intelligence instead.');
			await use(id, provider, 'api_key');
		});
	}
	const intelligenceHref = $derived(
		`/${encodeURIComponent(companyId)}/company/provider${returnTo ? `?return=${encodeURIComponent(returnTo)}` : ''}`
	);
</script>

<div class="connect-model" aria-busy={busy}>
	{#if waiting}
		<p class="connect-status" role="status">
			<i aria-hidden="true"></i><i aria-hidden="true"></i><i aria-hidden="true"></i><span
				>Connected. Exec is getting ready…</span
			>
		</p>
	{:else if !loaded}
		<p class="connect-status" role="status"><span>Looking for your connections…</span></p>
	{:else}
		{#if reusable.length && !pasting}
			<ul class="connect-list" aria-label="Connections in your account">
				{#each reusable as row (row.id)}
					<li>
						<span
							><strong>{label(row)}</strong><small
								>{row.kind === 'oauth' ? 'Signed in' : 'Key stored'} in your account</small
							></span
						>
						<button
							type="button"
							class="btn small primary"
							disabled={busy}
							onclick={() => void run(() => use(row.id, row.provider, row.kind ?? 'api_key'))}
							>Use it here</button
						>
					</li>
				{/each}
			</ul>
			{#if canPaste}<button type="button" class="connect-link" onclick={() => (pasting = true)}
					>Paste a new API key instead</button
				>{:else}<a class="connect-link" href={manageUrl}>Add a key in your account</a>{/if}
		{:else if !canPaste}
			<p class="connect-note">
				Your account has no model connection yet. <a class="connect-link" href={manageUrl}
					>Add a key in your account</a
				>, and it shows up here.
			</p>
		{:else}
			<form class="form flush" onsubmit={pasteKey}>
				<Field label="Provider" for="connect-provider">
					<select id="connect-provider" bind:value={provider} disabled={busy}>
						{#each PROVIDERS as id (id)}<option value={id}>{PROVIDER_LABELS[id]}</option>{/each}
					</select>
				</Field>
				<Field
					label="API key"
					hint="Saved to your account, so your other companies can use it too."
					for="connect-secret"
				>
					<input
						id="connect-secret"
						type="password"
						bind:value={secret}
						autocomplete="off"
						placeholder="Paste your API key"
						disabled={busy}
					/>
				</Field>
				<FormFoot submit="Connect" {busy} busyLabel="Connecting…">
					{#if reusable.length}<button
							type="button"
							class="btn small"
							onclick={() => (pasting = false)}>Use a saved connection</button
						>{/if}
				</FormFoot>
			</form>
		{/if}
		<a class="connect-link" href={intelligenceHref}>Sign in with ChatGPT or Claude instead</a>
	{/if}
	{#if failure}<p class="connect-failure" role="alert">{failure}</p>{/if}
</div>

<style>
	.connect-model {
		display: grid;
		gap: 10px;
		min-width: 0;
	}
	.connect-list {
		display: grid;
		gap: 6px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.connect-list li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 8px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
	}
	.connect-list span {
		display: grid;
		min-width: 0;
	}
	.connect-list strong {
		font-weight: 600;
	}
	.connect-list small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.connect-link {
		justify-self: start;
		padding: 0;
		border: 0;
		background: none;
		color: var(--intent-conversation);
		font: inherit;
		font-size: var(--t-label);
		font-weight: 500;
		text-decoration: none;
		cursor: pointer;
	}
	.connect-link:hover {
		text-decoration: underline;
	}
	.connect-status {
		display: flex;
		align-items: center;
		gap: 5px;
		min-height: 32px;
		margin: 0;
		color: var(--text-secondary);
	}
	.connect-status span {
		margin-left: 6px;
	}
	.connect-status i {
		width: 6px;
		height: 6px;
		border-radius: 999px;
		background: var(--text-tertiary);
		animation: breathe 1.2s ease-in-out infinite;
	}
	.connect-status i:nth-child(2) {
		animation-delay: 0.15s;
	}
	.connect-status i:nth-child(3) {
		animation-delay: 0.3s;
	}
	.connect-failure {
		margin: 0;
		color: var(--state-danger);
	}
	@keyframes breathe {
		50% {
			opacity: 0.3;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.connect-status i {
			animation: none;
		}
	}
</style>
