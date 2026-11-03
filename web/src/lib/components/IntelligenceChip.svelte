<script lang="ts">
	/* Which model this agent thinks with, one click from where the owner talks
	 * to it. Choosing another model of the same connection applies to its next
	 * session, and the toast says so. Thinking effort is one company setting, chosen here too;
	 * everything else lives in Company › Intelligence. */
	import {
		EFFORTS,
		agentRouteState,
		intelligenceQuery,
		optimisticAssign,
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
	import { agentConnection, agentPickerSelection } from '$lib/model/intelligence-route';

	let {
		companyId,
		actorId,
		name,
		progressing = false
	}: {
		companyId: string;
		actorId: string;
		name: string;
		/** One of this agent's turns is running now: its model answers, whatever the routes say. */
		progressing?: boolean;
	} = $props();
	/* Choosing a model is the owner's alone, as the server decides; for anyone else the chip is not
	 * there, and reads nothing they would be refused. */
	const principal = $derived(companyPrincipalQuery(companyId));
	const allowed = $derived(hasOwnerSurfaceAccess(principal.view));
	const catalog = modelCatalog(() => allowed);
	const source = $derived(intelligenceQuery(companyId, () => allowed));
	const agent = $derived(source.view?.agents.find((row) => row.id === actorId) ?? null);
	const routeState = $derived(agentRouteState(source.view, actorId, progressing));
	const connection = $derived(
		source.view && agent ? (agentConnection(source.view, agent) ?? null) : null
	);
	const assignment = $derived(
		source.view && agent ? agentPickerSelection(source.view, agent) : null
	);
	const current = $derived(assignment?.model ?? '');
	/* An empty list from the connection is no list: fall back to the catalogue (see ModelAssignmentEditor). */
	const choices = $derived(
		connection?.models?.length
			? connection.models
			: catalog.models(
					connection?.account_provider ?? connection?.provider ?? '',
					connection?.account_kind ?? 'api_key'
				)
	);
	const assign = optimisticAssign();
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
			actor = actorId;
		const active = () => company === companyId && actor === actorId;
		if (!source.view || !assignment || busy || model === current) return;
		busy = true;
		failure = '';
		const chosen = choices.find((row) => row.id === model)?.name ?? modelLabel(model);
		const who = name;
		/* The chip names the choice at once and the menu closes; a refusal puts the old model back
		 * and says why (S109: no minutes between two states). */
		close();
		try {
			await assign(company, actor, {
				connection: assignment.connection,
				model,
				reset: false,
				revision: source.view.revision
			});
			toast(`${who} will use ${chosen} from their next session.`);
		} catch (cause) {
			toast(failureSentence(cause, `Could not move ${who} to ${chosen}.`), { tone: 'failure' });
		} finally {
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
	/* Where the composer is narrow (a phone), the family name gives way: "Sonnet 4.5". */
	const compactModel = $derived(shortModel.replace(/^Claude\s+/, ''));
	holdQueries(() => [source]);
</script>

<!-- A chosen model whose connection is still loading names that model: it is the choice, and it
     is ready within seconds. "Checking model…" and back was the jank of every switch (S109).
     Whatever it says, the chip keeps one width, so "Checking model…" becoming the model's name
     never moves the controls beside it (S109 C7). -->
{#if allowed}<div class="model-slot">
		{#if routeState !== 'ready' && !(routeState === 'starting' && agent)}
			<a
				class="chip-link"
				href={`/${encodeURIComponent(companyId)}/company/provider`}
				title={actorId === 'exec' && source.view?.company_issue
					? source.view.company_issue
					: 'Check this agent’s model connection in Intelligence'}
				>{routeState === 'checking' || routeState === 'starting'
					? 'Checking model…'
					: routeState === 'needs_connection'
						? 'Choose a model'
						: 'Model unavailable'}</a
			>
		{:else if agent}
			<div
				class="chip"
				title={`${name} thinks with ${modelLabel(agent.effective_model)}, ${effortLabel(agent.thinking_effort).toLowerCase()} effort`}
			>
				<Popover label={`${name}’s model`}>
					{#snippet trigger()}<span class="face"
							><Brain size={14} strokeWidth={1.8} aria-hidden="true" /><span class="model-name"
								><span class="full">{shortModel}</span><span class="compact" aria-hidden="true"
									>{compactModel}</span
								></span
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
						<a class="more" href={`/${encodeURIComponent(companyId)}/company/provider`}
							>More settings</a
						>
					{/snippet}
				</Popover>
			</div>
		{/if}
	</div>{/if}

<style>
	/* One width for every state: room for "Checking model…", "Model unavailable" and the longest
	 * short model name (18 characters) with its mark; a longer label ends in an ellipsis. Its width
	 * comes from the room beside it, never from what it says, so a state change moves nothing.
	 * Short of room it gives way first, down to its mark (the tooltip names the model), so the
	 * chip beside it keeps its whole label: on a phone the page chip was cut to "W." (S110). */
	.model-slot {
		display: flex;
		align-items: center;
		flex: 0 100 136px;
		min-width: 26px;
		overflow: hidden;
	}
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
	.model-name .compact {
		display: none;
	}
	/* The composer's control slot is a size container (chat.css): narrower than the full name and
	 * the page chip beside it, the short name shows. The slot's width never depends on what it
	 * holds, so this never flips back and forth. */
	@container composer-slot (max-width: 192px) {
		.model-name .full {
			display: none;
		}
		.model-name .compact {
			display: inline;
		}
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
	/* One line, never "Choose / a / model". */
	.chip-link {
		flex: 0 1 auto;
		min-width: 0;
		padding: 0 8px;
		overflow: hidden;
		text-overflow: ellipsis;
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
