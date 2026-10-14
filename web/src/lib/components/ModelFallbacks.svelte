<script lang="ts">
	/* The owner's ordered fallback models: when the main model is out of quota or unreachable, the
	 * company tries each in turn instead of stopping. Exact per-agent choices never fall back. */
	import { Section, Item, Notice, Empty } from '$lib/ui/page';
	import { failureSentence } from '$lib/model/failure';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { MODEL_PRESETS } from '$lib/model/model-presets';
	import { saveModelFailover, type ProviderStatus } from '$lib/model/provider';
	import { toast } from '$lib/ui/feedback/toast.svelte';

	let {
		companyId,
		status,
		onsaved
	}: { companyId: string; status: ProviderStatus; onsaved: (status: ProviderStatus) => void } =
		$props();

	const models = $derived(status.model_failover ?? []);
	/* Suggest models of providers this company can already reach; any full ID is accepted. */
	const suggestions = $derived(
		MODEL_PRESETS.filter((preset) =>
			status.connections.some(
				(row) => row.provider === preset.id && row.credential_status === 'present'
			)
		).flatMap((preset) => preset.models.map((model) => `${preset.id}/${model.id}`))
	);
	let adding = $state(false),
		draft = $state(''),
		saving = $state(false),
		failure = $state('');

	async function save(next: string[], message: string) {
		if (saving) return;
		saving = true;
		failure = '';
		try {
			onsaved(await saveModelFailover(companyId, next, status.revision));
			toast(message);
			return true;
		} catch (error) {
			failure = failureSentence(error, 'The fallback models were not saved.');
			return false;
		} finally {
			saving = false;
		}
	}
	async function add() {
		const model = draft.trim();
		if (!model) return;
		if (await save([...models, model], `${modelLabel(model)} added as a fallback`)) {
			draft = '';
			adding = false;
		}
	}
	function move(index: number, by: number) {
		const next = [...models];
		const [model] = next.splice(index, 1);
		next.splice(index + by, 0, model);
		void save(next, 'Fallback order saved');
	}
</script>

<Section
	title="Fallback models"
	info={`Tried in this order when ${status.primary_model ? modelLabel(status.primary_model) : 'the main model'} is out of quota or unreachable, so the company keeps working. Each needs a connection this company can use. An agent you gave its own model never falls back.`}
	count={models.length || null}
>
	{#snippet actions()}
		{#if !adding && models.length < 5}<button
				class="btn small"
				type="button"
				disabled={saving}
				onclick={() => {
					failure = '';
					adding = true;
				}}>Add fallback</button
			>{/if}
	{/snippet}
	{#each models as model, index (model)}
		<Item title={`${index + 1}. ${modelLabel(model)}`} meta={model}>
			{#snippet actions()}
				<button
					class="btn small ghost"
					type="button"
					disabled={saving || index === 0}
					aria-label={`Move ${modelLabel(model)} up`}
					onclick={() => move(index, -1)}>Up</button
				>
				<button
					class="btn small ghost"
					type="button"
					disabled={saving || index === models.length - 1}
					aria-label={`Move ${modelLabel(model)} down`}
					onclick={() => move(index, 1)}>Down</button
				>
				<button
					class="btn small ghost"
					type="button"
					disabled={saving}
					onclick={() =>
						void save(
							models.filter((item) => item !== model),
							`${modelLabel(model)} removed`
						)}>Remove</button
				>
			{/snippet}
		</Item>
	{:else}
		{#if !adding}<Empty
				compact
				title="No fallback: the company waits if its model is unavailable"
			/>{/if}
	{/each}
	{#if adding}
		<form
			class="add"
			onsubmit={(event) => {
				event.preventDefault();
				void add();
			}}
		>
			<label for="fallback-model" class="sr-only">Fallback model</label>
			<input
				id="fallback-model"
				type="text"
				list="fallback-model-options"
				required
				placeholder="provider/model"
				bind:value={draft}
				disabled={saving}
				title="A full provider/model ID, such as anthropic/claude-sonnet-4-6"
			/>
			<datalist id="fallback-model-options">
				{#each suggestions.filter((item) => !models.includes(item) && item !== status.primary_model) as item (item)}<option
						value={item}>{modelLabel(item)}</option
					>{/each}
			</datalist>
			<button class="btn primary small" disabled={saving || !draft.trim()}
				>{saving ? 'Saving…' : 'Add'}</button
			>
			<button
				class="btn small ghost"
				type="button"
				disabled={saving}
				onclick={() => {
					adding = false;
					draft = '';
				}}>Cancel</button
			>
		</form>
	{/if}
	{#if failure}<Notice tone="danger" title="Nothing changed">{failure}</Notice>{/if}
</Section>

<style>
	.add {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		align-items: center;
		padding: var(--space-2) 0;
	}
	.add input {
		flex: 1;
		min-width: 12rem;
	}
</style>
