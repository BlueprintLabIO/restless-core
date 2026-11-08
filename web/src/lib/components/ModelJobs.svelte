<script lang="ts">
	/* Company › Intelligence › Models by job (S107): which model does which job. The main model
	 * thinks for the Exec, first planning, owner work, repairs and reviews; a cheaper everyday
	 * model takes Staff chat, routine first tasks and (owner, 4 Nov) team leads' routine turns,
	 * which a switch can keep on the main model; a small decision model spares the Exec wake-ups
	 * for routine results. Four fixed rows; each editor opens below its row. */
	import { Section, Item, Notice } from '$lib/ui/page';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import { toastWhile } from '$lib/ui/feedback/toast.svelte';
	import { formatRelative } from '$lib/ui/time';
	import Cpu from '@lucide/svelte/icons/cpu';
	import Feather from '@lucide/svelte/icons/feather';
	import Signpost from '@lucide/svelte/icons/signpost';
	import Users from '@lucide/svelte/icons/users';
	import Toggle from '$lib/ui/page/Toggle.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { modelLabel } from '$lib/model/intelligence-labels';
	import { pinsOwnModel } from '$lib/model/intelligence-route';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { modelJobsQuery } from '$lib/model/model-jobs.svelte';
	import {
		decisionDetail,
		decisionFix,
		decisionLine,
		everydayLine,
		leadsDetail,
		leadsLine
	} from '$lib/model/model-jobs';
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
	let savingLeads = $state(false);

	onMount(() =>
		watchIntelligenceChanges((changed) => {
			if (affectsCompany(changed, companyId)) void jobs.refresh();
		}, true)
	);

	const view = $derived(jobs.view);
	/* Agents the owner gave their own model on People: the everyday model never replaces theirs.
	 * The company's main model on another login of the same provider is not one (8 Nov). */
	const own = $derived(
		(intelligence.view?.agents ?? []).filter((agent) =>
			pinsOwnModel(agent.assignment, intelligence.view?.default)
		)
	);
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

	async function saveLeads(allowed: boolean) {
		savingLeads = true;
		await save(
			{ leads_use_everyday: allowed },
			allowed ? 'Leads use the everyday model for routine turns' : 'Leads stay on the main model'
		);
		savingLeads = false;
	}

	async function save(
		body: { everyday_model?: string; decision_model?: string; leads_use_everyday?: boolean },
		receipt: string
	) {
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
				? `${view.primary_model ? modelLabel(view.primary_model) : 'Not chosen yet'} · Exec, planning, owner work, repairs and reviews`
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
			meta={view ? `${everydayLine(view)} · Staff chat and routine tasks` : null}
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
		<!-- Owner, 4 Nov: leads' routine turns on the everyday model, on by default. With no everyday
		     model the switch has nothing to do, so it says so and the Change button above is the way. -->
		<Item title="Leads may use the everyday model" meta={view ? leadsLine(view) : null}>
			{#snippet leading()}<Users size={15} strokeWidth={1.8} />{/snippet}
			{#snippet trailing()}
				{#if !view}<span class="hold" aria-hidden="true"></span>{:else}
					<InfoTip text={leadsDetail(view)} />
					<Toggle
						checked={view.leads_use_everyday}
						label="Leads may use the everyday model"
						title={!view.everyday || view.everyday_model === 'off'
							? 'Choose an everyday model above first'
							: view.leads_use_everyday
								? 'Keep every lead turn on the main model'
								: 'Let leads use the everyday model for routine turns'}
						disabled={savingLeads || !view.everyday || view.everyday_model === 'off'}
						onchange={(next) => void saveLeads(next)}
					/>
				{/if}
			{/snippet}
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
					<!-- Decisions that cannot run offer the way to make them run (S110). Without an
					     OpenRouter key every other choice is closed here, so that is the one action. -->
					{#if decisionFix(view) === 'add_openrouter' && editing !== 'decisions'}<a
							class="btn small"
							href="/account/settings/accounts"
							title="Add an OpenRouter key in your account and give this company access, then choose a decision model here"
							>Add OpenRouter</a
						>{:else if editing !== 'decisions'}<button
							class="btn small"
							aria-label={decisionFix(view) === 'use_openrouter'
								? 'Use an OpenRouter model for decisions'
								: 'Change the decision model'}
							onclick={() => (editing = 'decisions')}
							>{decisionFix(view) === 'use_openrouter' ? 'Use OpenRouter' : 'Change'}</button
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
