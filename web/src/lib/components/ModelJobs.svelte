<script lang="ts">
	/* Company › Intelligence › Models by job (S107): which model does which job. The main model
	 * thinks for the Exec, team leads, repairs and reviews; a cheaper everyday model takes Staff
	 * chat and routine first tasks; a small decision model spares the Exec wake-ups for routine
	 * results. Three fixed rows; each editor opens below its row. */
	import { Section, Item, Notice } from '$lib/ui/page';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import { toastWhile } from '$lib/ui/feedback/toast.svelte';
	import { formatRelative } from '$lib/ui/time';
	import Cpu from '@lucide/svelte/icons/cpu';
	import Feather from '@lucide/svelte/icons/feather';
	import Signpost from '@lucide/svelte/icons/signpost';
	import { failureSentence } from '$lib/model/failure';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { modelJobsQuery } from '$lib/model/model-jobs.svelte';
	import { decisionDetail, decisionLine, everydayLine } from '$lib/model/model-jobs';
	import { watchIntelligenceChanges, affectsCompany } from '$lib/model/intelligence-events';
	import { onMount, untrack } from 'svelte';
	import EditorForm from './editors/EditorForm.svelte';
	import EverydayModelEditor from './editors/EverydayModelEditor.svelte';
	import DecisionModelEditor from './editors/DecisionModelEditor.svelte';

	let { companyId }: { companyId: string } = $props();
	/* The page keys this component by company, so its query is built once. */
	const jobs = untrack(() => modelJobsQuery(companyId));
	const intelligence = $derived(intelligenceQuery(companyId));
	const catalog = modelCatalog();
	let editing = $state<'' | 'everyday' | 'decisions'>('');
	let notice = $state(''),
		error = $state('');

	onMount(() =>
		watchIntelligenceChanges((changed) => {
			if (affectsCompany(changed, companyId)) void jobs.refresh();
		}, true)
	);

	const view = $derived(jobs.view);
	/* Agents the owner gave their own model on People: the everyday model never replaces theirs. */
	const own = $derived((intelligence.view?.agents ?? []).filter((agent) => !!agent.assignment));
	const ownLine = $derived(
		own.length
			? `${own.length} ${own.length === 1 ? 'agent keeps its' : 'agents keep their'} own model`
			: ''
	);
	/* The main model's provider serves the everyday model on the same connection. */
	const everydayModels = $derived.by(() => {
		const primary = view?.primary_model;
		if (!primary) return [];
		const provider = primary.split('/')[0];
		const route = intelligence.view?.default ?? intelligence.view?.base_default;
		const connection = intelligence.view?.connections.find((c) => c.id === route?.connection);
		return catalog.models(provider, connection?.account_kind ?? 'api_key').map((model) => ({
			...model,
			id: model.id.startsWith(`${provider}/`) ? model.id : `${provider}/${model.id}`
		}));
	});

	async function save(body: { everyday_model?: string; decision_model?: string }, receipt: string) {
		error = '';
		try {
			await jobs.save(body);
			editing = '';
			notice = receipt;
		} catch (cause) {
			error = failureSentence(cause, 'The model was not saved. Try again.');
		}
	}
	toastWhile(() => !!notice && { text: notice });
	toastWhile(() => !!error && { text: error, tone: 'failure' });
</script>

<Section
	id="jobs"
	title="Models by job"
	info="Every model call resends the company's whole context, so routine jobs go to cheaper models and anything hard stays on the main model. An agent you gave its own model in People keeps it for every job."
>
	{#if jobs.error && !view}
		<Notice tone="danger" title="Could not load the models by job">
			{#snippet actions()}<button class="btn small" onclick={() => jobs.refresh()}>Retry</button
				>{/snippet}
		</Notice>
	{:else}
		<Item
			title="Main"
			meta={view
				? `${view.primary_model ? modelLabel(view.primary_model) : 'Not chosen yet'} · Exec, team leads, repairs and reviews`
				: null}
		>
			{#snippet leading()}<Cpu size={15} strokeWidth={1.8} />{/snippet}
			{#snippet trailing()}
				{#if !view}<span class="hold" aria-hidden="true"></span>{:else if ownLine}<span
						class="quiet"
						title={own.map((agent) => (agent.id === 'exec' ? 'Exec' : agent.name)).join(', ')}
						>{ownLine}</span
					>{/if}
				<a class="btn small" href="#models" aria-label="Change the main model">Change</a>
			{/snippet}
		</Item>
		<Item
			title="Everyday"
			meta={view ? `${everydayLine(view)} · Staff chat and routine first tasks` : null}
			selected={editing === 'everyday'}
		>
			{#snippet leading()}<Feather size={15} strokeWidth={1.8} />{/snippet}
			{#snippet trailing()}
				{#if !view}<span class="hold" aria-hidden="true"
					></span>{:else if editing !== 'everyday'}<button
						class="btn small"
						disabled={!view.primary_model}
						aria-label="Change the everyday model"
						onclick={() => (editing = 'everyday')}>Change</button
					>{/if}
			{/snippet}
			{#if editing === 'everyday' && view}
				<EditorForm oncancel={() => (editing = '')}>
					<EverydayModelEditor
						jobs={view}
						models={everydayModels}
						onsave={(everyday_model, receipt) => save({ everyday_model }, receipt)}
					/>
				</EditorForm>
			{/if}
		</Item>
		<Item
			title="Decisions"
			meta={view ? decisionLine(view, (at) => formatRelative(at)) : null}
			selected={editing === 'decisions'}
		>
			{#snippet leading()}<Signpost size={15} strokeWidth={1.8} />{/snippet}
			{#snippet trailing()}
				{#if !view}<span class="hold" aria-hidden="true"></span>{:else}
					<InfoTip text={decisionDetail(view)} />
					{#if editing !== 'decisions'}<button
							class="btn small"
							aria-label="Change the decision model"
							onclick={() => (editing = 'decisions')}>Change</button
						>{/if}
				{/if}
			{/snippet}
			{#if editing === 'decisions' && view}
				<EditorForm oncancel={() => (editing = '')}>
					<DecisionModelEditor
						jobs={view}
						onsave={(decision_model, receipt) => save({ decision_model }, receipt)}
					/>
				</EditorForm>
			{/if}
		</Item>
	{/if}
</Section>

<style>
	/* The Change button's room while the models load, so nothing shifts when they arrive. */
	.hold {
		display: inline-block;
		width: 64px;
		height: 28px;
	}
	.quiet {
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: nowrap;
	}
</style>
