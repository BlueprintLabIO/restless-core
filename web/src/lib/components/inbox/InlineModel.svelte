<script lang="ts">
	/* The company's model, fixed inside its Inbox card (S101): how much headroom its account has,
	 * and the other accounts you hold, each one tap to switch to or to add as a fallback that takes
	 * over by itself. Top up opens the plan in a small window where credit is the issue. */
	import { onMount } from 'svelte';
	import { failureSentence } from '$lib/model/failure';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { freshness, meterTone, readCompanyHeadroom, resetsIn, timeOfDay, type CompanyHeadroom } from '$lib/model/account-usage';
	import { addFallback, loadModelChoices, switchModel, type ModelOption } from '$lib/model/inline-model';
	import { popupFeatures } from '$lib/model/inline-credentials';
	import type { ProviderStatus } from '$lib/model/provider';
	import { addApiKeyConnection } from '$lib/model/account-connections';
	import { MODEL_PRESETS } from '$lib/model/model-presets';

	let {
		companyId,
		owner,
		topUpHref,
		ondone
	}: {
		companyId: string;
		owner: boolean;
		/** Restless's own plan page, where credit is what ran out. */
		topUpHref?: string | null;
		ondone: (receipt: string) => void;
	} = $props();

	let status = $state<ProviderStatus | null>(null);
	let options = $state<ModelOption[]>([]);
	let headroom = $state<CompanyHeadroom | null>(null);
	let busy = $state('');
	let error = $state('');
	let now = $state(Date.now());
	let adding = $state(false);
	let keyProvider = $state('anthropic');
	let keySecret = $state('');

	async function reload() {
		const loaded = await loadModelChoices(companyId);
		status = loaded.status;
		options = loaded.options;
	}

	async function addKey(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !keySecret.trim()) return;
		busy = 'key';
		error = '';
		try {
			const name = MODEL_PRESETS.find((preset) => preset.id === keyProvider)?.name ?? keyProvider;
			await addApiKeyConnection({ label: name, provider: keyProvider, secret: keySecret.trim() });
			keySecret = '';
			adding = false;
			await reload();
		} catch (cause) {
			error = failureSentence(cause, 'The key was not added. Check it and try again.');
		} finally {
			busy = '';
		}
	}

	onMount(() => {
		void readCompanyHeadroom(companyId).then((value) => (headroom = value)).catch(() => {});
		if (owner)
			void loadModelChoices(companyId)
				.then((loaded) => {
					status = loaded.status;
					options = loaded.options;
				})
				.catch(() => {});
		const timer = setInterval(() => (now = Date.now()), 30_000);
		return () => clearInterval(timer);
	});

	async function act(key: string, work: () => Promise<string>) {
		if (busy || !status) return;
		busy = key;
		error = '';
		try {
			ondone(await work());
		} catch (cause) {
			error = failureSentence(cause, 'That did not save. Try again.');
		} finally {
			busy = '';
		}
	}

	const switchTo = (option: ModelOption) =>
		act(`switch:${option.connection.id}`, async () => {
			await switchModel(companyId, option, status!.revision);
			return `The company now runs on ${option.connection.label} (${modelLabel(option.model)}). Work carries on.`;
		});
	const fallBack = (option: ModelOption) =>
		act(`fallback:${option.connection.id}`, async () => {
			await addFallback(companyId, option, status!);
			return `${option.connection.label} is the fallback now: it takes over by itself whenever the main model can't answer.`;
		});
</script>

<section class="inline-model" aria-label="Fix the company's model">
	{#if headroom?.windows.length}
		<div class="windows">
			{#each headroom.windows as window (window.name)}
				{@const tone = meterTone(window.used_percent)}
				<div class="window">
					<div class="line">
						<span>{window.label}</span><span class="figure {tone}">{Math.round(window.used_percent)}% used</span>
					</div>
					<div class="meter {tone}" role="meter" aria-label={`${window.label} limit used`} aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(window.used_percent)}>
						<span style:width={`${Math.min(100, window.used_percent)}%`}></span>
					</div>
					<div class="facts">
						{#if window.resets_at}<span>{resetsIn(window.resets_at, now)} ({timeOfDay(window.resets_at, now)})</span>{/if}
						<span>{freshness(window.observed_at, now)}</span>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	{#if !owner}
		<p class="quiet">The owner chooses the company's model.</p>
	{:else if status}
		<p class="current">Runs on <strong>{modelLabel(status.primary_model)}</strong>{#if status.model_failover?.length}, then {status.model_failover.map((model) => modelLabel(model)).join(', then ')}{/if}.</p>
		{#if options.length}
			<div class="options" role="list" aria-label="Other accounts you hold">
				{#each options as option (option.connection.id)}
					<div class="option" role="listitem">
						<span class="who"><strong>{option.connection.label}</strong><small>{modelLabel(option.model)}</small></span>
						<button class="btn small" type="button" disabled={!!busy || option.inUse} onclick={() => void fallBack(option)}
							>{option.inUse ? 'Fallback' : busy === `fallback:${option.connection.id}` ? 'Adding…' : 'Add as fallback'}</button
						>
						<button class="btn small primary" type="button" disabled={!!busy} onclick={() => void switchTo(option)}
							>{busy === `switch:${option.connection.id}` ? 'Switching…' : 'Switch to it'}</button
						>
					</div>
				{/each}
			</div>
		{/if}
		{#if adding || !options.length}
			<form class="add-key" onsubmit={addKey}>
				<span class="quiet">{options.length ? 'Add another account' : 'You hold no other AI account yet. Add a key:'}</span>
				<div class="key-row">
					<select bind:value={keyProvider} aria-label="Provider">
						{#each MODEL_PRESETS as preset (preset.id)}<option value={preset.id}>{preset.name}</option>{/each}
					</select>
					<input type="password" bind:value={keySecret} placeholder="API key" autocomplete="off" aria-label="API key" />
					<button class="btn small primary" disabled={!!busy || !keySecret.trim()}>{busy === 'key' ? 'Adding…' : 'Add'}</button>
				</div>
			</form>
		{:else}
			<button class="link" type="button" onclick={() => (adding = true)}>Add another account</button>
		{/if}
		{#if topUpHref}
			<button class="btn small" type="button" onclick={() => window.open(topUpHref, 'restless-top-up', popupFeatures(560, 760))}
				>Top up credit</button
			>
		{/if}
	{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
	.inline-model {
		display: grid;
		gap: 12px;
		margin-top: 12px;
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
	}
	.windows {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 10px 18px;
	}
	.window {
		display: grid;
		gap: 4px;
	}
	.line {
		display: flex;
		justify-content: space-between;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.figure {
		font-variant-numeric: tabular-nums;
	}
	.figure.warn {
		color: var(--state-warning, var(--ink));
	}
	.figure.full {
		color: var(--state-danger);
	}
	.meter {
		height: 6px;
		border-radius: 99px;
		background: var(--surface-alt);
		overflow: hidden;
	}
	.meter span {
		display: block;
		height: 100%;
		background: var(--state-success);
	}
	.meter.warn span {
		background: var(--state-warning, var(--text-secondary));
	}
	.meter.full span {
		background: var(--state-danger);
	}
	.facts {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.current,
	.quiet,
	.error {
		margin: 0;
	}
	.current {
		color: var(--text-secondary);
	}
	.current strong {
		color: var(--ink);
		font-weight: 500;
	}
	.options {
		display: grid;
		gap: 6px;
	}
	.option {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		align-items: center;
		gap: 8px;
	}
	.who {
		display: grid;
		min-width: 0;
	}
	.who strong {
		font-weight: 500;
	}
	.who small,
	.quiet {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.error {
		color: var(--state-danger);
		font-size: var(--t-label);
	}
	.add-key {
		display: grid;
		gap: 6px;
	}
	.key-row {
		display: flex;
		gap: 6px;
		min-width: 0;
	}
	.key-row input {
		flex: 1;
		min-width: 0;
	}
	.link {
		justify-self: start;
		padding: 0;
		border: 0;
		background: none;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-decoration: underline;
		cursor: pointer;
	}
	@media (max-width: 520px) {
		.key-row {
			flex-wrap: wrap;
		}
		.option {
			grid-template-columns: 1fr 1fr;
		}
		.who {
			grid-column: 1 / -1;
		}
	}
</style>
