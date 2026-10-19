<script lang="ts">
	/* The company's model, fixed in its card (S101): the account's headroom on top; below, the
	 * other accounts you hold, or a key to add. The footer switches to the one chosen, or adds it
	 * as the fallback that takes over by itself; Top up opens the plan where credit ran out. */
	import { onMount } from 'svelte';
	import { failureSentence } from '$lib/model/failure';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { freshness, meterTone, readCompanyHeadroom, resetsIn, timeOfDay, type CompanyHeadroom } from '$lib/model/account-usage';
	import { addFallback, loadModelChoices, switchModel, type ModelOption } from '$lib/model/inline-model';
	import { popupFeatures } from '$lib/model/inline-credentials';
	import type { ProviderStatus } from '$lib/model/provider';
	import { addApiKeyConnection } from '$lib/model/account-connections';
	import { MODEL_PRESETS } from '$lib/model/model-presets';
	import { assignModel, type IntelligenceView } from '$lib/model/intelligence.svelte';
	import { ownerRequest } from '$lib/model/setting.svelte';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import { cardActions, type CardAction } from './card-actions.svelte';

	let {
		companyId,
		owner,
		topUpHref,
		refusedModel = null,
		ondone
	}: {
		companyId: string;
		owner: boolean;
		/** Restless's own plan page, where credit is what ran out. */
		topUpHref?: string | null;
		/** The model that was refused, as provider/model; an agent may run on it alone. */
		refusedModel?: string | null;
		ondone: (receipt: string) => void;
	} = $props();

	const ADD = '__add-key';
	let provider = $state<ProviderStatus | null>(null);
	let options = $state<ModelOption[]>([]);
	let headroom = $state<CompanyHeadroom | null>(null);
	let loadFailed = $state(false);
	let error = $state('');
	let now = $state(Date.now());
	let choice = $state<string | null>(null);
	let keyProvider = $state('anthropic');
	let keySecret = $state('');
	const picked = $derived(options.find((option) => option.connection.id === choice) ?? null);
	/* Agents the owner put on the refused model themselves: the company default does not reach
	 * them, so the card puts them back on it (19 October: Snake Oil's Exec on GPT-6 Luna). */
	let intelligence = $state<IntelligenceView | null>(null);
	const stranded = $derived(
		refusedModel && intelligence
			? intelligence.agents.filter(
					(agent) =>
						agent.assignment &&
						agent.effective_model === refusedModel &&
						/* Only when the company model is on another account: a limit is account-wide, so
						 * moving to the same account's default would be refused too (Snake Oil, 19 Oct). */
						agent.assignment.connection !== (intelligence?.default ?? intelligence?.base_default)?.connection
				)
			: []
	);

	async function load() {
		loadFailed = false;
		try {
			if (refusedModel)
				intelligence = await ownerRequest<IntelligenceView>(
					`/api/companies/${encodeURIComponent(companyId)}/intelligence`
				).catch(() => null);
			const loaded = await loadModelChoices(companyId);
			provider = loaded.status;
			options = loaded.options;
			if (!options.some((option) => option.connection.id === choice))
				choice = options[0]?.connection.id ?? ADD;
		} catch {
			loadFailed = true;
		}
	}

	onMount(() => {
		void readCompanyHeadroom(companyId).then((value) => (headroom = value)).catch(() => {});
		if (owner) void load();
		const timer = setInterval(() => (now = Date.now()), 30_000);
		return () => clearInterval(timer);
	});

	async function attempt(work: () => Promise<string | null>, failure: string) {
		error = '';
		try {
			const receipt = await work();
			if (receipt) ondone(receipt);
		} catch (cause) {
			error = failureSentence(cause, failure);
		}
	}

	cardActions(() => {
		if (!owner) return {};
		if (loadFailed) return { primary: { label: 'Try again', busyLabel: 'Reading…', run: load } };
		if (!provider) return {};
		if (stranded.length && intelligence) {
			const view = intelligence;
			const company = provider;
			const names = stranded.map((agent) => agent.name).join(' and ');
			return {
				primary: {
					label: `Put ${names} on the company model`,
					busyLabel: 'Saving…',
					run: () =>
						attempt(async () => {
							for (const agent of stranded)
								await assignModel(companyId, agent.id, {
									connection: agent.assignment!.connection,
									model: agent.assignment!.model,
									reset: true,
									revision: view.revision
								});
							return `${names} now ${stranded.length > 1 ? 'run' : 'runs'} on the company model, ${modelLabel(company.primary_model)}.`;
						}, 'That did not save. Try again.')
				}
			};
		}
		const secondary: CardAction[] = [];
		if (topUpHref)
			secondary.push({
				label: 'Top up credit',
				run: () => void window.open(topUpHref, 'restless-top-up', popupFeatures(560, 760))
			});
		if (choice === ADD) {
			const name = MODEL_PRESETS.find((preset) => preset.id === keyProvider)?.name ?? keyProvider;
			return {
				secondary,
				primary: {
					label: `Add ${name} key`,
					busyLabel: 'Adding…',
					disabled: !keySecret.trim(),
					run: () =>
						attempt(async () => {
							await addApiKeyConnection({ label: name, provider: keyProvider, secret: keySecret.trim() });
							keySecret = '';
							choice = null;
							await load();
							return null;
						}, 'The key was not added. Check it and try again.')
				}
			};
		}
		if (!picked) return { secondary };
		const option = picked;
		const current = provider;
		if (!option.inUse)
			secondary.push({
				label: 'Add as fallback',
				busyLabel: 'Adding…',
				title: 'Takes over by itself whenever the main model cannot answer',
				run: () =>
					attempt(async () => {
						await addFallback(companyId, option, current);
						return `${option.connection.label} is the fallback now: it takes over by itself whenever the main model can't answer.`;
					}, 'That did not save. Try again.')
			});
		return {
			secondary,
			primary: {
				label: `Switch to ${option.connection.label}`,
				busyLabel: 'Switching…',
				run: () =>
					attempt(async () => {
						await switchModel(companyId, option, current.revision);
						return `The company now runs on ${option.connection.label} (${modelLabel(option.model)}). Work carries on.`;
					}, 'That did not save. Try again.')
			}
		};
	});
</script>

<InlinePanel
	label="Fix the company's model"
	{error}
	note={!owner
		? "The owner chooses the company's model."
		: loadFailed
			? "The company's model could not be read."
			: !provider
				? "Reading the company's model…"
				: ''}
>
	{#snippet status()}
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
		{#if owner && provider && !stranded.length}
			<p class="current">
				Runs on <strong>{modelLabel(provider.primary_model)}</strong>{#if provider.model_failover?.length}, then {provider.model_failover.map((model) => modelLabel(model)).join(', then ')}{/if}.
			</p>
		{/if}
	{/snippet}
	{#if owner && provider && stranded.length}
		<p class="current">
			{stranded.map((agent) => agent.name).join(' and ')}
			{stranded.length > 1 ? 'run' : 'runs'} on <strong>{modelLabel(refusedModel)}</strong>, chosen for
			{stranded.length > 1 ? 'them' : 'it'} alone. The rest of the company runs on
			<strong>{modelLabel(provider.primary_model)}</strong>.
		</p>
	{:else if owner && provider}
		<InlineChoice
			label="Run the company on"
			options={[
				...options.map((option) => ({
					value: option.connection.id,
					title: option.connection.label,
					detail: modelLabel(option.model),
					mark: option.inUse ? 'Fallback' : undefined
				})),
				{ value: ADD, title: 'Another account', detail: 'Add an API key' }
			]}
			bind:value={choice}
		/>
		{#if choice === ADD}
			<div class="key-row">
				<select bind:value={keyProvider} aria-label="Provider">
					{#each MODEL_PRESETS as preset (preset.id)}<option value={preset.id}>{preset.name}</option>{/each}
				</select>
				<input type="password" bind:value={keySecret} placeholder="API key" autocomplete="off" aria-label="API key" />
			</div>
		{/if}
	{/if}
</InlinePanel>

<style>
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
	.current {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.current strong {
		color: var(--ink);
		font-weight: 500;
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
	@media (max-width: 520px) {
		.key-row {
			flex-wrap: wrap;
		}
	}
</style>
