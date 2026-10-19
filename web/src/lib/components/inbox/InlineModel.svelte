<script lang="ts">
	/* The company's model, fixed in its card (S101) with Company › Intelligence's own parts: the
	 * account's headroom on top; below, the other accounts you hold, and for the one chosen the
	 * "Use here" editor (which model, then switch, or keep it as the fallback that takes over by
	 * itself), or Settings › Accounts' Add API key. Top up opens the plan where credit ran out. An
	 * agent on a refused model of its own gets the Intelligence page's model editor, starting from
	 * the company model. */
	import { onMount } from 'svelte';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { addFallback, loadModelChoices, type ModelOption } from '$lib/model/inline-model';
	import { popupFeatures } from '$lib/model/inline-credentials';
	import type { ProviderStatus } from '$lib/model/provider';
	import { MODEL_PRESETS } from '$lib/model/model-presets';
	import type { IntelligenceView } from '$lib/model/intelligence.svelte';
	import { ownerRequest } from '$lib/model/setting.svelte';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import Field from '$lib/ui/forms/Field.svelte';
	import ModelHeadroom from '../ModelHeadroom.svelte';
	import ModelAssignmentEditor from '../editors/ModelAssignmentEditor.svelte';
	import ModelAccountEditor from '../editors/ModelAccountEditor.svelte';
	import ApiKeyEditor from '../editors/ApiKeyEditor.svelte';
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
	const id = $props.id();
	let provider = $state<ProviderStatus | null>(null);
	let options = $state<ModelOption[]>([]);
	let loadFailed = $state(false);
	let choice = $state<string | null>(null);
	const KEY_PROVIDERS = MODEL_PRESETS.map((preset) => [preset.id, preset.name] as [string, string]);
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
						agent.assignment.connection !==
							(intelligence?.default ?? intelligence?.base_default)?.connection
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
		if (owner) void load();
	});

	/* The editors offer the switch, the fallback and the key; the card adds Top up. */
	cardActions(() => {
		if (!owner) return {};
		if (loadFailed) return { primary: { label: 'Try again', busyLabel: 'Reading…', run: load } };
		if (!provider || (stranded.length && intelligence) || !topUpHref) return {};
		const href = topUpHref;
		return {
			secondary: [
				{
					label: 'Top up credit',
					run: () => void window.open(href, 'restless-top-up', popupFeatures(560, 760))
				}
			]
		};
	});
</script>

<InlinePanel
	label="Fix the company's model"
	form={owner && !!provider}
	note={!owner
		? "The owner chooses the company's model."
		: loadFailed
			? "The company's model could not be read."
			: !provider
				? "Reading the company's model…"
				: ''}
>
	{#snippet status()}
		<ModelHeadroom {companyId} compact />
		{#if owner && provider && !stranded.length}
			<p class="current">
				Runs on <strong>{modelLabel(provider.primary_model)}</strong
				>{#if provider.model_failover?.length}, then {provider.model_failover
						.map((model) => modelLabel(model))
						.join(', then ')}{/if}.
			</p>
		{:else if owner && provider && intelligence}
			<p class="current">
				{stranded.map((agent) => agent.name).join(' and ')}
				{stranded.length > 1 ? 'run' : 'runs'} on <strong>{modelLabel(refusedModel)}</strong>,
				chosen for
				{stranded.length > 1 ? 'them' : 'it'} alone. The rest of the company runs on
				<strong>{modelLabel(provider.primary_model)}</strong>.
			</p>
		{/if}
	{/snippet}
	{#if owner && provider && stranded.length && intelligence}
		<ModelAssignmentEditor
			{companyId}
			view={intelligence}
			agentId={stranded[0].id}
			start={intelligence.default ?? intelligence.base_default ?? null}
			onsaved={(receipt) => ondone(receipt)}
		/>
	{:else if owner && provider}
		<Field label="Run the company on">
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
		</Field>
		{#if choice === ADD}
			<ApiKeyEditor
				providers={KEY_PROVIDERS}
				action="Add key"
				onadded={async () => {
					choice = null;
					await load();
				}}
			/>
		{:else if picked && provider}
			{@const option = picked}
			{@const current = provider}
			{#key option.connection.id}<ModelAccountEditor
					{companyId}
					connection={option.connection}
					revision={current.revision}
					makeDefault
					offerDefault={false}
					action={`Switch to ${option.connection.label}`}
					fallback={option.inUse
						? undefined
						: {
								label: 'Add as fallback',
								run: async (model) => {
									await addFallback(companyId, { ...option, model }, current);
									ondone(
										`${option.connection.label} is the fallback now: it takes over by itself whenever the main model can't answer.`
									);
								}
							}}
					onsaved={(_provider, model) =>
						ondone(
							`The company now runs on ${option.connection.label} (${modelLabel(model)}). Work carries on.`
						)}
				/>{/key}
		{/if}
	{/if}
</InlinePanel>

<style>
	.current {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.current strong {
		color: var(--ink);
		font-weight: 500;
	}
</style>
