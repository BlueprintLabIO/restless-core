<script lang="ts">
	import { beforeNavigate } from '$app/navigation';
	import { getContext, onMount } from 'svelte';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	const models = modelCatalog();
	import { attentionQuery, companiesQuery, companyQuery } from '$lib/model/queries.svelte';

	type Settings = { display_name: string; mission: string; model: string; revision: string };
	let {
		companyId,
		section = 'provider',
		onsaved
	}: { companyId: string; section?: 'provider' | 'name'; onsaved?: () => void } = $props();
	const header = getContext<{ name: string | null }>('company-setup-draft');
	const attention = $derived(attentionQuery(companyId));
	const company = $derived(companyQuery(companyId));
	const catalog = companiesQuery();
	let name = $state('');
	let mission = $state('');
	let provider = $state('zai');
	let customProvider = $state('');
	let modelChoice = $state('glm-5.3');
	let customModel = $state('');
	let revision = '';
	let saved = '';
	let ready = $state(false);
	let saving = $state(false);
	let dirty = $state(false);
	let error = $state('');
	let timer: ReturnType<typeof setTimeout>;
	let inFlight: Promise<boolean> | null = null;
	const presets = $derived(models.providers.find((item) => item.id === provider));
	const model = $derived(
		`${provider === 'custom' ? customProvider.trim() : provider}/${modelChoice === 'custom' ? customModel.trim() : modelChoice}`
	);
	const draftKey = $derived(`restless-settings:${section}:${companyId}`);
	const endpoint = $derived(`/api/companies/${encodeURIComponent(companyId)}/setup`);

	function values() {
		return { display_name: name.trim(), mission, model };
	}
	function selectModel(value: string) {
		const slash = value.indexOf('/');
		const providerId = value.slice(0, slash);
		const modelId = value.slice(slash + 1);
		const preset = models.providers.find((item) => item.id === providerId);
		provider = preset ? preset.id : 'custom';
		customProvider = preset ? '' : providerId;
		modelChoice = preset?.models.some((item) => item.id === modelId) ? modelId : 'custom';
		customModel = modelChoice === 'custom' ? modelId : '';
	}
	function providerChanged() {
		modelChoice = models.providers.find((item) => item.id === provider)?.models[0].id ?? 'custom';
		customModel = '';
		changed();
	}
	async function readSettings(): Promise<Settings> {
		const response = await fetch(endpoint, { cache: 'no-store' });
		if (!response.ok) throw new Error('Could not load company settings. Reload to try again.');
		return response.json();
	}
	onMount(() => {
		let active = true;
		void readSettings()
			.then((settings) => {
				if (!active) return;
				revision = settings.revision;
				name = settings.display_name;
				mission = settings.mission;
				selectModel(settings.model);
				saved = JSON.stringify(values());
				const draft = sessionStorage.getItem(draftKey);
				if (draft) {
					try {
						const stored = JSON.parse(draft);
						name = stored.display_name;
						mission = stored.mission;
						selectModel(stored.model);
						dirty = JSON.stringify(values()) !== saved;
						if (dirty && stored.revision !== revision)
							error = 'Saved settings changed while this draft was open. Your draft is preserved.';
						if (dirty) revision = stored.revision;
					} catch {
						sessionStorage.removeItem(draftKey);
					}
				}
				ready = true;
				if (dirty && !error) changed();
			})
			.catch((cause) => {
				error = cause.message;
			});
		const warn = (event: BeforeUnloadEvent) => {
			if (dirty || saving) event.preventDefault();
		};
		window.addEventListener('beforeunload', warn);
		return () => {
			active = false;
			clearTimeout(timer);
			header.name = null;
			window.removeEventListener('beforeunload', warn);
		};
	});
	$effect(() => {
		if (ready && section === 'name') header.name = name || 'Untitled company';
	});

	async function reloadSaved() {
		try {
			const settings = await readSettings();
			name = settings.display_name;
			mission = settings.mission;
			selectModel(settings.model);
			revision = settings.revision;
			saved = JSON.stringify(values());
			dirty = false;
			error = '';
			ready = true;
			sessionStorage.removeItem(draftKey);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Could not reload settings.';
		}
	}

	function changed() {
		dirty = JSON.stringify(values()) !== saved;
		error = '';
		sessionStorage.setItem(draftKey, JSON.stringify({ ...values(), revision }));
		clearTimeout(timer);
		if (dirty) timer = setTimeout(() => void save(), 650);
	}
	function save(): Promise<boolean> {
		clearTimeout(timer);
		if (inFlight) return inFlight.then((ok) => (ok && dirty ? save() : ok));
		if (!dirty) return Promise.resolve(true);
		if (
			!name.trim() ||
			(provider === 'custom' && !customProvider.trim()) ||
			(modelChoice === 'custom' && !customModel.trim())
		)
			return Promise.resolve(false);
		const snapshot = JSON.stringify(values());
		saving = true;
		inFlight = (async () => {
			try {
				const response = await fetch(endpoint, {
					method: 'PUT',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify({ ...JSON.parse(snapshot), revision })
				});
				const body = await response.json();
				if (!response.ok)
					throw new Error(body.message ?? 'Could not save settings. Your edits are preserved.');
				error = '';
				revision = body.revision;
				saved = snapshot;
				dirty = JSON.stringify(values()) !== saved;
				if (dirty) sessionStorage.setItem(draftKey, JSON.stringify({ ...values(), revision }));
				else sessionStorage.removeItem(draftKey);
				void attention.refresh();
				void company.refresh();
				void catalog.refresh();
				onsaved?.();
				return true;
			} catch (cause) {
				error = cause instanceof Error ? cause.message : 'Could not save settings.';
				return false;
			} finally {
				saving = false;
				inFlight = null;
			}
		})();
		return inFlight;
	}
	beforeNavigate((navigation) => {
		// A failed or incomplete draft must never trap the owner on this page.
		// changed() already retains the draft in sessionStorage; attempt a final
		// save without cancelling navigation. Returning restores unsaved edits.
		if (!navigation.willUnload && (dirty || saving)) void save();
	});
</script>

<section class="setup-page">
	<header>
		<h2>{section === 'provider' ? 'Model' : 'Company name'}</h2>
		<span class:failed={!!error} role="status" aria-live="polite"
			>{error
				? 'Not saved'
				: saving
					? 'Saving…'
					: dirty
						? 'Unsaved changes'
						: ready
							? 'Saved'
							: 'Loading…'}</span
		>
	</header>
	{#if error}<div class="error" role="alert">
			{error}<button class="btn small" type="button" onclick={() => void save()}>Retry save</button
			><button class="btn small" type="button" onclick={() => void reloadSaved()}
				>Reload saved settings</button
			>
		</div>{/if}
	{#if ready}
		<div class="setup-fields">
			{#if section === 'name'}
				<label for="setup-name" class="sr-only">Company name</label>
				<input
					id="setup-name"
					bind:value={name}
					oninput={changed}
					maxlength="120"
					autocomplete="organization"
				/>
			{:else}
				<div class="model-fields">
					<div>
						<label for="setup-provider">Provider</label><select
							id="setup-provider"
							bind:value={provider}
							onchange={providerChanged}
							>{#each models.providers as preset}<option value={preset.id}>{preset.name}</option
								>{/each}<option value="custom">Custom provider…</option></select
						>
					</div>
					<div>
						<label for="setup-model">Model</label><select
							id="setup-model"
							bind:value={modelChoice}
							onchange={changed}
							>{#if modelChoice !== 'custom' && !presets?.models.some((m) => m.id === modelChoice)}<option
									value={modelChoice}>{modelChoice} (saved)</option
								>{/if}{#each presets?.models ?? [] as preset}<option value={preset.id}
									>{preset.name}</option
								>{/each}<option value="custom">Custom model…</option></select
						>
					</div>
				</div>
				{#if provider === 'custom'}<label for="custom-provider">Custom provider</label><input
						id="custom-provider"
						bind:value={customProvider}
						oninput={changed}
						placeholder="Provider ID"
						spellcheck="false"
					/>{/if}
				{#if modelChoice === 'custom'}<label for="custom-model">Custom model</label><input
						id="custom-model"
						bind:value={customModel}
						oninput={changed}
						placeholder="Model ID"
						spellcheck="false"
					/>{/if}
			{/if}
		</div>
	{/if}
</section>

<style>
	.setup-page {
		padding: 0;
		width: 100%;
		max-width: 820px;
		margin: 0 auto;
		overflow-y: auto;
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-bottom: var(--space-6);
	}
	h2 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
	}
	header span {
		font-size: var(--t-label);
		color: var(--text-tertiary);
		white-space: nowrap;
	}
	.setup-fields {
		display: grid;
		gap: 12px;
	}
	label {
		display: block;
		font-size: var(--t-body);
		font-weight: 500;
	}
	.setup-fields > label:not(:first-child) {
		margin-top: 12px;
	}
	input,
	select {
		box-sizing: border-box;
		width: 100%;
		min-width: 0;
		padding: 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
		font-size: var(--t-body);
	}
	input:focus-visible,
	select:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.model-fields {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 16px;
		margin-top: 12px;
	}
	.model-fields label {
		margin-bottom: 12px;
	}
	.error,
	.failed {
		color: var(--state-danger);
	}
	.error {
		padding: 12px;
		margin-bottom: 16px;
		font-size: var(--t-body);
	}
	.error button {
		margin-left: 12px;
	}
	@container company-canvas (max-width: 640px) {
		.setup-page {
			padding: 0;
		}
		.model-fields {
			grid-template-columns: 1fr;
		}
	}
</style>
