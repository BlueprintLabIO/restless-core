<script lang="ts">
	/* Which model this agent thinks with, one click from where the owner talks
	 * to it. Choosing another model of the same connection applies to its next
	 * session, and the toast says so. Thinking effort is one company setting, chosen here too;
	 * everything else lives in Company › Intelligence. */
	import {
		EFFORTS,
		assignModel,
		intelligenceQuery,
		setEffort
	} from '$lib/model/intelligence.svelte';
	import { effortLabel, modelLabel } from '$lib/model/intelligence-labels';
	import { modelCatalog } from '$lib/model/model-catalog.svelte';
	import { announceIntelligenceChange } from '$lib/model/intelligence-events';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import Popover from '$lib/ui/controls/Popover.svelte';
	import EffortSlider from './editors/EffortSlider.svelte';
	import { untrack } from 'svelte';
	import Brain from '@lucide/svelte/icons/brain';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import ModelPicker from '$lib/primitives/ModelPicker.svelte';

	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { hasOwnerSurfaceAccess } from '$lib/model/company-access';

	let { companyId, actorId, name }: { companyId: string; actorId: string; name: string } = $props();
	/* Choosing a model is the owner's alone, as the server decides; for anyone else the chip is not
	 * there, and reads nothing they would be refused. */
	const principal = $derived(companyPrincipalQuery(companyId));
	const allowed = $derived(hasOwnerSurfaceAccess(principal.view));
	const catalog = modelCatalog(() => allowed);
	const source = $derived(intelligenceQuery(companyId, () => allowed));
	const agent = $derived(source.view?.agents.find((row) => row.id === actorId) ?? null);
	const assignment = $derived(
		agent?.assignment ?? source.view?.default ?? source.view?.base_default ?? null
	);
	const connection = $derived(
		source.view?.connections.find((row) => row.id === assignment?.connection) ?? null
	);
	const choices = $derived(
		connection?.models ??
			catalog.models(
				connection?.account_provider ?? connection?.provider ?? '',
				connection?.account_kind ?? 'api_key'
			)
	);
	const current = $derived(assignment?.model ?? '');
	let busy = $state(false);
	let failure = $state('');

	$effect(() => {
		companyId;
		actorId;
		untrack(() => {
			busy = false;
			failure = '';
		});
	});

	async function choose(model: string, close: () => void) {
		const company = companyId,
			actor = actorId,
			originalSource = source;
		const active = () => company === companyId && actor === actorId;
		if (!source.view || !assignment || busy || model === current) return;
		busy = true;
		failure = '';
		try {
			await assignModel(company, actor, {
				connection: assignment.connection,
				model,
				reset: false,
				revision: source.view.revision
			});
			announceIntelligenceChange(company);
			if (!active()) return;
			const chosen = choices.find((row) => row.id === model)?.name ?? modelLabel(model);
			toast(`${name} will use ${chosen} from their next session.`);
			close();
		} catch (cause) {
			if (active()) failure = failureSentence(cause, 'Could not change the model.');
		} finally {
			await originalSource.refresh();
			if (active()) busy = false;
		}
	}
	async function chooseEffort(effort: string) {
		const company = companyId,
			actor = actorId,
			originalSource = source;
		const active = () => company === companyId && actor === actorId;
		if (!source.view || busy || effort === agent?.thinking_effort) return;
		busy = true;
		failure = '';
		try {
			await setEffort(company, { effort, revision: source.view.revision });
			announceIntelligenceChange(company);
			if (!active()) return;
			const label = EFFORTS.find((row) => row.value === effort)?.label ?? effort;
			toast(`Agents will think with ${label.toLowerCase()} effort from their next session.`);
		} catch (cause) {
			if (active()) failure = failureSentence(cause, 'Could not change the effort.');
		} finally {
			await originalSource.refresh();
			if (active()) busy = false;
		}
	}
	/* The chip names the model, short: the family without its provider or date. */
	const shortModel = $derived.by(() => {
		const label = agent ? modelLabel(agent.effective_model) : '';
		return label.length > 18 ? `${label.slice(0, 17)}…` : label;
	});
	holdQueries(() => [source]);
</script>

{#if !allowed}{:else if agent && modelLabel(agent.effective_model) === 'Unavailable'}
	<a class="chip-link" href={`/${encodeURIComponent(companyId)}/company/provider`}>Choose a model</a
	>
{:else if agent}
	<div
		class="chip"
		title={`${name} thinks with ${modelLabel(agent.effective_model)}, ${effortLabel(agent.thinking_effort).toLowerCase()} effort`}
	>
		<Popover label={`${name}’s model`}>
			{#snippet trigger()}<span class="face"
					><Brain size={14} strokeWidth={1.8} aria-hidden="true" /><span class="model-name"
						>{shortModel}</span
					></span
				>{/snippet}
			{#snippet children(close)}
				<!-- Only what choosing needs: effort, the models, a way to the rest. -->
				<div class="effort">
					<EffortSlider
						id={`efforts-${actorId}`}
						value={agent?.thinking_effort}
						disabled={busy}
						hint={false}
						onchange={(effort) => chooseEffort(effort)}
					/>
				</div>
				<ModelPicker
					models={choices}
					{current}
					disabled={busy || !assignment}
					label={`Model for ${name}`}
					onchoose={(model) => choose(model, close)}
				/>
				{#if failure}<p class="failure" role="alert">{failure}</p>{/if}
				<a class="more" href={`/${encodeURIComponent(companyId)}/company/provider`}>More settings</a
				>
			{/snippet}
		</Popover>
	</div>
{/if}

<style>
	.chip,
	.chip :global(.selection-popover) {
		flex: 0 1 auto;
		min-width: 0;
	}
	.chip :global(summary) {
		display: flex;
		align-items: center;
		min-width: 0;
		height: 24px;
		padding: 0 6px;
		border-radius: 999px;
		color: var(--text-tertiary);
	}
	.chip :global(summary:hover) {
		color: var(--ink);
	}
	.face {
		display: flex;
		align-items: center;
		gap: 4px;
		min-width: 0;
	}
	.model-name {
		min-width: 0;
		overflow: hidden;
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* A narrow menu: the longest model name sets it, not five effort boxes. */
	.chip :global(.selection-popover-panel) {
		width: 232px;
		padding: 4px;
		font: inherit;
	}
	.effort {
		display: grid;
		gap: 6px;
		margin-bottom: 4px;
		padding: 8px 8px 10px;
		border-bottom: 1px solid var(--border);
	}

	.more {
		display: block;
		margin-top: 4px;
		padding: 7px 8px 5px;
		border-top: 1px solid var(--border);
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.more:hover {
		color: var(--ink);
	}
	/* One line, never "Choose / a / model": the context chip beside it gives way instead. */
	.chip-link {
		flex: none;
		padding: 0 8px;
		white-space: nowrap;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.chip-link:hover {
		color: var(--ink);
	}
	.failure {
		margin: 4px 8px;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
